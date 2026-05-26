//! Formosaic — main application logic.
//!
//! # Play Modes
//!
//! * **Online** — Fetch a random model from poly.pizza, scramble it, and play.
//!   After solving, the model + credits are saved locally for replay.
//! * **Offline** — Browse and replay locally saved levels.
//!
//! # Puzzle Mechanics (Information Theory)
//!
//! The scramble axis is selected via an entropy search
//! (`puzzle::entropy::best_scramble_axis`): we sample many candidate axes and
//! pick the one whose viewpoint-entropy is lowest — meaning there is exactly
//! one "obvious" viewing direction and all others look clearly wrong.
//!
//! # Hints (3-tier progressive)
//!
//! Press **H** to cycle:
//!   Tier 1 → warm/cold compass (colour shows proximity to solution)
//!   Tier 2 → translucent axis-plane disc in 3D space
//!   Tier 3 → ghost snap (model slowly un-scrambles over ~5 s)
//!
//! # Controls
//!
//! | Key | Action                          |
//! |-----|---------------------------------|
//! | H   | Show next hint tier             |
//! | R   | Random offline level            |
//! | N   | Fetch random level from internet|
//! | K   | Cheat — instant solve           |
//! | Esc | Back to level select menu       |

// ─── Per-frame game data (passed to pipeline / renderers each frame) ──────────


use cgmath::{InnerSpace, Vector3};
use std::{cell::RefCell, rc::Rc, time::Instant, sync::mpsc::{channel, Receiver, Sender}, path::PathBuf};

use formosaic_engine::{
    architecture::{
        models::{
            model_loader::{IncrementalModelBuilder, ModelLoadData, ModelLoader},
            simple_model::{PuzzleParams, SimpleModel},
        },
        scene::{
            entity::simple_entity::SimpleEntity,
            node::node::NodeBehavior,
            scene_context::SceneContext,
        },
    },
    input::{Event, Key},
    rendering::{
        instances::camera::orbit_controller::OrbitController,
        render_state::LightConfig,
    },
};
use imgui;

use crate::{
    level::{
        poly_pizza::{ModelDownload, ModelSummary, PolyPizzaClient},
        storage::{LevelMeta, LevelRegistry},
    },
    puzzle::{
        entropy::{best_scramble_axis_from_offsets, difficulty_label, EntropyReport},
        hints::{HintOutput, HintSystem, HintTier},
        scrambler::{make_scrambled_orbit, ScrambleState},
    },
};

// Application trait lives in the engine — re-use it here.
pub use formosaic_engine::app::application::Application;

// ─── Shared UI state (written by game logic, read by UiNodes) ────────────────
//
// UiNodes live in the scenegraph and cannot hold a reference back to Formosaic
// (that would be a cycle).  Instead they capture an Rc<RefCell<UiState>> which
// Formosaic fills in each frame inside `populate_scene_context` — before the
// engine calls the UiNodes for that frame.

#[derive(Default, Clone)]
pub struct UiState {
    // ── In-game ──────────────────────────────────────────────────────────
    pub elapsed_secs:  f32,
    pub difficulty:    Option<f32>,
    pub hint_count:    u32,
    pub hint_tier:     HintTier,
    pub hint_warmth:   f32,
    pub is_solved:     bool,
    pub is_downloading: bool,
    pub is_loading:    bool,
    pub download_progress: Option<f32>,
    pub show_menu:     bool,
    pub is_touch:      bool,
    // ── Menu ─────────────────────────────────────────────────────────────
    pub levels:        Vec<LevelMeta>,
    pub current_level: Option<LevelMeta>,
    // ── Events fired by UI back to game logic ────────────────────────────
    /// Key events queued by UiNode callbacks; drained by Formosaic::on_update.
    pub queued_events: Vec<formosaic_engine::input::Event>,
    /// Specific level ID to play — set by the Play button, drained by on_update.
    pub play_specific: Option<String>,
    /// URL to open from credits.
    pub open_url: Option<String>,
}

// ─── Constants ────────────────────────────────────────────────────────────────

const TARGET_WORLD_RADIUS: f32 = 1.0;
const CAMERA_FOV:          f32 = 75.0 * std::f32::consts::PI / 180.0;
const SNAP_THRESHOLD_DOT:  f32 = 0.996;
const CAMERA_FADE_DOT:     f32 = 0.966;  // cos(15°) — un-scramble starts within this angle of solution
const RESTORE_DURATION:    f32 = 1.8;
/// Number of candidate axes tested in the entropy search.
const ENTROPY_CANDIDATES:  usize = 32;

// ─── State machine ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    Playing,
    Restoring {
        elapsed:   f32,
        cam_start: Vector3<f32>,
        cam_end:   Vector3<f32>,
    },
    Solved,
}

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    LevelSelect,
    InGame  { level_id: String },
    Downloading { summary: ModelSummary },
    Loading { level_id: String },
    Building { level_id: String },
}

/// Result sent from the background load thread to the main thread.
/// Includes the CPU-parsed model data PLUS the pre-computed entropy axis,
/// entropy report, and puzzle params — so the main thread never runs
/// `best_scramble_axis` or `compute_puzzle_params`.
struct LoadResult {
    request_id: u64,
    level_id: String,
    data: ModelLoadData,
    axis: Vector3<f32>,
    report: EntropyReport,
    params: PuzzleParams,
}

