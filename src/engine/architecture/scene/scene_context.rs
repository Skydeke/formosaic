use cgmath::Vector2;
use std::cell::RefCell;
use std::rc::Rc;

use crate::{
    engine::{
        architecture::scene::node::scenegraph::Scenegraph,
        rendering::{instances::camera::camera::Camera, render_output_data::RenderOutputData},
    },
    opengl::objects::clip_plane::ClipPlane,
};

pub struct SceneContext {
    clip_plane: ClipPlane,
    scene: Option<Scenegraph>,
    output_data: Option<RenderOutputData>,
    camera: Rc<RefCell<Camera>>,
}

impl SceneContext {
    pub fn new() -> Self {
        let camera = Rc::new(RefCell::new(Camera::new()));
        Self {
            clip_plane: ClipPlane::NONE,
            scene: Some(Scenegraph::new()),
            output_data: None,
            camera,
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

    pub fn camera(&self) -> Option<Rc<RefCell<Camera>>> {
        Some(self.camera.clone())
    }

    pub fn get_camera(&self) -> Rc<RefCell<Camera>> {
        self.camera.clone()
    }
}

impl Default for SceneContext {
    fn default() -> Self {
        Self::new()
    }
}
