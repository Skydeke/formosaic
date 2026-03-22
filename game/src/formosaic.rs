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
use std::{cell::RefCell, rc::Rc, time::Instant};

use formosaic_engine::{
    architecture::{
        models::{model_loader::ModelLoader, simple_model::SimpleModel},
        scene::{
            entity::simple_entity::SimpleEntity, node::node::NodeBehavior,
            scene_context::SceneContext,
        },
    },
    input::{Event, Key},
    rendering::{instances::camera::orbit_controller::OrbitController, render_state::LightConfig},
};
use imgui;

use crate::{
    level::{
        poly_pizza::{ModelDownload, ModelSummary, PolyPizzaClient},
        storage::{LevelMeta, LevelRegistry},
    },
    puzzle::{
        entropy::{best_scramble_axis, difficulty_label, EntropyReport},
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
    pub elapsed_secs: f32,
    pub difficulty: Option<f32>,
    pub hint_count: u32,
    pub hint_tier: HintTier,
    pub hint_warmth: f32,
    pub is_solved: bool,
    pub is_downloading: bool,
    pub show_menu: bool,
    pub is_touch: bool,
    // ── Menu ─────────────────────────────────────────────────────────────
    pub levels: Vec<LevelMeta>,
    // ── Events fired by UI back to game logic ────────────────────────────
    /// Key events queued by UiNode callbacks; drained by Formosaic::on_update.
    pub queued_events: Vec<formosaic_engine::input::Event>,
    /// Specific level ID to play — set by the Play button, drained by on_update.
    pub play_specific: Option<String>,
}

// ─── Constants ────────────────────────────────────────────────────────────────

const TARGET_WORLD_RADIUS: f32 = 1.0;
const CAMERA_FOV: f32 = 75.0 * std::f32::consts::PI / 180.0;
const SNAP_THRESHOLD_DOT: f32 = 0.996;
const RESTORE_DURATION: f32 = 1.8;
/// Number of candidate axes tested in the entropy search.
const ENTROPY_CANDIDATES: usize = 32;

// ─── State machine ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    Playing,
    Restoring {
        elapsed: f32,
        cam_start: Vector3<f32>,
        cam_end: Vector3<f32>,
    },
    Solved,
}

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    LevelSelect,
    InGame { level_id: String },
    Downloading { summary: ModelSummary },
}

// ─── Main struct ──────────────────────────────────────────────────────────────

pub struct Formosaic {
    model: Option<Rc<RefCell<SimpleModel>>>,
    entity: Option<Rc<RefCell<SimpleEntity>>>,
    orbit: Option<OrbitController>,
    scramble_state: Option<ScrambleState>,
    entropy_report: Option<EntropyReport>,
    game_state: GameState,
    hints: HintSystem,
    /// Cached output from the last hints.update() call — read by the renderer.
    last_hint_output: Option<HintOutput>,
    elapsed_secs: f32,
    level_start: Option<Instant>,
    registry: LevelRegistry,
    client: PolyPizzaClient,
    mode: AppMode,
    solved_timer: f32, // seconds since solve; transitions to menu after 5s
    /// Scene lighting config — passed to the pipeline each frame via GameFrameData.
    scene_lights: LightConfig,
    /// Shared state read by UiNodes each frame.  Written in populate_scene_context.
    ui_state: Rc<RefCell<UiState>>,
}

impl Formosaic {
    pub fn new() -> Self {
        let data_dir = LevelRegistry::default_data_dir();
        let registry = LevelRegistry::load(&data_dir);
        log::info!(
            "[Formosaic] Data dir: {}  ({} saved levels)",
            data_dir.display(),
            registry.levels.len()
        );
        Self {
            model: None,
            entity: None,
            orbit: None,
            scramble_state: None,
            entropy_report: None,
            game_state: GameState::Playing,
            hints: HintSystem::new(),
            last_hint_output: None,
            elapsed_secs: 0.0,
            level_start: None,
            registry,
            client: PolyPizzaClient::new(),
            mode: AppMode::LevelSelect,
            solved_timer: 0.0,
            scene_lights: LightConfig::default(),
            ui_state: Rc::new(RefCell::new(UiState::default())),
        }
    }

    // ── Level loading ──────────────────────────────────────────────────────