struct PendingLoad {
    request_id: u64,
    level_id: String,
    data: ModelLoadData,
    axis: Vector3<f32>,
    report: EntropyReport,
    params: PuzzleParams,
}

// ─── Main struct ──────────────────────────────────────────────────────────────

pub struct Formosaic {
    model:            Option<Rc<RefCell<SimpleModel>>>,
    entity:           Option<Rc<RefCell<SimpleEntity>>>,
    orbit:            Option<OrbitController>,
    scramble_state:   Option<ScrambleState>,
    entropy_report:   Option<EntropyReport>,
    game_state:       GameState,
    hints:            HintSystem,
    /// Cached output from the last hints.update() call — read by the renderer.
    last_hint_output: Option<HintOutput>,
    elapsed_secs:     f32,
    level_start:      Option<Instant>,
    registry:         LevelRegistry,
    client:           PolyPizzaClient,
    load_seq:         u64,
    loading_frames:   u32,
    loading_started:  Option<Instant>,
    load_tx:          Sender<LoadResult>,
    load_rx:          Receiver<LoadResult>,
    pending_load:            Option<PendingLoad>,
    incremental_builder:     Option<IncrementalModelBuilder>,
    /// Builder whose work is fully done — finalization deferred to next frame
    /// so it never compounds with the last mesh/texture upload.
    pending_finalize_builder: Option<IncrementalModelBuilder>,
    /// Pre-computed entropy axis/report/params from the background thread,
    /// stored here while the incremental builder runs.
    pending_axis:   Option<Vector3<f32>>,
    pending_report: Option<EntropyReport>,
    pending_params: Option<PuzzleParams>,
    mode:           AppMode,
    solved_timer:     f32,   // seconds since solve; transitions to menu after 5s
    /// Scene lighting config — passed to the pipeline each frame via GameFrameData.
    scene_lights:     LightConfig,
    /// Shared state read by UiNodes each frame.  Written in populate_scene_context.
    ui_state:         Rc<RefCell<UiState>>,
}

impl Formosaic {
    pub fn new() -> Self {
        let data_dir = LevelRegistry::default_data_dir();
        let registry = LevelRegistry::load(&data_dir);
        let (load_tx, load_rx) = channel();
        log::info!(
            "[Formosaic] Data dir: {}  ({} saved levels)",
            data_dir.display(),
            registry.levels.len()
        );
        Self {
            model: None, entity: None, orbit: None,
            scramble_state: None, entropy_report: None,
            game_state: GameState::Playing,
            hints: HintSystem::new(),
            last_hint_output: None,
            elapsed_secs: 0.0, level_start: None,
            registry,
            client: PolyPizzaClient::new(),
            load_seq: 0,
            loading_frames: 0,
            loading_started: None,
            load_tx,
            load_rx,
            pending_load: None,
            incremental_builder: None,
            pending_finalize_builder: None,
            pending_axis: None,
            pending_report: None,
            pending_params: None,
            mode: AppMode::LevelSelect,
            solved_timer: 0.0,
            scene_lights: LightConfig::default(),
            ui_state: Rc::new(RefCell::new(UiState::default())),
        }
    }

    // ── Level loading ──────────────────────────────────────────────────────

    fn load_model_from_bytes(&mut self, path: &str, level_id: &str, preloaded: Option<&[u8]>, ctx: &mut SceneContext) {
        // Clear scene then immediately re-register UiNodes — they live in the
        // scenegraph and scene.clear() would otherwise wipe them on every load.
        if let Some(scene) = ctx.scene() { scene.clear(); }
        self.register_ui_nodes(ctx);
        self.model  = None;
        self.entity = None;
        self.orbit  = None;
        self.hints.reset();
        self.loading_started = None;
        self.incremental_builder = None;
        self.pending_finalize_builder = None;
        self.pending_axis = None;
        self.pending_report = None;
        self.pending_params = None;

        let bytes = if let Some(b) = preloaded {
            b.to_vec()
        } else {
            let p = std::path::Path::new(path);
            if p.is_absolute() {
                std::fs::read(p)
                    .unwrap_or_else(|e| panic!("Failed to read model '{}': {e}", p.display()))
            } else {
                crate::asset_loader::load_3d_asset(path)
            }
        };
        let hint = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("obj");
        let data = ModelLoader::prepare_from_bytes(path, &bytes, hint);

        // Compute entropy and puzzle params from raw CPU data (no GPU needed).
        // This runs before building so the ~3.8 s search never compounds with
        // texture uploads on the main thread.
        let pos_slices: Vec<&[f32]> = data.meshes.iter().map(|m| m.positions.as_slice()).collect();
        let params = PuzzleParams::from_raw_positions(&pos_slices, &data.mesh_transforms, TARGET_WORLD_RADIUS, CAMERA_FOV);
        let flat_positions: Vec<f32> = data.meshes.iter().flat_map(|m| m.positions.iter().copied()).collect();
        let search = best_scramble_axis_from_offsets(&flat_positions, params.min_disp, params.max_disp, ENTROPY_CANDIDATES);

        let mut builder = IncrementalModelBuilder::new(data);
        while !builder.build_next() {}
        self.finalize_loaded_model(level_id.to_string(), builder, search.axis, search.report, params, ctx);
    }

