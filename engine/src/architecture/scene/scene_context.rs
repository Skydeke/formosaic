use cgmath::Vector2;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    architecture::scene::node::scenegraph::Scenegraph,
    opengl::objects::clip_plane::ClipPlane,
    platform::PlatformInfo,
    rendering::{
        instances::camera::camera::Camera,
        render_output_data::RenderOutputData,
        render_state::LightConfig,
    },
};

pub struct SceneContext {
    clip_plane: ClipPlane,
    scene: Option<Scenegraph>,
    output_data: Option<RenderOutputData>,
    camera: Rc<RefCell<Camera>>,

    // ── Per-frame render state — written by the game layer, read by renderers ──
    /// Scene lighting — read by the deferred lighting pass.
    pub lights: LightConfig,
    /// When `false` the engine skips the geometry/lighting passes entirely.
    /// Defaults to `true`. The game sets this to `false` on full-screen menus.
    pub render_3d: bool,
    /// Platform information (touch vs desktop, UI scale, etc.).
    pub platform: PlatformInfo,
    /// Frame delta time in seconds.
    pub delta_time: f32,
    /// Opaque per-frame data written by the game layer, forwarded to
    /// game-side renderers.  The engine never inspects this.  Renderers
    /// downcast to their own concrete game struct.
    pub game_render_data: Option<Box<dyn Any>>,

    ui_actions: Vec<Box<dyn Any>>,
}

impl SceneContext {
    pub fn new() -> Self {
        let camera = Rc::new(RefCell::new(Camera::new()));
        Self {
            clip_plane: ClipPlane::NONE,
            scene: Some(Scenegraph::new()),
            output_data: None,
            camera,
            lights: LightConfig::default(),
            render_3d: true,
            platform: PlatformInfo::detect(),
            delta_time: 0.0,
            game_render_data: None,
            ui_actions: Vec::new(),
        }
    }

    pub fn get_clip_plane(&self) -> &ClipPlane {
        &self.clip_plane
    }
    pub fn clip_plane(&self) -> ClipPlane {
        self.clip_plane
    }

    pub fn set_clip_plane(&mut self, clip_plane: ClipPlane) {
        self.clip_plane = clip_plane;
    }

    pub fn scene(&self) -> Option<&Scenegraph> {
        self.scene.as_ref()
    }

    pub fn output_data(&self) -> Option<&RenderOutputData> {
        self.output_data.as_ref()
    }
    pub fn set_output_data(&mut self, output_data: RenderOutputData) {
        self.output_data = Some(output_data);
    }

    pub fn set_resolution(&mut self, size: Vector2<u32>) {
        self.camera.borrow_mut().set_resolution(size);
    }

    pub fn update(&mut self) {
        if let Some(scene) = &mut self.scene {
            scene.update();
        }
        self.camera.borrow_mut().update();
    }

    pub fn camera(&self) -> &RefCell<Camera> {
        &self.camera
    }

    pub fn get_camera(&self) -> &RefCell<Camera> {
        &self.camera
    }

    /// Queue a UI action emitted by a `UiNode` callback.
    pub fn push_ui_action<A: Any>(&mut self, action: A) {
        self.ui_actions.push(Box::new(action));
    }

    /// Drain queued UI actions of one concrete type.
    ///
    /// Actions that do not match type `A` are **re-queued** rather than
    /// silently dropped, so multiple action types can coexist safely.
    pub fn drain_ui_actions<A: Any>(&mut self) -> Vec<A> {
        let mut matched = Vec::new();
        let mut remaining = Vec::new();
        for action in std::mem::take(&mut self.ui_actions) {
            match action.downcast::<A>() {
                Ok(a) => matched.push(*a),
                Err(a) => remaining.push(a),
            }
        }
        self.ui_actions = remaining;
        matched
    }
}

impl Default for SceneContext {
    fn default() -> Self {
        Self::new()
    }
}