    fn load_model_and_start(&mut self, path: &str, level_id: &str, ctx: &mut SceneContext) {
        // Clear scene then immediately re-register UiNodes — they live in the
        // scenegraph and scene.clear() would otherwise wipe them on every load.
        if let Some(scene) = ctx.scene() {
            scene.clear();
        }
        self.register_ui_nodes(ctx);
        self.model = None;
        self.entity = None;
        self.orbit = None;
        self.hints.reset();

        let bytes = crate::asset_loader::load_3d_asset(path);
        let model = ModelLoader::load_from_bytes_with_path(path, &bytes);

        // ── Entropy-based axis selection ───────────────────────────────────
        let search = best_scramble_axis(
            &model.borrow(),
            ENTROPY_CANDIDATES,
            TARGET_WORLD_RADIUS,
            CAMERA_FOV,
        );
        self.entropy_report = Some(search.report);
        log::info!(
            "[Formosaic] difficulty={} ({:.2})  entropy={:.2} bits  isolation={:.1}°",
            difficulty_label(search.report.difficulty),
            search.report.difficulty,
            search.report.entropy_bits,
            search.report.solution_isolation_rad.to_degrees(),
        );

        let params = model
            .borrow()
            .compute_puzzle_params(TARGET_WORLD_RADIUS, CAMERA_FOV);
        model
            .borrow_mut()
            .scramble_along_axis(search.axis, params.min_disp, params.max_disp);

        if let Some(scene) = ctx.scene() {
            let entity = Rc::new(RefCell::new(SimpleEntity::new(model.clone())));

            let s = params.entity_scale;
            entity
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(s, s, s));
            entity
                .borrow_mut()
                .transform_mut()
                .add_rotation_euler_world(0.0, 180.0, 0.0);
            entity
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(0.0, 0.0, 0.0));

            let entity_rot = entity.borrow().transform().rotation;
            let solution_dir = (entity_rot * search.axis).normalize();

            self.scramble_state = Some(ScrambleState {
                solution_dir,
                params,
            });

            scene.add_node(entity.clone());
            self.entity = Some(entity.clone());