    fn finalize_loaded_model(
        &mut self,
        level_id: String,
        builder: IncrementalModelBuilder,
        axis: Vector3<f32>,
        report: EntropyReport,
        params: PuzzleParams,
        ctx: &mut SceneContext,
    ) {
        let model = builder.finish();

        self.entropy_report = Some(report);
        log::info!(
            "[Formosaic] difficulty={} ({:.2})  entropy={:.2} bits  isolation={:.1}°",
            difficulty_label(report.difficulty),
            report.difficulty,
            report.entropy_bits,
            report.solution_isolation_rad.to_degrees(),
        );

        if let Some(m) = self.registry.levels.iter_mut().find(|m| m.id == level_id) {
            m.difficulty = report.difficulty;
        }

        model.borrow_mut().scramble_along_axis(axis, params.min_disp, params.max_disp);

        if let Some(scene) = ctx.scene() {
            let entity = Rc::new(RefCell::new(SimpleEntity::new(model.clone())));
            let s = params.entity_scale;
            entity.borrow_mut().transform_mut().set_scale(Vector3::new(s, s, s));
            entity.borrow_mut().transform_mut().add_rotation_euler_world(0.0, 180.0, 0.0);
            entity.borrow_mut().transform_mut().set_position(Vector3::new(0.0, 0.0, 0.0));

            let entity_rot = entity.borrow().transform().rotation;
            let solution_dir = (entity_rot * axis).normalize();
            self.scramble_state = Some(ScrambleState { solution_dir, params });
            scene.add_node(entity.clone());
            self.entity = Some(entity.clone());

            let camera = ctx.camera();
            let centroid = entity.borrow().centroid();
            let dist = params.orbit_distance;
            let (ctrl, sp) = make_scrambled_orbit(centroid, dist, solution_dir);
            camera.borrow_mut().transform.position = sp;
            camera.borrow_mut().transform.look_at(centroid, Vector3::unit_y());
            self.orbit = Some(OrbitController::new(centroid, dist));
            camera.borrow_mut().set_controller(Some(Box::new(ctrl)));
        }

        self.model = Some(model);
        self.game_state = GameState::Playing;
        self.level_start = Some(Instant::now());
        self.elapsed_secs = 0.0;
        self.mode = AppMode::InGame { level_id };
        self.solved_timer = 0.0;
        self.loading_started = None;
    }

    /// Ensure the builtin cactus level exists in the registry so it shows up
    /// in the menu grid on first run.  Safe to call every launch — it's a no-op
    /// if the level is already saved.
    fn seed_builtin_levels(&mut self) {
        let id = "cactus_builtin";
        if self.registry.levels.iter().any(|l| l.id == id) { return; }

        let meta = LevelMeta {
            id:             id.to_string(),
            name:           "Cactus".to_string(),
            author:         "SoyMaria".to_string(),
            license:        "CC-BY 3.0".to_string(),
            source_url:     "https://poly.pizza/m/7S5Snphkam".to_string(),
            model_file:     "cactus.fbx".to_string(),
            best_time_secs: None,
            play_count:     0,
            difficulty:     0.3,
        };

        let bytes = crate::asset_loader::load_3d_asset("models/Cactus/cactus.fbx");
        if let Err(e) = self.registry.save_level(meta, &bytes) {
            log::warn!("[Formosaic] Failed to seed builtin cactus: {e}");
        }
    }

    fn load_random_saved(&mut self, ctx: &mut SceneContext) {
        if let Some(meta) = self.registry.random_level() {
            let path = self.registry.model_path(meta);
            let id   = meta.id.clone();
            if path.exists() {
                self.begin_saved_level_load(id, path, ctx);
                return;
            }
        }
        // No saved levels — fetch one online instead
        log::info!("[Formosaic] No saved levels for random — fetching online");
        self.fetch_online_level();
    }

    fn fetch_online_level(&mut self) {
        if !self.client.is_explore_pending() {
            // Page 0 is fetched first to discover total, then a random valid
            // page is chosen — all handled inside fetch_explore.
            self.client.fetch_explore_page(0, 20);
            log::info!("[Formosaic] Fetching poly.pizza random page");
        }
    }

    fn begin_saved_level_load(&mut self, level_id: String, path: PathBuf, ctx: &mut SceneContext) {
        self.load_seq = self.load_seq.wrapping_add(1);
        self.incremental_builder = None;
        self.pending_finalize_builder = None;
        self.pending_axis = None;
        self.pending_report = None;
        self.pending_params = None;
        let request_id = self.load_seq;
        self.mode = AppMode::Loading { level_id: level_id.clone() };
        self.loading_frames = 0;
        self.loading_started = Some(Instant::now());
        self.model = None;
        self.entity = None;
        self.orbit = None;
        self.scramble_state = None;
        self.entropy_report = None;
        self.hints.reset();
        if let Some(scene) = ctx.scene() { scene.clear(); }
        self.register_ui_nodes(ctx);
        let tx = self.load_tx.clone();
        std::thread::spawn(move || {
            if let Ok(bytes) = std::fs::read(&path) {
                let data = ModelLoader::prepare_from_bytes_with_path(path.to_string_lossy().as_ref(), &bytes);
                let pos_slices: Vec<&[f32]> = data.meshes.iter().map(|m| m.positions.as_slice()).collect();
                let params = PuzzleParams::from_raw_positions(&pos_slices, &data.mesh_transforms, TARGET_WORLD_RADIUS, CAMERA_FOV);
                let flat_positions: Vec<f32> = data.meshes.iter().flat_map(|m| m.positions.iter().copied()).collect();
                let search = best_scramble_axis_from_offsets(&flat_positions, params.min_disp, params.max_disp, ENTROPY_CANDIDATES);
                let _ = tx.send(LoadResult {
                    request_id,
                    level_id,
                    data,
                    axis: search.axis,
                    report: search.report,
                    params,
                });
            }
        });
    }

