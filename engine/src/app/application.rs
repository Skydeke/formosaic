//! Application trait — the contract between the engine host and game logic.
//!
//! Any struct implementing `Application` can be driven by `GameEngine`.
//! The engine knows nothing about game rules; the game knows nothing about
//! winit or glutin.

use crate::architecture::scene::scene_context::SceneContext;
use crate::input::Event;

pub trait Application {
    // ── Lifecycle ─────────────────────────────────────────────────────────
    fn on_init(&mut self, context: &mut SceneContext);
    fn on_update(&mut self, delta_time: f32, context: &mut SceneContext);
    fn on_event(&mut self, event: &Event, context: &mut SceneContext);

    // ── Render bridge ─────────────────────────────────────────────────────
    /// Write all per-frame render state into `SceneContext` before
    /// `pipeline.draw()` is called.  The only place game state flows into
    /// the renderer.
    fn populate_scene_context(&mut self, ctx: &mut SceneContext, delta_time: f32);

    // ── Renderer registration ─────────────────────────────────────────────
    /// Register game-specific `IRenderer` instances onto the pipeline.
    /// Called once by `GameEngine` after GL is initialised.
    fn register_renderers(&mut self, pipeline: &mut crate::rendering::pipeline::Pipeline);

    // ── imgui ─────────────────────────────────────────────────────────────
    /// Configure the imgui context (theme, fonts, DPI) once at startup.
    fn configure_imgui(&self, imgui: &mut imgui::Context, scale: f32);

    /// Build the imgui UI for this frame.  Called inside the imgui frame.
    fn build_ui(&mut self, ui: &imgui::Ui, w: f32, h: f32, ctx: &mut SceneContext);

    // ── Platform hints ────────────────────────────────────────────────────
    /// The window title shown in the OS title bar.
    fn title(&self) -> &str { "App" }
}