            if let Some(camera) = ctx.camera() {
                let centroid = entity.borrow().centroid();
                let dist = params.orbit_distance;
                let (ctrl, sp) = make_scrambled_orbit(centroid, dist, solution_dir);
                camera.borrow_mut().transform.position = sp;
                self.orbit = Some(OrbitController::new(centroid, dist));
                camera.borrow_mut().set_controller(Some(Box::new(ctrl)));
            }
        }

        self.model = Some(model);
        self.game_state = GameState::Playing;
        self.level_start = Some(Instant::now());
        self.elapsed_secs = 0.0;
        self.mode = AppMode::InGame {
            level_id: level_id.to_string(),
        };
        self.solved_timer = 0.0;
    }

    fn load_builtin_cactus(&mut self, ctx: &mut SceneContext) {
        self.load_model_and_start("models/Cactus/cactus.fbx", "cactus_builtin", ctx);
    }

    /// Ensure the builtin cactus level exists in the registry so it shows up
    /// in the menu grid on first run.  Safe to call every launch — it's a no-op
    /// if the level is already saved.
    fn seed_builtin_levels(&mut self) {
        let id = "cactus_builtin";
        if self.registry.levels.iter().any(|l| l.id == id) {
            return;
        }

        let meta = LevelMeta {
            id: id.to_string(),
            name: "Cactus".to_string(),
            author: "SoyMaria".to_string(),
            license: "CC-BY 3.0".to_string(),
            source_url: "https://poly.pizza/m/7S5Snphkam".to_string(),
            model_file: "cactus.fbx".to_string(),
            best_time_secs: None,
            play_count: 0,
            difficulty: 0.3,
        };

        let bytes = crate::asset_loader::load_3d_asset("models/Cactus/cactus.fbx");
        if let Err(e) = self.registry.save_level(meta, &bytes) {
            log::warn!("[Formosaic] Failed to seed builtin cactus: {e}");
        }
    }

    fn load_random_saved(&mut self, ctx: &mut SceneContext) {
        if let Some(meta) = self.registry.random_level() {
            let path = self.registry.model_path(meta);
            let id = meta.id.clone();
            if path.exists() {
                self.load_model_and_start(path.to_string_lossy().as_ref(), &id, ctx);
                return;
            }
        }
        self.load_builtin_cactus(ctx);
    }

    fn fetch_online_level(&mut self) {
        if !self.client.is_explore_pending() {
            // Page 0 is fetched first to discover total, then a random valid
            // page is chosen — all handled inside fetch_explore.
            self.client.fetch_explore_page(0, 20);
            log::info!("[Formosaic] Fetching poly.pizza random page");
        }
    }

    // ── Puzzle ─────────────────────────────────────────────────────────────

    fn trigger_solve(&mut self, ctx: &mut SceneContext) {
        if self.game_state != GameState::Playing {
            return;
        }
        let solution_dir = match &self.scramble_state {
            Some(s) => s.solution_dir,
            None => return,
        };

        log::info!(
            "[Formosaic] SOLVED! time={:.1}s hints={}",
            self.elapsed_secs,
            self.hints.hint_count()
        );

        if let Some(camera) = ctx.camera() {
            let cam = camera.borrow();
            let target = self
                .orbit
                .as_ref()
                .map(|o| o.target)
                .unwrap_or(cam.transform.position);
            let dist = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
            let fwd = cam.transform.forward().normalize();
            let dir = if fwd.dot(solution_dir) > 0.0 {
                solution_dir
            } else {
                -solution_dir
            };
            let cam_end = target - dir * dist;
            let cam_start = cam.transform.position;
            drop(cam);
            camera.borrow_mut().set_controller(None);
            self.game_state = GameState::Restoring {
                elapsed: 0.0,
                cam_start,
                cam_end,
            };
        }

        if let Some(start) = self.level_start {
            self.elapsed_secs = start.elapsed().as_secs_f32();
        }
    }

    fn finish_restore(&mut self, ctx: &mut SceneContext) {
        if let Some(camera) = ctx.camera() {
            let target = self
                .orbit
                .as_ref()
                .map(|o| o.target)
                .unwrap_or_else(|| camera.borrow().transform.position);
            let dist = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
            let pos = camera.borrow().transform.position;
            let mut post = OrbitController::new(target, dist);
            post.set_initial_position(pos);
            camera.borrow_mut().set_controller(Some(Box::new(post)));
        }
        self.game_state = GameState::Solved;
        self.solved_timer = 0.0;

        let level_id = match &self.mode {
            AppMode::InGame { level_id } => level_id.clone(),
            _ => return,
        };
        self.registry
            .record_completion(&level_id, self.elapsed_secs);
    }

    // ── Download polling ────────────────────────────────────────────────────

    // ── Download polling ────────────────────────────────────────────────────

    fn poll_client(&mut self, ctx: &mut SceneContext) {
        // Poll explore results — handle Ok, empty Ok, and Err separately
        if let Some(result) = self.client.poll_explore() {
            match result {
                Ok(summaries) if !summaries.is_empty() => {
                    use rand::Rng;
                    let idx = rand::rng().random_range(0..summaries.len());
                    let summary = summaries[idx].clone();
                    self.mode = AppMode::Downloading {
                        summary: summary.clone(),
                    };
                    self.client.download_model(&summary);
                    log::info!("[Formosaic] Downloading '{}'", summary.name);
                }
                Ok(_) => {
                    log::warn!("[Formosaic] Poly Pizza returned 0 models — loading cactus");
                    self.load_builtin_cactus(ctx);
                }
                Err(e) => {
                    log::warn!("[Formosaic] Poly Pizza fetch error: {} — loading cactus", e);
                    self.load_builtin_cactus(ctx);
                }
            }
        }

        // Poll download results
        if let Some(result) = self.client.poll_download() {
            match result {
                Ok(dl) => self.on_download_complete(dl, ctx),
                Err(e) => {
                    log::warn!("[Formosaic] Download failed: {} — loading cactus", e);
                    self.load_builtin_cactus(ctx);
                }
            }
        }
    }

    fn on_download_complete(&mut self, dl: ModelDownload, ctx: &mut SceneContext) {
        let meta = LevelMeta {
            id: dl.id.clone(),
            name: dl.name.clone(),
            author: dl.author.clone(),
            license: dl.license.clone(),
            source_url: dl.source_url.clone(),
            model_file: format!("model.{}", dl.file_ext),
            best_time_secs: None,
            play_count: 0,
            difficulty: 0.5, // updated after entropy analysis below
        };

        if let Err(e) = self.registry.save_level(meta.clone(), &dl.bytes) {
            log::warn!("[Formosaic] Failed to save level '{}': {}", dl.name, e);
        }

        let path = self.registry.model_path(&meta);
        self.load_model_and_start(path.to_string_lossy().as_ref(), &meta.id, ctx);

        // Persist the entropy-derived difficulty now that analysis has run.
        if let Some(report) = self.entropy_report {
            let data_dir = LevelRegistry::default_data_dir();
            let meta_path = data_dir.join("levels").join(&dl.id).join("meta.json");
            if let Some(m) = self.registry.levels.iter_mut().find(|m| m.id == dl.id) {
                m.difficulty = report.difficulty;
                let _ = std::fs::write(meta_path, m.to_json());
            }
        }
    }

    // ── Public accessors (read by game_engine / pipeline) ──────────────

    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed_secs
    }
    pub fn solved_timer(&self) -> f32 {
        self.solved_timer
    }
    pub fn entropy_report(&self) -> Option<&EntropyReport> {
        self.entropy_report.as_ref()
    }
    pub fn is_solved(&self) -> bool {
        self.game_state == GameState::Solved
    }
    pub fn is_downloading(&self) -> bool {
        matches!(self.mode, AppMode::Downloading { .. }) || self.client.is_download_pending()
    }
    pub fn saved_levels(&self) -> &[LevelMeta] {
        &self.registry.levels
    }
    pub fn hint_system(&self) -> &HintSystem {
        &self.hints
    }
    pub fn hint_system_mut(&mut self) -> &mut HintSystem {
        &mut self.hints
    }
    /// Cached hint output from this frame — never call hints.update() externally.
    pub fn last_hint_output(&self) -> Option<&HintOutput> {
        self.last_hint_output.as_ref()
    }

    /// True when the level-select / menu overlay should be shown.
    pub fn is_in_menu(&self) -> bool {
        matches!(self.mode, AppMode::LevelSelect)
    }

    /// Index of the currently highlighted level in the menu grid.
    pub fn selected_level_idx(&self) -> usize {
        0
    }

    pub fn solution_dir(&self) -> Option<Vector3<f32>> {
        self.scramble_state.as_ref().map(|s| s.solution_dir)
    }

    /// Current scene lighting configuration.
    pub fn lights(&self) -> LightConfig {
        self.scene_lights
    }

    /// Default lighting — used by game_engine for initial GL clear colour.
    pub fn default_lights() -> LightConfig {
        LightConfig::default()
    }

    // ── UiNode registration ────────────────────────────────────────────────
    //
    // Each panel is a named UiNode added to the scenegraph.  The engine
    // collects and renders them automatically every frame.
    // show/hide them at runtime via `node.borrow_mut().set_hidden(true)`.

    fn register_ui_nodes(&self, ctx: &mut SceneContext) {
        use crate::puzzle::entropy::difficulty_label;
        use formosaic_engine::architecture::scene::node::ui_node::UiNode;
        use formosaic_engine::input::{Event, Key};
        use imgui::*;

        let Some(scene) = ctx.scene() else { return };

        // ── Helpers (shared across closures via free fns to avoid captures) ──
        fn diff_color(d: f32) -> [f32; 4] {
            match d {
                v if v < 0.25 => [0.18, 0.75, 0.50, 1.0],
                v if v < 0.50 => [0.75, 0.63, 0.18, 1.0],
                v if v < 0.75 => [0.75, 0.19, 0.29, 1.0],
                _ => [0.63, 0.18, 0.75, 1.0],
            }
        }
        fn diff_label_str(d: f32) -> &'static str {
            match d {
                v if v < 0.25 => "Easy",
                v if v < 0.50 => "Medium",
                v if v < 0.75 => "Hard",
                _ => "Expert",
            }
        }
        fn truncate(s: &str, max: usize) -> String {
            if s.len() > max {
                format!("{}..", &s[..max])
            } else {
                s.to_string()
            }
        }

        let hud_flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_INPUTS
            | WindowFlags::NO_SAVED_SETTINGS;

        // ── HUD (top-right, in-game only) ─────────────────────────────────
        {
            let state = Rc::clone(&self.ui_state);
            let hud = UiNode::new("hud", move |ui, w, _h, _ctx| {
                let s = state.borrow();
                if s.show_menu {
                    return;
                }
                ui.window("##hud")
                    .flags(hud_flags)
                    .position([w - 220.0, 10.0], Condition::Always)
                    .size([210.0, 100.0], Condition::Always)
                    .build(|| {
                        if let Some(diff) = s.difficulty {
                            ui.text_colored(
                                [0.8, 0.85, 0.95, 0.9],
                                format!("{:.1}s  |  {}", s.elapsed_secs, difficulty_label(diff)),
                            );
                        }
                        if s.hint_count > 0 {
                            ui.text_colored(
                                [0.9, 0.6, 0.2, 0.8],
                                format!("{} hint(s)", s.hint_count),
                            );
                        }
                        if s.is_solved {
                            ui.text_colored([0.2, 0.9, 0.5, 1.0], "SOLVED!");
                        }
                        if s.is_downloading {
                            ui.text_colored([0.4, 0.7, 1.0, 0.8], "Fetching...");
                        }
                    });
            });
            scene.add_node(Rc::new(RefCell::new(hud)));
        }

        // ── Hint warmth indicator (top-left, in-game only) ────────────────
        {
            let state = Rc::clone(&self.ui_state);
            let hints_node = UiNode::new("hint_warmth", move |ui, _w, h, _ctx| {
                let s = state.borrow();
                if s.show_menu {
                    return;
                }
                if s.hint_tier == crate::puzzle::hints::HintTier::None {
                    return;
                }
                ui.window("##hints")
                    .flags(hud_flags)
                    .position([10.0, h - 70.0], Condition::Always)
                    .size([180.0, 60.0], Condition::Always)
                    .build(|| {
                        let w = s.hint_warmth;
                        let cold = [0.2_f32, 0.4, 0.9, 1.0];
                        let hot = [0.9_f32, 0.2, 0.1, 1.0];
                        let col = [
                            cold[0] + (hot[0] - cold[0]) * w,
                            cold[1] + (hot[1] - cold[1]) * w,
                            cold[2] + (hot[2] - cold[2]) * w,
                            1.0,
                        ];
                        let label = if w > 0.8 {
                            "HOT"
                        } else if w > 0.5 {
                            "WARM"
                        } else {
                            "COLD"
                        };
                        ui.text_colored(col, format!(">> {} <<", label));
                        ui.text_colored([0.5, 0.5, 0.5, 0.7], format!("Hint: {:?}", s.hint_tier));
                    });
            });
            scene.add_node(Rc::new(RefCell::new(hints_node)));
        }

        // ── Android touch buttons (bottom-right, in-game only) ────────────
        #[cfg(target_os = "android")]
        {
            let state = Rc::clone(&self.ui_state);
            let touch = UiNode::new("touch_buttons", move |ui, w, h, ctx| {
                let s = state.borrow();
                if s.show_menu {
                    return;
                }
                let btn_h = (h * 0.10).max(44.0);
                let btn_w = (w * 0.14).max(80.0);
                let margin = w * 0.012;
                let win_w = btn_w * 2.0 + margin * 3.0;
                let win_h = btn_h + margin * 2.0;

                let mut hint_clicked = false;
                let mut esc_clicked = false;
                ui.window("##touch_btns")
                    .flags(
                        WindowFlags::NO_DECORATION
                            | WindowFlags::NO_MOVE
                            | WindowFlags::NO_BACKGROUND
                            | WindowFlags::NO_SAVED_SETTINGS,
                    )
                    .position([w - win_w, h - win_h], Condition::Always)
                    .size([win_w, win_h], Condition::Always)
                    .build(|| {
                        ui.set_cursor_pos([margin, margin]);
                        hint_clicked = ui.button_with_size("HINT", [btn_w, btn_h]);
                        ui.same_line_with_spacing(0.0, margin);
                        esc_clicked = ui.button_with_size("ESC", [btn_w, btn_h]);
                    });
                drop(s);
                if hint_clicked {
                    state
                        .borrow_mut()
                        .queued_events
                        .push(Event::KeyDown { key: Key::H });
                }
                if esc_clicked {
                    state
                        .borrow_mut()
                        .queued_events
                        .push(Event::KeyDown { key: Key::Escape });
                }
            });
            scene.add_node(Rc::new(RefCell::new(touch)));
        }

        // ── Main menu overlay (full-screen) ───────────────────────────────
        {
            let state = Rc::clone(&self.ui_state);
            let menu = UiNode::new("menu", move |ui, w, h, _ctx| {
                let s = state.borrow();
                if !s.show_menu {
                    return;
                }
                let is_touch = s.is_touch;
                let levels = s.levels.clone();
                let is_dl = s.is_downloading;

                let menu_flags = WindowFlags::NO_DECORATION
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_SAVED_SETTINGS
                    | WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_SCROLL_WITH_MOUSE;
                let pad = (w * 0.025).max(8.0);

                ui.window("##menu")
                    .flags(menu_flags)
                    .position([0.0, 0.0], Condition::Always)
                    .size([w, h], Condition::Always)
                    .build(|| {
                        let mut fetch_online = false;
                        let mut random_saved = false;
                        let mut play_level_id: Option<String> = None;

                        if is_touch {
                            ui.set_cursor_pos([pad, pad]);
                            ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                            ui.same_line();
                            ui.text_colored([0.36, 0.42, 0.56, 0.8], " v0.1");
                            let btn_w = w - pad * 2.0;
                            let btn_h = (h * 0.09).max(44.0);
                            let gap = pad * 0.5;
                            ui.set_cursor_pos([pad, h * 0.12]);
                            if ui.button_with_size("[N] Fetch Online", [btn_w, btn_h]) {
                                fetch_online = true;
                            }
                            ui.set_cursor_pos([pad, h * 0.12 + btn_h + gap]);
                            if ui.button_with_size("[R] Random Saved", [btn_w, btn_h]) {
                                random_saved = true;
                            }
                            if is_dl {
                                ui.set_cursor_pos([pad, h * 0.12 + (btn_h + gap) * 2.0]);
                                ui.text_colored([0.4, 0.7, 1.0, 1.0], "Fetching...");
                            }
                            let list_y = h * 0.12 + (btn_h + gap) * 2.0 + pad;
                            let list_h = h - list_y - pad;
                            let card_h = (h * 0.20).max(80.0);
                            let card_w = (w - pad * 3.0) * 0.5;
                            ui.set_cursor_pos([pad, list_y]);
                            ui.child_window("##levels")
                                .size([w - pad * 2.0, list_h])
                                .border(false)
                                .build(|| {
                                    if levels.is_empty() {
                                        ui.spacing();
                                        ui.text_colored(
                                            [0.36, 0.42, 0.56, 1.0],
                                            "No saved levels yet.",
                                        );
                                        ui.text_colored(
                                            [0.36, 0.42, 0.56, 0.8],
                                            "Use [N] to fetch from Poly Pizza.",
                                        );
                                    } else {
                                        let mut col = 0usize;
                                        for level in &levels {
                                            ui.child_window(format!("##c_{}", level.id))
                                                .size([card_w, card_h])
                                                .border(true)
                                                .build(|| {
                                                    ui.text_colored(
                                                        [0.88, 0.90, 0.96, 1.0],
                                                        truncate(&level.name, 12),
                                                    );
                                                    ui.text_colored(
                                                        [0.36, 0.42, 0.56, 0.8],
                                                        truncate(&level.author, 14),
                                                    );
                                                    ui.text_colored(
                                                        diff_color(level.difficulty),
                                                        diff_label_str(level.difficulty),
                                                    );
                                                    if let Some(t) = level.best_time_secs {
                                                        ui.text_colored(
                                                            [0.36, 0.42, 0.56, 0.7],
                                                            format!("{:.1}s", t),
                                                        );
                                                    }
                                                    let play_h = (card_h * 0.3).max(24.0);
                                                    if ui.button_with_size(
                                                        "Play",
                                                        [card_w - 8.0, play_h],
                                                    ) {
                                                        play_level_id = Some(level.id.clone());
                                                    }
                                                });
                                            col += 1;
                                            if col < 2 {
                                                ui.same_line();
                                            } else {
                                                col = 0;
                                            }
                                        }
                                    }
                                });
                        } else {
                            let sidebar_w = (w * 0.28).max(150.0);
                            let btn_size = [sidebar_w - pad * 2.0, 30.0];
                            let card_h = 112.0_f32;
                            ui.set_cursor_pos([pad, pad]);
                            ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                            ui.same_line();
                            ui.text_colored([0.36, 0.42, 0.56, 0.8], "  puzzle  v0.1");
                            ui.set_cursor_pos([pad, h * 0.42]);
                            if ui.button_with_size("[N] Fetch Online", btn_size) {
                                fetch_online = true;
                            }
                            ui.set_cursor_pos([pad, h * 0.42 + 38.0]);
                            if ui.button_with_size("[R] Random Saved", btn_size) {
                                random_saved = true;
                            }
                            if is_dl {
                                ui.set_cursor_pos([pad, h * 0.42 + 78.0]);
                                ui.text_colored([0.4, 0.7, 1.0, 1.0], "Fetching...");
                            }
                            ui.set_cursor_pos([pad, h - 20.0]);
                            ui.text_colored([0.28, 0.34, 0.46, 0.7], "v0.1.0");
                            let content_x = sidebar_w + pad;
                            let content_w = w - content_x - pad;
                            let content_y = h * 0.08;
                            ui.set_cursor_pos([content_x, content_y]);
                            let _tok = ui
                                .push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
                            ui.child_window("##levels")
                                .size([content_w, h - content_y - 28.0])
                                .border(false)
                                .build(|| {
                                    if levels.is_empty() {
                                        ui.spacing();
                                        ui.spacing();
                                        ui.text_colored(
                                            [0.36, 0.42, 0.56, 1.0],
                                            "No saved levels yet.",
                                        );
                                        ui.spacing();
                                        ui.text_colored(
                                            [0.36, 0.42, 0.56, 0.8],
                                            "Press [N] to fetch a model from Poly Pizza.",
                                        );
                                    } else {
                                        let avail = ui.content_region_avail()[0];
                                        let cols = 3usize;
                                        let cw =
                                            ((avail - 8.0 * cols as f32) / cols as f32).max(80.0);
                                        let mut col = 0usize;
                                        for level in &levels {
                                            ui.child_window(format!("##c_{}", level.id))
                                                .size([cw, card_h])
                                                .border(true)
                                                .scroll_bar(false)
                                                .build(|| {
                                                    ui.text_colored(
                                                        [0.88, 0.90, 0.96, 1.0],
                                                        truncate(&level.name, 14),
                                                    );
                                                    ui.text_colored(
                                                        [0.36, 0.42, 0.56, 0.8],
                                                        truncate(&level.author, 16),
                                                    );
                                                    ui.text_colored(
                                                        diff_color(level.difficulty),
                                                        diff_label_str(level.difficulty),
                                                    );
                                                    if let Some(t) = level.best_time_secs {
                                                        ui.same_line();
                                                        ui.text_colored(
                                                            [0.36, 0.42, 0.56, 0.7],
                                                            format!("  {:.1}s", t),
                                                        );
                                                    }
                                                    if ui
                                                        .button_with_size("Play", [cw - 12.0, 22.0])
                                                    {
                                                        play_level_id = Some(level.id.clone());
                                                    }
                                                });
                                            col += 1;
                                            if col < cols {
                                                ui.same_line();
                                            } else {
                                                col = 0;
                                            }
                                        }
                                    }
                                });
                            ui.set_cursor_pos([content_x, h - 20.0]);
                            ui.text_colored(
                                [0.28, 0.34, 0.46, 0.7],
                                "Models: Poly Pizza (poly.pizza) CC-BY",
                            );
                        }

                        // Queue events (buttons cannot call self directly from a closure)
                        drop(s);
                        if fetch_online {
                            state
                                .borrow_mut()
                                .queued_events
                                .push(Event::KeyDown { key: Key::N });
                        }
                        if random_saved {
                            state
                                .borrow_mut()
                                .queued_events
                                .push(Event::KeyDown { key: Key::R });
                        }
                        if let Some(id) = play_level_id {
                            state.borrow_mut().play_specific = Some(id);
                        }
                    });
            });
            scene.add_node(Rc::new(RefCell::new(menu)));
        }
    }
}