    // ── Puzzle ─────────────────────────────────────────────────────────────

    fn trigger_solve(&mut self, ctx: &mut SceneContext) {
        if self.game_state != GameState::Playing { return; }
        let solution_dir = match &self.scramble_state { Some(s) => s.solution_dir, None => return };

        log::info!("[Formosaic] SOLVED! time={:.1}s hints={}", self.elapsed_secs, self.hints.hint_count());

        let camera = ctx.camera();
        let cam    = camera.borrow();
        let target = self.orbit.as_ref().map(|o| o.target).unwrap_or(cam.transform.position);
        let dist   = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
        let fwd    = cam.transform.forward().normalize();
        let dir    = if fwd.dot(solution_dir) > 0.0 { solution_dir } else { -solution_dir };
        let cam_end = target - dir * dist;
        let cam_start = cam.transform.position;
        drop(cam);
        camera.borrow_mut().set_controller(None);
        self.game_state = GameState::Restoring { elapsed: 0.0, cam_start, cam_end };

        // Snap the model to solved immediately — no un-scramble animation.
        if let Some(model) = &self.model {
            model.borrow().upload_lerp(0.0);
        }

        if let Some(start) = self.level_start {
            self.elapsed_secs = start.elapsed().as_secs_f32();
        }
    }

    fn finish_restore(&mut self, ctx: &mut SceneContext) {
        let camera = ctx.camera();
        let target = self.orbit.as_ref().map(|o| o.target)
            .unwrap_or_else(|| camera.borrow().transform.position);
        let dist = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
        let pos  = camera.borrow().transform.position;
        let mut post = OrbitController::new(target, dist);
        post.set_initial_position(pos);
        camera.borrow_mut().set_controller(Some(Box::new(post)));
        self.game_state = GameState::Solved;
        self.solved_timer = 0.0;
        self.hints.reset();

        let level_id = match &self.mode {
            AppMode::InGame { level_id } => level_id.clone(),
            _ => return,
        };
        self.registry.record_completion(&level_id, self.elapsed_secs);
    }

    // ── Download polling ────────────────────────────────────────────────────


    // ── Download polling ────────────────────────────────────────────────────

    fn poll_client(&mut self, ctx: &mut SceneContext) {
        // Poll explore results — handle Ok, empty Ok, and Err separately
        if let Some(result) = self.client.poll_explore() {
            match result {
                Ok(summaries) if !summaries.is_empty() => {
                    use rand::Rng;
                    let idx     = rand::rng().random_range(0..summaries.len());
                    let summary = summaries[idx].clone();
                    self.mode   = AppMode::Downloading { summary: summary.clone() };
                    self.client.download_model(&summary);
                    log::info!("[Formosaic] Downloading '{}'", summary.name);
                }
                Ok(_) => {
                    log::warn!("[Formosaic] Poly Pizza returned 0 models — retrying");
                    self.client.fetch_explore_page(0, 20);
                }
                Err(e) => {
                    log::warn!("[Formosaic] Poly Pizza fetch error: {} — retrying", e);
                    self.client.fetch_explore_page(0, 20);
                }
            }
        }

        // Poll download results
        if let Some(result) = self.client.poll_download() {
            match result {
                Ok(dl)  => self.on_download_complete(dl, ctx),
                Err(e)  => {
                    log::warn!("[Formosaic] Download failed: {} — retrying fetch", e);
                    // Re-fetch a new model rather than falling back to cactus
                    self.client.fetch_explore_page(0, 20);
                    self.mode = AppMode::LevelSelect;
                }
            }
        }
    }

    fn on_download_complete(&mut self, dl: ModelDownload, ctx: &mut SceneContext) {
        let meta = LevelMeta {
            id:             dl.id.clone(),
            name:           dl.name.clone(),
            author:         dl.author.clone(),
            license:        dl.license.clone(),
            source_url:     dl.source_url.clone(),
            model_file:     format!("model.{}", dl.file_ext),
            best_time_secs: None,
            play_count:     0,
            difficulty:     0.5, // updated after entropy analysis below
        };

        if let Err(e) = self.registry.save_level(meta.clone(), &dl.bytes) {
            log::warn!("[Formosaic] Failed to save level '{}': {}", dl.name, e);
        }

        let path = self.registry.model_path(&meta);
        // Pass the bytes we already have in memory — avoids re-reading from disk,
        // which would go through the JNI asset manager on Android and crash.
        self.begin_preloaded_level_load(meta.id.clone(), path.to_string_lossy().into_owned(), dl.bytes, ctx);

        // Persist the entropy-derived difficulty now that analysis has run.
        if let Some(report) = self.entropy_report {
            let data_dir  = LevelRegistry::default_data_dir();
            let meta_path = data_dir.join("levels").join(&dl.id).join("meta.json");
            if let Some(m) = self.registry.levels.iter_mut().find(|m| m.id == dl.id) {
                m.difficulty = report.difficulty;
                let _ = std::fs::write(meta_path, m.to_json());
            }
        }
    }

