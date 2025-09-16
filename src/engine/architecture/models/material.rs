use std::rc::Rc;

use cgmath::Vector4;

use crate::opengl::textures::texture::Texture;

pub struct Material {
    pub name: Option<String>,

    pub diffuse_color: Vector4<f32>,
    pub specular_color: Vector4<f32>,
    pub ambient_color: Vector4<f32>,
    pub emissive_color: Vector4<f32>,
    pub transparent_color: Vector4<f32>,

    pub diffuse_texture: Option<Rc<dyn Texture>>,
    pub use_diffuse_tex: bool,

    pub normal_texture: Option<Rc<dyn Texture>>,
    pub use_normal_tex: bool,

    pub cull_backface: bool,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: None,
            diffuse_color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            specular_color: Vector4::new(1.0, 1.0, 1.0, 1.0),
            ambient_color: Vector4::new(0.0, 0.0, 0.0, 1.0),
            emissive_color: Vector4::new(0.0, 0.0, 0.0, 1.0),
            transparent_color: Vector4::new(0.0, 0.0, 0.0, 0.0),

            diffuse_texture: None,
            use_diffuse_tex: false,

            normal_texture: None,
            use_normal_tex: false,

            cull_backface: true,
        }
    }
}

impl Material {
    /// Create a new empty material
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diffuse_color(mut self, color: Vector4<f32>) -> Self {
        self.diffuse_color = color;
        self
    }

    pub fn with_specular_color(mut self, color: Vector4<f32>) -> Self {
        self.specular_color = color;
        self
    }

    pub fn with_ambient_color(mut self, color: Vector4<f32>) -> Self {
        self.ambient_color = color;
        self
    }

    pub fn with_emissive_color(mut self, color: Vector4<f32>) -> Self {
        self.emissive_color = color;
        self
    }

    pub fn with_transparent_color(mut self, color: Vector4<f32>) -> Self {
        self.transparent_color = color;
        self
    }

    pub fn with_diffuse_texture(mut self, texture: Rc<dyn Texture>) -> Self {
        self.diffuse_texture = Some(texture);
        self.use_diffuse_tex = true;
        self
    }

    pub fn with_normal_texture(mut self, texture: Rc<dyn Texture>) -> Self {
        self.normal_texture = Some(texture);
        self.use_normal_tex = true;
        self
    }

    pub fn enable_backface_culling(mut self) -> Self {
        self.cull_backface = true;
        self
    }

    pub fn disable_backface_culling(mut self) -> Self {
        self.cull_backface = false;
        self
    }

    /// Configure GL state before rendering
    pub fn preconfigure(&self) {
        if self.cull_backface {
            unsafe {
                gl::Enable(gl::CULL_FACE);
            }
        } else {
            unsafe {
                gl::Disable(gl::CULL_FACE);
            }
        }
    }
}