impl Default for Formosaic {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Application impl ─────────────────────────────────────────────────────────

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Application for Formosaic {
    fn on_init(&mut self, ctx: &mut SceneContext) {
        log::info!("[Formosaic] on_init");
        // Seed the builtin cactus into the registry so it appears in the menu
        // grid on first run (no-op if already saved).
        self.seed_builtin_levels();
        // Register UI nodes and open the level-select menu.
        self.register_ui_nodes(ctx);
        // mode is already AppMode::LevelSelect from Self::new().
    }

    fn on_update(&mut self, delta_time: f32, ctx: &mut SceneContext) {
        // Drain play_specific first — a specific level ID from the menu Play button.
        let play_id = self.ui_state.borrow_mut().play_specific.take();
        if let Some(id) = play_id {
            if let Some(meta) = self.registry.levels.iter().find(|m| m.id == id).cloned() {
                let path = self.registry.model_path(&meta);
                if path.exists() {
                    self.load_model_and_start(path.to_string_lossy().as_ref(), &meta.id, ctx);
                }
            }
        }

        // Drain any key events queued by UiNode callbacks.
        let queued: Vec<_> = self.ui_state.borrow_mut().queued_events.drain(..).collect();
        for ev in queued {
            self.on_event(&ev, ctx);
        }

        self.poll_client(ctx);

        // Update hints once per frame and cache for the renderer.
        self.last_hint_output =
            if let (Some(sc), Some(camera)) = (&self.scramble_state, ctx.camera()) {
                let fwd = camera.borrow().transform.forward().normalize();
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

        let mut do_solve = false;
        let mut do_restore_complete = false;
        let mut cam_pos: Option<Vector3<f32>> = None;
        let mut cam_target: Option<Vector3<f32>> = None;

        match &mut self.game_state {
            GameState::Playing => {
                self.elapsed_secs += delta_time;
                let snap = if let (Some(sc), Some(cam)) = (&self.scramble_state, ctx.camera()) {
                    cam.borrow()
                        .transform
                        .forward()
                        .normalize()
                        .dot(sc.solution_dir.normalize())
                        .abs()
                        >= SNAP_THRESHOLD_DOT
                } else {
                    false
                };
                if snap {
                    do_solve = true;
                }
            }
            GameState::Restoring {
                elapsed,
                cam_start,
                cam_end,
            } => {
                *elapsed += delta_time;
                let ev = *elapsed;
                let tri_t = if ev >= RESTORE_DURATION {
                    do_restore_complete = true;
                    0.0
                } else {
                    1.0 - smoothstep(ev / RESTORE_DURATION)
                };
                if let Some(m) = &self.model {
                    m.borrow().upload_lerp(tri_t);
                }
                let t = smoothstep((ev / RESTORE_DURATION).min(1.0));
                cam_pos = Some(*cam_start + (*cam_end - *cam_start) * t);
                cam_target = self.orbit.as_ref().map(|o| o.target);
            }
            GameState::Solved => {
                self.solved_timer += delta_time;
                if self.solved_timer >= 5.0 {
                    self.mode = AppMode::LevelSelect;
                }
            }
        }

        if let (Some(pos), Some(tgt)) = (cam_pos, cam_target) {
            if let Some(camera) = ctx.camera() {
                let mut c = camera.borrow_mut();
                c.transform.position = pos;
                c.transform.look_at(tgt, Vector3::unit_y());
            }
        }

        if do_solve {
            self.trigger_solve(ctx);
        }
        if do_restore_complete {
            self.finish_restore(ctx);
        }
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
            }
            _ => {
                if matches!(self.game_state, GameState::Playing | GameState::Solved) {
                    if let Some(camera) = ctx.camera() {
                        let (w, h) = {
                            let c = camera.borrow();
                            (c.resolution.x as f32, c.resolution.y as f32)
                        };
                        camera.borrow_mut().controller.handle_event(event, w, h);
                    }
                }
            }
        }
    }