    fn begin_preloaded_level_load(&mut self, level_id: String, path: String, bytes: Vec<u8>, ctx: &mut SceneContext) {
        self.load_seq = self.load_seq.wrapping_add(1);
        self.incremental_builder = None;
        self.pending_finalize_builder = None;
        self.pending_axis = None;
        self.pending_report = None;
        self.pending_params = None;
        let request_id = self.load_seq;
        self.mode = AppMode::Loading { level_id: level_id.clone() };
        self.loading_frames = 0;
        self.loading_started = Some(Instant::now());
        self.model = None;
        self.entity = None;
        self.orbit = None;
        self.scramble_state = None;
        self.entropy_report = None;
        self.hints.reset();
        if let Some(scene) = ctx.scene() { scene.clear(); }
        self.register_ui_nodes(ctx);
        let tx = self.load_tx.clone();
        std::thread::spawn(move || {
            let data = ModelLoader::prepare_from_bytes_with_path(&path, &bytes);
            let pos_slices: Vec<&[f32]> = data.meshes.iter().map(|m| m.positions.as_slice()).collect();
            let params = PuzzleParams::from_raw_positions(&pos_slices, &data.mesh_transforms, TARGET_WORLD_RADIUS, CAMERA_FOV);
            let flat_positions: Vec<f32> = data.meshes.iter().flat_map(|m| m.positions.iter().copied()).collect();
            let search = best_scramble_axis_from_offsets(&flat_positions, params.min_disp, params.max_disp, ENTROPY_CANDIDATES);
            let _ = tx.send(LoadResult {
                request_id,
                level_id,
                data,
                axis: search.axis,
                report: search.report,
                params,
            });
        });
    }

    // ── Public accessors (read by game_engine / pipeline) ──────────────

    pub fn elapsed_secs(&self)           -> f32                    { self.elapsed_secs }
    pub fn solved_timer(&self)           -> f32                    { self.solved_timer }
    pub fn entropy_report(&self)         -> Option<&EntropyReport> { self.entropy_report.as_ref() }
    pub fn is_solved(&self)              -> bool                   { self.game_state == GameState::Solved }
    pub fn is_downloading(&self)         -> bool                   { matches!(self.mode, AppMode::Downloading { .. }) || self.client.is_download_pending() }
    pub fn saved_levels(&self)           -> &[LevelMeta]           { &self.registry.levels }
    pub fn hint_system(&self)            -> &HintSystem            { &self.hints }
    pub fn hint_system_mut(&mut self)    -> &mut HintSystem        { &mut self.hints }
    /// Cached hint output from this frame — never call hints.update() externally.
    pub fn last_hint_output(&self)       -> Option<&HintOutput>    { self.last_hint_output.as_ref() }

    /// True when the level-select / menu overlay should be shown.
    pub fn is_in_menu(&self) -> bool {
        matches!(self.mode, AppMode::LevelSelect | AppMode::Loading { .. } | AppMode::Building { .. })
    }

    /// Index of the currently highlighted level in the menu grid.
    pub fn selected_level_idx(&self) -> usize { 0 }

    pub fn solution_dir(&self) -> Option<Vector3<f32>> {
        self.scramble_state.as_ref().map(|s| s.solution_dir)
    }

    /// Current scene lighting configuration.
    pub fn lights(&self) -> LightConfig { self.scene_lights }

    /// Default lighting — used by game_engine for initial GL clear colour.
    pub fn default_lights() -> LightConfig { LightConfig::default() }

    // ── UiNode registration ────────────────────────────────────────────────
    //
    // Each panel extracted to game/src/ui/{panel}.rs — each creates its own
    // UiNode and registers it with the scenegraph.
    //
    // Panel registration order determines z-order:
    //   hud → hint_warmth → touch_buttons → menu → credits → loading

    fn register_ui_nodes(&self, ctx: &mut SceneContext) {
        let Some(scene) = ctx.scene() else { return };
        let state = Rc::clone(&self.ui_state);
        crate::ui::hud::register(scene, Rc::clone(&state));
        crate::ui::hint_warmth::register(scene, Rc::clone(&state));
        #[cfg(target_os = "android")]
        crate::ui::touch_buttons::register(scene, Rc::clone(&state));
        crate::ui::menu::register(scene, Rc::clone(&state));
        crate::ui::credits::register(scene, Rc::clone(&state));
        crate::ui::loading::register(scene, Rc::clone(&state));
    }
}
impl Default for Formosaic { fn default() -> Self { Self::new() } }

