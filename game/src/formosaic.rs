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
        entropy::{best_scramble_axis, difficulty_label, EntropyReport},
        hints::{HintOutput, HintSystem},
        scrambler::{make_scrambled_orbit, ScrambleState},
    },
};

// Application trait lives in the engine — re-use it here.
pub use formosaic_engine::app::application::Application;

// ─── Constants ────────────────────────────────────────────────────────────────

const TARGET_WORLD_RADIUS: f32 = 1.0;
const CAMERA_FOV:          f32 = 75.0 * std::f32::consts::PI / 180.0;
const SNAP_THRESHOLD_DOT:  f32 = 0.996;
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
    mode:             AppMode,
    solved_timer:     f32,   // seconds since solve; transitions to menu after 5s
    /// Scene lighting config — passed to the pipeline each frame via GameFrameData.
    scene_lights:     LightConfig,
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
            model: None, entity: None, orbit: None,
            scramble_state: None, entropy_report: None,
            game_state: GameState::Playing,
            hints: HintSystem::new(),
            last_hint_output: None,
            elapsed_secs: 0.0, level_start: None,
            registry,
            client: PolyPizzaClient::new(),
            mode: AppMode::LevelSelect,
            solved_timer: 0.0,
            scene_lights: LightConfig::default(),
        }
    }

    // ── Level loading ──────────────────────────────────────────────────────

    fn load_model_and_start(&mut self, path: &str, level_id: &str, ctx: &mut SceneContext) {
        // Clear scene.
        if let Some(scene) = ctx.scene() { scene.clear(); }
        self.model  = None;
        self.entity = None;
        self.orbit  = None;
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

        let params = model.borrow().compute_puzzle_params(TARGET_WORLD_RADIUS, CAMERA_FOV);
        model.borrow_mut().scramble_along_axis(search.axis, params.min_disp, params.max_disp);

        if let Some(scene) = ctx.scene() {
            let entity = Rc::new(RefCell::new(SimpleEntity::new(model.clone())));

            let s = params.entity_scale;
            entity.borrow_mut().transform_mut().set_scale(Vector3::new(s, s, s));
            entity.borrow_mut().transform_mut().add_rotation_euler_world(0.0, 180.0, 0.0);
            entity.borrow_mut().transform_mut().set_position(Vector3::new(0.0, 0.0, 0.0));

            let entity_rot   = entity.borrow().transform().rotation;
            let solution_dir = (entity_rot * search.axis).normalize();

            self.scramble_state = Some(ScrambleState { solution_dir, params });

            scene.add_node(entity.clone());
            self.entity = Some(entity.clone());

            if let Some(camera) = ctx.camera() {
                let centroid  = entity.borrow().centroid();
                let dist      = params.orbit_distance;
                let (ctrl, sp) = make_scrambled_orbit(centroid, dist, solution_dir);
                camera.borrow_mut().transform.position = sp;
                self.orbit = Some(OrbitController::new(centroid, dist));
                camera.borrow_mut().set_controller(Some(Box::new(ctrl)));
            }
        }

        self.model        = Some(model);
        self.game_state   = GameState::Playing;
        self.level_start  = Some(Instant::now());
        self.elapsed_secs = 0.0;
        self.mode         = AppMode::InGame { level_id: level_id.to_string() };
        self.solved_timer = 0.0;
    }

    fn load_builtin_cactus(&mut self, ctx: &mut SceneContext) {
        self.load_model_and_start("models/Cactus/cactus.fbx", "cactus_builtin", ctx);
    }

    fn load_random_saved(&mut self, ctx: &mut SceneContext) {
        if let Some(meta) = self.registry.random_level() {
            let path = self.registry.model_path(meta);
            let id   = meta.id.clone();
            if path.exists() {
                self.load_model_and_start(path.to_string_lossy().as_ref(), &id, ctx);
                return;
            }
        }
        self.load_builtin_cactus(ctx);
    }

    fn fetch_online_level(&mut self) {
        if !self.client.is_explore_pending() {
            // Use a small random offset so we get variety, but stay near the
            // beginning where free CC-BY models are most plentiful
            use rand::Rng;
            let offset = rand::rng().random_range(0usize..50);
            self.client.fetch_explore_page(offset, 20);
            log::info!("[Formosaic] Fetching poly.pizza page at offset {}", offset);
        }
    }

    // ── Puzzle ─────────────────────────────────────────────────────────────

    fn trigger_solve(&mut self, ctx: &mut SceneContext) {
        if self.game_state != GameState::Playing { return; }
        let solution_dir = match &self.scramble_state { Some(s) => s.solution_dir, None => return };

        log::info!("[Formosaic] SOLVED! time={:.1}s hints={}", self.elapsed_secs, self.hints.hint_count());

        if let Some(camera) = ctx.camera() {
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
        }

        if let Some(start) = self.level_start {
            self.elapsed_secs = start.elapsed().as_secs_f32();
        }
    }

    fn finish_restore(&mut self, ctx: &mut SceneContext) {
        if let Some(camera) = ctx.camera() {
            let target = self.orbit.as_ref().map(|o| o.target)
                .unwrap_or_else(|| camera.borrow().transform.position);
            let dist = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
            let pos  = camera.borrow().transform.position;
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
                Ok(dl)  => self.on_download_complete(dl, ctx),
                Err(e)  => {
                    log::warn!("[Formosaic] Download failed: {} — loading cactus", e);
                    self.load_builtin_cactus(ctx);
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
        self.load_model_and_start(path.to_string_lossy().as_ref(), &meta.id, ctx);

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
        matches!(self.mode, AppMode::LevelSelect)
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
        self.load_builtin_cactus(ctx);
    }

    fn on_update(&mut self, delta_time: f32, ctx: &mut SceneContext) {
        self.poll_client(ctx);

        // Update hints once per frame and cache for the renderer.
        self.last_hint_output = if let (Some(sc), Some(camera)) =
            (&self.scramble_state, ctx.camera())
        {
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
                let snap = if let (Some(sc), Some(cam)) = (&self.scramble_state, ctx.camera()) {
                    cam.borrow().transform.forward().normalize()
                        .dot(sc.solution_dir.normalize()).abs() >= SNAP_THRESHOLD_DOT
                } else { false };
                if snap { do_solve = true; }
            }
            GameState::Restoring { elapsed, cam_start, cam_end } => {
                *elapsed += delta_time;
                let ev = *elapsed;
                let tri_t = if ev >= RESTORE_DURATION { do_restore_complete = true; 0.0 }
                            else { 1.0 - smoothstep(ev / RESTORE_DURATION) };
                if let Some(m) = &self.model { m.borrow().upload_lerp(tri_t); }
                let t = smoothstep((ev / RESTORE_DURATION).min(1.0));
                cam_pos    = Some(*cam_start + (*cam_end - *cam_start) * t);
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
        style.window_rounding    = 4.0;
        style.frame_rounding     = 3.0;
        style.popup_rounding     = 3.0;
        style.scrollbar_rounding = 3.0;
        style.grab_rounding      = 3.0;
        style.window_border_size = 1.0;
        style[imgui::StyleColor::WindowBg]         = [0.06, 0.07, 0.10, 0.0];
        style[imgui::StyleColor::TitleBg]          = [0.04, 0.05, 0.08, 1.0];
        style[imgui::StyleColor::TitleBgActive]    = [0.08, 0.09, 0.14, 1.0];
        style[imgui::StyleColor::Border]           = [0.12, 0.13, 0.21, 1.0];
        style[imgui::StyleColor::FrameBg]          = [0.10, 0.11, 0.18, 1.0];
        style[imgui::StyleColor::FrameBgHovered]   = [0.15, 0.17, 0.26, 1.0];
        style[imgui::StyleColor::Button]           = [0.19, 0.31, 0.75, 0.75];
        style[imgui::StyleColor::ButtonHovered]    = [0.25, 0.40, 0.90, 0.90];
        style[imgui::StyleColor::ButtonActive]     = [0.75, 0.19, 0.29, 1.0];
        style[imgui::StyleColor::Header]           = [0.19, 0.31, 0.75, 0.45];
        style[imgui::StyleColor::HeaderHovered]    = [0.19, 0.31, 0.75, 0.80];
        style[imgui::StyleColor::CheckMark]        = [0.18, 0.75, 0.50, 1.0];
        style[imgui::StyleColor::Text]             = [0.82, 0.85, 0.94, 1.0];
        style[imgui::StyleColor::TextDisabled]     = [0.35, 0.38, 0.50, 1.0];
        style[imgui::StyleColor::Separator]        = [0.12, 0.13, 0.21, 1.0];
        style[imgui::StyleColor::SeparatorHovered] = [0.40, 0.50, 0.80, 0.78];
        #[cfg(target_os = "android")]
        { style.touch_extra_padding = [8.0, 8.0]; }
    }

    fn build_ui(&mut self, ui: &imgui::Ui, w: f32, h: f32, ctx: &mut SceneContext) {
        use imgui::*;
        use crate::puzzle::entropy::difficulty_label;
        use crate::puzzle::hints::HintTier;
        // `use imgui::*` brings imgui::Key into scope, shadowing the engine Key.
        // Re-import the engine Key so on_event() calls compile correctly.
        use formosaic_engine::input::Key;

        let show_menu = self.is_in_menu();
        let is_touch  = cfg!(target_os = "android");

        fn diff_color(d: f32) -> [f32; 4] {
            match d {
                v if v < 0.25 => [0.18, 0.75, 0.50, 1.0],
                v if v < 0.50 => [0.75, 0.63, 0.18, 1.0],
                v if v < 0.75 => [0.75, 0.19, 0.29, 1.0],
                _             => [0.63, 0.18, 0.75, 1.0],
            }
        }
        fn diff_label_str(d: f32) -> &'static str {
            match d {
                v if v < 0.25 => "Easy",
                v if v < 0.50 => "Medium",
                v if v < 0.75 => "Hard",
                _             => "Expert",
            }
        }
        fn truncate(s: &str, max: usize) -> String {
            if s.len() > max { format!("{}..", &s[..max]) } else { s.to_string() }
        }

        // ── In-game HUD ───────────────────────────────────────────────────
        let hud_flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_INPUTS
            | WindowFlags::NO_SAVED_SETTINGS;

        ui.window("##hud")
            .flags(hud_flags)
            .position([w - 220.0, 10.0], Condition::Always)
            .size([210.0, 100.0], Condition::Always)
            .build(|| {
                if let Some(r) = self.entropy_report() {
                    ui.text_colored([0.8, 0.85, 0.95, 0.9],
                        format!("{:.1}s  |  {}", self.elapsed_secs(), difficulty_label(r.difficulty)));
                    let hints = self.hint_system().hint_count();
                    if hints > 0 { ui.text_colored([0.9, 0.6, 0.2, 0.8], format!("{} hint(s)", hints)); }
                }
                if self.is_solved()      { ui.text_colored([0.2, 0.9, 0.5, 1.0], "SOLVED!"); }
                if self.is_downloading() { ui.text_colored([0.4, 0.7, 1.0, 0.8], "Fetching..."); }
            });

        // ── Hint warmth indicator ─────────────────────────────────────────
        let hint_tier = self.hint_system().tier();
        if hint_tier != HintTier::None {
            ui.window("##hints")
                .flags(hud_flags)
                .position([10.0, 10.0], Condition::Always)
                .size([180.0, 60.0], Condition::Always)
                .build(|| {
                    let warmth = self.last_hint_output().map(|o| o.warmth).unwrap_or(0.5);
                    let cold = [0.2_f32, 0.4, 0.9, 1.0];
                    let hot  = [0.9_f32, 0.2, 0.1, 1.0];
                    let col  = [cold[0]+(hot[0]-cold[0])*warmth, cold[1]+(hot[1]-cold[1])*warmth,
                                cold[2]+(hot[2]-cold[2])*warmth, 1.0];
                    let label = if warmth > 0.8 { "HOT" } else if warmth > 0.5 { "WARM" } else { "COLD" };
                    ui.text_colored(col, format!(">> {} <<", label));
                    ui.text_colored([0.5, 0.5, 0.5, 0.7], format!("Hint: {:?}", hint_tier));
                });
        }

        // ── Android in-game touch buttons (bottom-right) ────────────────
        #[cfg(target_os = "android")]
        if !show_menu {
            let btn_h  = (h * 0.10).max(44.0);
            let btn_w  = (w * 0.14).max(80.0);
            let margin = w * 0.012;
            // Window spans both buttons side-by-side, pinned to bottom-right.
            let win_w  = btn_w * 2.0 + margin * 3.0;
            let win_h  = btn_h + margin * 2.0;
            let win_x  = w - win_w;
            let win_y  = h - win_h;

            let mut esc_clicked  = false;
            let mut hint_clicked = false;

            ui.window("##touch_btns")
                .flags(
                    WindowFlags::NO_DECORATION
                    | WindowFlags::NO_MOVE
                    | WindowFlags::NO_BACKGROUND
                    | WindowFlags::NO_SAVED_SETTINGS,
                )
                .position([win_x, win_y], Condition::Always)
                .size([win_w, win_h], Condition::Always)
                .build(|| {
                    ui.set_cursor_pos([margin, margin]);
                    hint_clicked = ui.button_with_size("HINT", [btn_w, btn_h]);
                    ui.same_line_with_spacing(0.0, margin);
                    esc_clicked  = ui.button_with_size("ESC",  [btn_w, btn_h]);
                });

            if hint_clicked { self.on_event(&Event::KeyDown { key: Key::H },      ctx); }
            if esc_clicked  { self.on_event(&Event::KeyDown { key: Key::Escape }, ctx); }
            return;
        }

        if !show_menu { return; }


        // ── Menu overlay ──────────────────────────────────────────────────
        let menu_flags = WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE
            | WindowFlags::NO_SAVED_SETTINGS | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_SCROLL_WITH_MOUSE;
        let pad = (w * 0.025).max(8.0);
        let levels: Vec<_> = self.saved_levels().to_vec();

        ui.window("##menu")
            .flags(menu_flags)
            .position([0.0, 0.0], Condition::Always)
            .size([w, h], Condition::Always)
            .build(|| {
                if is_touch {
                    ui.set_cursor_pos([pad, pad]);
                    ui.text_colored([0.8, 0.85, 0.95, 1.0], "FORMOSAIC");
                    ui.same_line();
                    ui.text_colored([0.35, 0.38, 0.50, 0.8], " v0.1");
                    let btn_w = w - pad * 2.0;
                    let btn_h = (h * 0.09).max(44.0);
                    let gap   = pad * 0.5;
                    ui.set_cursor_pos([pad, h * 0.12]);
                    if ui.button_with_size("[N] Fetch Online", [btn_w, btn_h]) {
                        self.on_event(&Event::KeyDown { key: Key::N }, ctx);
                    }
                    ui.set_cursor_pos([pad, h * 0.12 + btn_h + gap]);
                    if ui.button_with_size("[R] Random Saved", [btn_w, btn_h]) {
                        self.on_event(&Event::KeyDown { key: Key::R }, ctx);
                    }
                    if self.is_downloading() {
                        ui.set_cursor_pos([pad, h * 0.12 + (btn_h + gap) * 2.0]);
                        ui.text_colored([0.4, 0.7, 1.0, 1.0], "Fetching...");
                    }
                    let list_y = h * 0.12 + (btn_h + gap) * 2.0 + pad;
                    let list_h = h - list_y - pad;
                    let card_h = (h * 0.20).max(80.0);
                    let card_w = (w - pad * 3.0) * 0.5;
                    ui.set_cursor_pos([pad, list_y]);
                    ui.child_window("##levels").size([w - pad * 2.0, list_h]).border(false).build(|| {
                        if levels.is_empty() {
                            ui.spacing();
                            ui.text_colored([0.35, 0.38, 0.50, 1.0], "No saved levels yet.");
                            ui.text_colored([0.35, 0.38, 0.50, 0.8], "Use [N] to fetch from Poly Pizza.");
                        } else {
                            let mut col = 0usize;
                            for level in &levels {
                                ui.child_window(format!("##c_{}", level.id))
                                    .size([card_w, card_h]).border(true).build(|| {
                                        ui.text_colored([0.82, 0.85, 0.94, 1.0], truncate(&level.name, 12));
                                        ui.text_colored([0.35, 0.38, 0.50, 0.8], truncate(&level.author, 14));
                                        ui.text_colored(diff_color(level.difficulty), diff_label_str(level.difficulty));
                                        if let Some(t) = level.best_time_secs {
                                            ui.text_colored([0.35, 0.38, 0.50, 0.7], format!("{:.1}s", t));
                                        }
                                        let play_h = (card_h * 0.3).max(24.0);
                                        if ui.button_with_size("Play", [card_w - 8.0, play_h]) {
                                            self.on_event(&Event::KeyDown { key: Key::R }, ctx);
                                        }
                                    });
                                col += 1;
                                if col < 2 { ui.same_line(); } else { col = 0; }
                            }
                        }
                    });
                } else {
                    let sidebar_w = (w * 0.28).max(150.0);
                    let btn_size  = [sidebar_w - pad * 2.0, 30.0];
                    let card_h    = 90.0_f32;
                    ui.set_cursor_pos([pad, pad]);
                    ui.text_colored([0.8, 0.85, 0.95, 1.0], "FORMOSAIC");
                    ui.same_line();
                    ui.text_colored([0.35, 0.38, 0.50, 0.8], "  puzzle  v0.1");
                    ui.set_cursor_pos([pad, h * 0.42]);
                    if ui.button_with_size("[N] Fetch Online", btn_size) {
                        self.on_event(&Event::KeyDown { key: Key::N }, ctx);
                    }
                    ui.set_cursor_pos([pad, h * 0.42 + 38.0]);
                    if ui.button_with_size("[R] Random Saved", btn_size) {
                        self.on_event(&Event::KeyDown { key: Key::R }, ctx);
                    }
                    if self.is_downloading() {
                        ui.set_cursor_pos([pad, h * 0.42 + 78.0]);
                        ui.text_colored([0.4, 0.7, 1.0, 1.0], "Fetching...");
                    }
                    ui.set_cursor_pos([pad, h - 20.0]);
                    ui.text_colored([0.25, 0.27, 0.36, 0.7], "v0.1.0");
                    let content_x = sidebar_w + pad;
                    let content_w = w - content_x - pad;
                    let content_y = h * 0.08;
                    ui.set_cursor_pos([content_x, content_y]);
                    ui.child_window("##levels").size([content_w, h - content_y - 28.0]).border(false).build(|| {
                        if levels.is_empty() {
                            ui.spacing(); ui.spacing();
                            ui.text_colored([0.35, 0.38, 0.50, 1.0], "No saved levels yet.");
                            ui.spacing();
                            ui.text_colored([0.35, 0.38, 0.50, 0.8], "Press [N] to fetch a model from Poly Pizza.");
                        } else {
                            let avail = ui.content_region_avail()[0];
                            let cols  = 3usize;
                            let cw    = ((avail - 8.0 * cols as f32) / cols as f32).max(80.0);
                            let mut col = 0usize;
                            for level in &levels {
                                ui.child_window(format!("##c_{}", level.id))
                                    .size([cw, card_h]).border(true).build(|| {
                                        ui.text_colored([0.82, 0.85, 0.94, 1.0], truncate(&level.name, 14));
                                        ui.text_colored([0.35, 0.38, 0.50, 0.8], truncate(&level.author, 16));
                                        ui.text_colored(diff_color(level.difficulty), diff_label_str(level.difficulty));
                                        if let Some(t) = level.best_time_secs {
                                            ui.same_line();
                                            ui.text_colored([0.35, 0.38, 0.50, 0.7], format!("  {:.1}s", t));
                                        }
                                        if ui.button_with_size("Play", [cw - 12.0, 22.0]) {
                                            self.on_event(&Event::KeyDown { key: Key::R }, ctx);
                                        }
                                    });
                                col += 1;
                                if col < cols { ui.same_line(); } else { col = 0; }
                            }
                        }
                    });
                    ui.set_cursor_pos([content_x, h - 20.0]);
                    ui.text_colored([0.25, 0.27, 0.36, 0.7], "Models: Poly Pizza (poly.pizza) CC-BY");
                }
            });
    }
}