    fn populate_scene_context(&mut self, ctx: &mut SceneContext, delta_time: f32) {
        use formosaic_engine::rendering::render_state::HintRenderState;
        let solved = self.game_state == GameState::Solved && !self.is_in_menu();
        let solved_t = self.solved_timer;
        ctx.lights = self.scene_lights;
        ctx.show_menu = self.is_in_menu();
        ctx.is_touch = cfg!(target_os = "android");
        ctx.delta_time = delta_time;
        ctx.solved_timer = if solved { Some(solved_t) } else { None };
        ctx.hints = self.last_hint_output.as_ref().map(|o| HintRenderState {
            warmth: o.warmth,
            warmth_color: o.warmth_color,
            tier: o.tier.as_u8(),
            time: solved_t,
        });

        // Sync game state into UiState so UiNodes can read it this frame.
        {
            let mut ui = self.ui_state.borrow_mut();
            ui.elapsed_secs = self.elapsed_secs;
            ui.difficulty = self.entropy_report.map(|r| r.difficulty);
            ui.hint_count = self.hints.hint_count() as u32;
            ui.hint_tier = self.hints.tier();
            ui.hint_warmth = self
                .last_hint_output
                .as_ref()
                .map(|o| o.warmth)
                .unwrap_or(0.5);
            ui.is_solved = self.game_state == GameState::Solved;
            ui.is_downloading = self.is_downloading();
            ui.show_menu = self.is_in_menu();
            ui.is_touch = cfg!(target_os = "android");
            ui.levels = self.registry.levels.clone();
            // Don't clear queued_events here — on_update drains them.
        }
    }