// ─── Application impl ─────────────────────────────────────────────────────────

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Application for Formosaic {
    fn on_init(&mut self, ctx: &mut SceneContext) {
        log::info!("[Formosaic] on_init");
        self.seed_builtin_levels();
        self.register_ui_nodes(ctx);

        // On resume after GL context loss, re-load the current level so
        // fresh GPU resources (VAOs, textures) are created for the new context.
        let resume_level_id = match &self.mode {
            AppMode::InGame { level_id }
            | AppMode::Loading { level_id }
            | AppMode::Building { level_id } => Some(level_id.clone()),
            _ => None,
        };
        if matches!(self.mode, AppMode::Downloading { .. }) {
            self.mode = AppMode::LevelSelect;
        }
        if let Some(id) = resume_level_id {
            if let Some(meta) = self.registry.levels.iter().find(|m| m.id == id).cloned() {
                let path = self.registry.model_path(&meta);
                if path.exists() {
                    self.begin_saved_level_load(meta.id.clone(), path, ctx);
                }
            }
        }
    }

    fn on_update(&mut self, delta_time: f32, ctx: &mut SceneContext) {
        // Drain play_specific first — a specific level ID from the menu Play button.
        let play_id = self.ui_state.borrow_mut().play_specific.take();
        if let Some(id) = play_id {
            if let Some(meta) = self.registry.levels.iter().find(|m| m.id == id).cloned() {
                let path = self.registry.model_path(&meta);
                if path.exists() {
                    self.begin_saved_level_load(meta.id.clone(), path, ctx);
                }
            }
        }

        // Drain any key events queued by UiNode callbacks.
        let queued: Vec<_> = self.ui_state.borrow_mut().queued_events.drain(..).collect();
        for ev in queued { self.on_event(&ev, ctx); }

        // Open browser URL from credits panel.
        if let Some(url) = self.ui_state.borrow_mut().open_url.take() {
            if let Err(e) = webbrowser::open(&url) {
                log::warn!("Failed to open URL {}: {e}", url);
            }
        }

        // Track elapsed frames in loading/building states.
        if matches!(self.mode, AppMode::Loading { .. } | AppMode::Building { .. }) {
            self.loading_frames = self.loading_frames.saturating_add(1);
        }

        // Receive CPU-parsed model data from the background thread.
        if let Ok(result) = self.load_rx.try_recv() {
            self.pending_load = Some(PendingLoad {
                request_id: result.request_id,
                level_id: result.level_id,
                data: result.data,
                axis: result.axis,
                report: result.report,
                params: result.params,
            });
        }

        // Loading → Building transition: data arrived from bg thread.
        if let Some(load) = self.pending_load.take() {
            if load.request_id != self.load_seq || self.loading_frames == 0 {
                self.pending_load = Some(load);
            } else {
                let level_id = load.level_id.clone();
                self.pending_axis = Some(load.axis);
                self.pending_report = Some(load.report);
                self.pending_params = Some(load.params);
                self.mode = AppMode::Building { level_id };
                self.incremental_builder = Some(IncrementalModelBuilder::new(load.data));
                self.loading_frames = 0;
            }
        }

        // Incremental GPU build: one unit of work per frame.
        if let Some(builder) = &mut self.incremental_builder {
            if builder.build_next() {
                // All meshes built and all textures uploaded — defer finalization
                // to the next frame so it never compounds with the last upload.
                self.pending_finalize_builder = self.incremental_builder.take();
            }
        }

        // Deferred finalization (separate frame from last build step).
        if let Some(builder) = self.pending_finalize_builder.take() {
            let level_id = match &self.mode {
                AppMode::Building { level_id } => level_id.clone(),
                _ => unreachable!(),
            };
            let axis = self.pending_axis.take().unwrap();
            let report = self.pending_report.take().unwrap();
            let params = self.pending_params.take().unwrap();
            self.finalize_loaded_model(level_id, builder, axis, report, params, ctx);
            self.loading_frames = 0;
        }

        self.poll_client(ctx);

        // Update hints once per frame and cache for the renderer.
        self.last_hint_output = if let Some(sc) = &self.scramble_state {
            let camera = ctx.camera();
            let fwd    = camera.borrow().transform.forward().normalize();
            let output = self.hints.update(delta_time, fwd, sc.solution_dir);
            // Apply ghost-snap lerp (Tier 3 hint).
            if output.ghost_lerp > 0.0 {
                if let Some(model) = &self.model {
                    model.borrow().upload_lerp(1.0 - output.ghost_lerp);
                }
            }
            Some(output)
        } else {
            None
        };

        let mut do_solve           = false;
        let mut do_restore_complete = false;
        let mut cam_pos:            Option<Vector3<f32>> = None;
        let mut cam_target:         Option<Vector3<f32>> = None;

        match &mut self.game_state {
            GameState::Playing => {
                self.elapsed_secs += delta_time;
                let snap = if let Some(sc) = &self.scramble_state {
                    let camera = ctx.camera();
                    let fwd = camera.borrow().transform.forward().normalize();
                    let dot = fwd.dot(sc.solution_dir.normalize()).abs();

                    // Camera-driven un-scramble: when looking from near the
                    // solution direction, smoothly reduce the scramble so
                    // triangle seams disappear — the model appears correct
                    // from the solution angle without needing ghost snap.
                    // Skip during GhostSnap hint (ghost_lerp handles it).
                    if self.hints.tier() != HintTier::GhostSnap {
                        if let Some(model) = &self.model {
                            model.borrow().upload_lerp(camera_scramble_t(dot));
                        }
                    }

                    dot >= SNAP_THRESHOLD_DOT
                } else { false };
                if snap { do_solve = true; }
            }
            GameState::Restoring { elapsed, cam_start, cam_end } => {
                *elapsed += delta_time;
                let ev = *elapsed;
                if ev >= RESTORE_DURATION { do_restore_complete = true; }
                let t = smoothstep((ev / RESTORE_DURATION).min(1.0));
                cam_pos    = Some(*cam_start + (*cam_end - *cam_start) * t);
                cam_target = self.orbit.as_ref().map(|o| o.target);
            }
            GameState::Solved => {
                self.solved_timer += delta_time;
            }
        }

        if let (Some(pos), Some(tgt)) = (cam_pos, cam_target) {
            let camera = ctx.camera();
            let mut c = camera.borrow_mut();
            c.transform.position = pos;
            c.transform.look_at(tgt, Vector3::unit_y());
        }

        if do_solve            { self.trigger_solve(ctx); }
        if do_restore_complete { self.finish_restore(ctx); }
    }

    fn on_event(&mut self, event: &Event, ctx: &mut SceneContext) {
        match event {
            Event::KeyDown { key: Key::H } => {
                self.hints.advance();
                log::info!("[Formosaic] Hint → {:?}", self.hints.tier());
            }
            Event::KeyDown { key: Key::K } => {
                log::info!("[Formosaic] Cheat-solve");
                self.trigger_solve(ctx);
            }
            Event::KeyDown { key: Key::R } => {
                log::info!("[Formosaic] Random saved level");
                self.load_random_saved(ctx);
            }
            Event::KeyDown { key: Key::N } => {
                log::info!("[Formosaic] Online random level");
                self.fetch_online_level();
            }
            Event::KeyDown { key: Key::Escape } => {
                self.mode = AppMode::LevelSelect;
                self.incremental_builder = None;
                self.pending_finalize_builder = None;
                self.pending_axis = None;
                self.pending_report = None;
                self.pending_params = None;
            }
            _ => {
                if matches!(self.game_state, GameState::Playing | GameState::Solved) {
                    let camera = ctx.camera();
                    let (w, h) = {
                        let c = camera.borrow();
                        (c.resolution.x as f32, c.resolution.y as f32)
                    };
                    camera.borrow_mut().controller.handle_event(event, w, h);
                }
            }
        }
    }

    fn populate_scene_context(&mut self, ctx: &mut SceneContext, delta_time: f32) {
        use formosaic_engine::rendering::render_state::HintRenderState;
        let solved   = self.game_state == GameState::Solved && !self.is_in_menu();
        let solved_t = self.solved_timer;
        ctx.lights       = self.scene_lights;
        ctx.show_menu    = self.is_in_menu();
        ctx.is_touch     = cfg!(target_os = "android");
        ctx.delta_time   = delta_time;
        ctx.solved_timer = if solved { Some(solved_t) } else { None };
        ctx.hints        = self.last_hint_output.as_ref().map(|o| HintRenderState {
            warmth:       o.warmth,
            warmth_color: o.warmth_color,
            tier:         o.tier.as_u8(),
            time:         solved_t,
        });

        // Sync game state into UiState so UiNodes can read it this frame.
        {
            let mut ui = self.ui_state.borrow_mut();
            ui.elapsed_secs   = self.elapsed_secs;
            ui.difficulty     = self.entropy_report.map(|r| r.difficulty);
            ui.hint_count     = self.hints.hint_count() as u32;
            ui.hint_tier      = self.hints.tier();
            ui.hint_warmth    = self.last_hint_output.as_ref().map(|o| o.warmth).unwrap_or(0.5);
            ui.is_solved      = self.game_state == GameState::Solved;
            ui.is_downloading = matches!(self.mode, AppMode::Downloading { .. }) || self.client.is_download_pending();
            ui.is_loading     = matches!(self.mode, AppMode::Loading { .. } | AppMode::Building { .. });
            ui.download_progress = if matches!(self.mode, AppMode::Loading { .. }) {
                // Phase 1: background thread parsing — capped at 30%.
                let t = self.loading_started.map(|start| start.elapsed().as_secs_f32()).unwrap_or(0.0);
                Some((t / 2.0).clamp(0.05, 0.30))
            } else if self.pending_finalize_builder.is_some() {
                // Deferred finalization frame — all GPU work done, bar at 99%.
                Some(0.99)
            } else if let AppMode::Building { .. } = &self.mode {
                // Phase 2: incremental GPU upload — real progress, 30% → 95%.
                let p = self.incremental_builder.as_ref().map(|b| b.progress()).unwrap_or(0.0);
                Some(0.30 + p * 0.65)
            } else if ui.is_downloading {
                Some((self.client.download_progress_bytes() as f32 / 1_000_000.0).clamp(0.0, 0.95))
            } else {
                None
            };
            ui.show_menu      = self.is_in_menu();
            ui.is_touch       = cfg!(target_os = "android");
            ui.levels         = self.registry.levels.clone();
            ui.current_level  = match &self.mode {
                AppMode::InGame { level_id } | AppMode::Loading { level_id } | AppMode::Building { level_id } => {
                    self.registry.levels.iter().find(|l| &l.id == level_id).cloned()
                }
                _ => None,
            };
            // Don't clear queued_events here — on_update drains them.
        }
    }

    fn title(&self) -> &str { "Formosaic" }

    fn register_renderers(&mut self, pipeline: &mut formosaic_engine::rendering::pipeline::Pipeline) {
        use crate::rendering::{
            menu_render::MenuRenderer,
            hint_render::HintRenderer,
            shine_render::ShineRenderer,
        };
        match MenuRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("MenuRenderer failed to init: {e}"),
        }
        match HintRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("HintRenderer failed to init: {e}"),
        }
        match ShineRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("ShineRenderer failed to init: {e}"),
        }
    }


    fn configure_imgui(&self, imgui: &mut imgui::Context, scale: f32) {
        imgui.set_ini_filename(None);
        let font_size   = (16.0_f32 * scale).round().max(16.0);
        let imgui_scale = 1.0 / scale;
        imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);
        imgui.io_mut().font_global_scale = imgui_scale;
        let style = imgui.style_mut();
        style.window_rounding    = 6.0;
        style.frame_rounding     = 4.0;
        style.popup_rounding     = 4.0;
        style.scrollbar_rounding = 4.0;
        style.grab_rounding      = 4.0;
        style.child_rounding     = 4.0;
        style.window_border_size = 1.0;
        style.frame_border_size  = 0.0;
        style.item_spacing       = [8.0, 6.0];
        style.frame_padding      = [8.0, 4.0];

        // ── Palette ─────────────────────────────────────────────────────
        // Base: deep slate-teal to match the 3D background (0.05, 0.08, 0.12)
        // Accent: warm amber/gold — contrast against the cool bg
        // Text: cool off-white with good readability
        style[imgui::StyleColor::WindowBg]          = [0.06, 0.09, 0.13, 0.0];   // transparent — 3D shows through
        style[imgui::StyleColor::ChildBg]           = [0.03, 0.03, 0.05, 0.88];  // near-black panels, matches bg
        style[imgui::StyleColor::PopupBg]           = [0.07, 0.10, 0.16, 0.96];
        style[imgui::StyleColor::Border]            = [0.20, 0.30, 0.45, 0.60];
        style[imgui::StyleColor::BorderShadow]      = [0.0,  0.0,  0.0,  0.0];

        style[imgui::StyleColor::FrameBg]           = [0.10, 0.14, 0.22, 0.90];
        style[imgui::StyleColor::FrameBgHovered]    = [0.14, 0.20, 0.32, 0.90];
        style[imgui::StyleColor::FrameBgActive]     = [0.16, 0.24, 0.38, 1.0];

        style[imgui::StyleColor::TitleBg]           = [0.06, 0.09, 0.14, 1.0];
        style[imgui::StyleColor::TitleBgActive]     = [0.08, 0.13, 0.20, 1.0];
        style[imgui::StyleColor::TitleBgCollapsed]  = [0.04, 0.06, 0.10, 0.80];

        style[imgui::StyleColor::MenuBarBg]         = [0.06, 0.09, 0.14, 1.0];

        // Amber/gold accent for interactive elements
        style[imgui::StyleColor::Button]            = [0.68, 0.48, 0.12, 0.80];
        style[imgui::StyleColor::ButtonHovered]     = [0.85, 0.62, 0.18, 0.95];
        style[imgui::StyleColor::ButtonActive]      = [0.95, 0.72, 0.20, 1.0];

        style[imgui::StyleColor::Header]            = [0.68, 0.48, 0.12, 0.40];
        style[imgui::StyleColor::HeaderHovered]     = [0.68, 0.48, 0.12, 0.70];
        style[imgui::StyleColor::HeaderActive]      = [0.85, 0.62, 0.18, 1.0];

        style[imgui::StyleColor::SliderGrab]        = [0.68, 0.48, 0.12, 0.90];
        style[imgui::StyleColor::SliderGrabActive]  = [0.90, 0.65, 0.20, 1.0];
        style[imgui::StyleColor::CheckMark]         = [0.85, 0.62, 0.18, 1.0];

        style[imgui::StyleColor::ScrollbarBg]       = [0.04, 0.06, 0.10, 0.60];
        style[imgui::StyleColor::ScrollbarGrab]     = [0.20, 0.30, 0.45, 0.80];
        style[imgui::StyleColor::ScrollbarGrabHovered] = [0.30, 0.42, 0.60, 0.90];
        style[imgui::StyleColor::ScrollbarGrabActive]  = [0.68, 0.48, 0.12, 1.0];

        style[imgui::StyleColor::Separator]         = [0.20, 0.30, 0.45, 0.50];
        style[imgui::StyleColor::SeparatorHovered]  = [0.68, 0.48, 0.12, 0.78];
        style[imgui::StyleColor::SeparatorActive]   = [0.85, 0.62, 0.18, 1.0];

        // Text: bright cool-white for readability over dark bg
        style[imgui::StyleColor::Text]              = [0.88, 0.90, 0.96, 1.0];
        style[imgui::StyleColor::TextDisabled]      = [0.36, 0.42, 0.56, 1.0];

        style[imgui::StyleColor::ResizeGrip]        = [0.68, 0.48, 0.12, 0.20];
        style[imgui::StyleColor::ResizeGripHovered] = [0.68, 0.48, 0.12, 0.67];
        style[imgui::StyleColor::ResizeGripActive]  = [0.85, 0.62, 0.18, 0.95];

        style[imgui::StyleColor::Tab]               = [0.08, 0.12, 0.20, 0.90];
        style[imgui::StyleColor::TabHovered]        = [0.68, 0.48, 0.12, 0.80];
        style[imgui::StyleColor::TabActive]         = [0.55, 0.38, 0.10, 1.0];

        #[cfg(target_os = "android")]
        { style.touch_extra_padding = [8.0, 8.0]; }
    }
}

// ─── Camera-driven lerp formula (extracted for testability) ────────────────
//
// When the camera looks within CAMERA_FADE_DOT of the solution direction, the
// model smoothly un-scrambles.  t=1 = fully scrambled, t=0 = fully solved.

pub fn camera_scramble_t(dot: f32) -> f32 {
    if dot >= CAMERA_FADE_DOT {
        1.0 - (dot - CAMERA_FADE_DOT) / (1.0 - CAMERA_FADE_DOT)
    } else {
        1.0
    }
}