    fn title(&self) -> &str {
        "Formosaic"
    }

    fn register_renderers(
        &mut self,
        pipeline: &mut formosaic_engine::rendering::pipeline::Pipeline,
    ) {
        use crate::rendering::{
            hint_render::HintRenderer, menu_render::MenuRenderer, shine_render::ShineRenderer,
        };
        match MenuRenderer::new() {
            Ok(r) => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("MenuRenderer failed to init: {e}"),
        }
        match HintRenderer::new() {
            Ok(r) => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("HintRenderer failed to init: {e}"),
        }
        match ShineRenderer::new() {
            Ok(r) => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("ShineRenderer failed to init: {e}"),
        }
    }

    fn configure_imgui(&self, imgui: &mut imgui::Context, scale: f32) {
        imgui.set_ini_filename(None);
        let font_size = (16.0_f32 * scale).round().max(16.0);
        let imgui_scale = 1.0 / scale;
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);
        imgui.io_mut().font_global_scale = imgui_scale;
        let style = imgui.style_mut();
        style.window_rounding = 6.0;
        style.frame_rounding = 4.0;
        style.popup_rounding = 4.0;
        style.scrollbar_rounding = 4.0;
        style.grab_rounding = 4.0;
        style.child_rounding = 4.0;
        style.window_border_size = 1.0;
        style.frame_border_size = 0.0;
        style.item_spacing = [8.0, 6.0];
        style.frame_padding = [8.0, 4.0];

        // ── Palette ─────────────────────────────────────────────────────
        // Base: deep slate-teal to match the 3D background (0.05, 0.08, 0.12)
        // Accent: warm amber/gold — contrast against the cool bg
        // Text: cool off-white with good readability
        style[imgui::StyleColor::WindowBg] = [0.06, 0.09, 0.13, 0.0]; // transparent — 3D shows through
        style[imgui::StyleColor::ChildBg] = [0.03, 0.03, 0.05, 0.88]; // near-black panels, matches bg
        style[imgui::StyleColor::PopupBg] = [0.07, 0.10, 0.16, 0.96];
        style[imgui::StyleColor::Border] = [0.20, 0.30, 0.45, 0.60];
        style[imgui::StyleColor::BorderShadow] = [0.0, 0.0, 0.0, 0.0];

        style[imgui::StyleColor::FrameBg] = [0.10, 0.14, 0.22, 0.90];
        style[imgui::StyleColor::FrameBgHovered] = [0.14, 0.20, 0.32, 0.90];
        style[imgui::StyleColor::FrameBgActive] = [0.16, 0.24, 0.38, 1.0];

        style[imgui::StyleColor::TitleBg] = [0.06, 0.09, 0.14, 1.0];
        style[imgui::StyleColor::TitleBgActive] = [0.08, 0.13, 0.20, 1.0];
        style[imgui::StyleColor::TitleBgCollapsed] = [0.04, 0.06, 0.10, 0.80];

        style[imgui::StyleColor::MenuBarBg] = [0.06, 0.09, 0.14, 1.0];

        // Amber/gold accent for interactive elements
        style[imgui::StyleColor::Button] = [0.68, 0.48, 0.12, 0.80];
        style[imgui::StyleColor::ButtonHovered] = [0.85, 0.62, 0.18, 0.95];
        style[imgui::StyleColor::ButtonActive] = [0.95, 0.72, 0.20, 1.0];

        style[imgui::StyleColor::Header] = [0.68, 0.48, 0.12, 0.40];
        style[imgui::StyleColor::HeaderHovered] = [0.68, 0.48, 0.12, 0.70];
        style[imgui::StyleColor::HeaderActive] = [0.85, 0.62, 0.18, 1.0];

        style[imgui::StyleColor::SliderGrab] = [0.68, 0.48, 0.12, 0.90];
        style[imgui::StyleColor::SliderGrabActive] = [0.90, 0.65, 0.20, 1.0];
        style[imgui::StyleColor::CheckMark] = [0.85, 0.62, 0.18, 1.0];

        style[imgui::StyleColor::ScrollbarBg] = [0.04, 0.06, 0.10, 0.60];
        style[imgui::StyleColor::ScrollbarGrab] = [0.20, 0.30, 0.45, 0.80];
        style[imgui::StyleColor::ScrollbarGrabHovered] = [0.30, 0.42, 0.60, 0.90];
        style[imgui::StyleColor::ScrollbarGrabActive] = [0.68, 0.48, 0.12, 1.0];

        style[imgui::StyleColor::Separator] = [0.20, 0.30, 0.45, 0.50];
        style[imgui::StyleColor::SeparatorHovered] = [0.68, 0.48, 0.12, 0.78];
        style[imgui::StyleColor::SeparatorActive] = [0.85, 0.62, 0.18, 1.0];

        // Text: bright cool-white for readability over dark bg
        style[imgui::StyleColor::Text] = [0.88, 0.90, 0.96, 1.0];
        style[imgui::StyleColor::TextDisabled] = [0.36, 0.42, 0.56, 1.0];

        style[imgui::StyleColor::ResizeGrip] = [0.68, 0.48, 0.12, 0.20];
        style[imgui::StyleColor::ResizeGripHovered] = [0.68, 0.48, 0.12, 0.67];
        style[imgui::StyleColor::ResizeGripActive] = [0.85, 0.62, 0.18, 0.95];

        style[imgui::StyleColor::Tab] = [0.08, 0.12, 0.20, 0.90];
        style[imgui::StyleColor::TabHovered] = [0.68, 0.48, 0.12, 0.80];
        style[imgui::StyleColor::TabActive] = [0.55, 0.38, 0.10, 1.0];

        #[cfg(target_os = "android")]
        {
            style.touch_extra_padding = [8.0, 8.0];
        }
    }
}
