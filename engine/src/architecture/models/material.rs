use std::rc::Rc;

use cgmath::Vector4;

use crate::opengl::textures::texture::Texture;

/// Alpha rendering mode, matching glTF spec.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask(f32),  // alpha cutoff
    Blend,
}

impl Default for AlphaMode {
    fn default() -> Self { AlphaMode::Opaque }
}

/// Full PBR + legacy material properties loaded from assimp.
///
/// The flat low-poly shader uses `diffuse_color` / `diffuse_texture` as the
/// primary albedo source.  All other fields are stored so the material system
/// is complete, and they participate in lighting math when present.
#[derive(Clone)]
pub struct Material {
    pub name: Option<String>,

    // ── Colour scalars ─────────────────────────────────────────────────────
    pub diffuse_color:     Vector4<f32>,
    pub specular_color:    Vector4<f32>,
    pub ambient_color:     Vector4<f32>,
    pub emissive_color:    Vector4<f32>,
    pub transparent_color: Vector4<f32>,

    /// PBR metallic factor (0 = dielectric, 1 = metal).
    pub metallic_factor:   f32,
    /// PBR roughness factor.
    pub roughness_factor:  f32,

    // ── Texture maps ───────────────────────────────────────────────────────
    /// Albedo / base-color texture.
    pub diffuse_texture:           Option<Rc<dyn Texture>>,
    /// Tangent-space normal map.
    pub normal_texture:            Option<Rc<dyn Texture>>,
    /// glTF ORM: R=occlusion, G=roughness, B=metallic (combined in one tex).
    pub metallic_roughness_texture: Option<Rc<dyn Texture>>,
    /// Emissive glow texture.
    pub emissive_texture:          Option<Rc<dyn Texture>>,
    /// Separate ambient-occlusion texture (when not packed into ORM).
    pub occlusion_texture:         Option<Rc<dyn Texture>>,
    /// Specular/shininess texture (legacy Phong pipelines).
    pub specular_texture:          Option<Rc<dyn Texture>>,

    // ── Render state ───────────────────────────────────────────────────────
    pub cull_backface: bool,
    pub alpha_mode:    AlphaMode,
    /// Emissive strength multiplier (KHR_materials_emissive_strength).
    pub emissive_strength: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            name: None,
            diffuse_color:     Vector4::new(1.0, 1.0, 1.0, 1.0),
            specular_color:    Vector4::new(1.0, 1.0, 1.0, 1.0),
            ambient_color:     Vector4::new(0.0, 0.0, 0.0, 1.0),
            emissive_color:    Vector4::new(0.0, 0.0, 0.0, 1.0),
            transparent_color: Vector4::new(0.0, 0.0, 0.0, 0.0),
            metallic_factor:   0.0,
            roughness_factor:  0.5,

            diffuse_texture:            None,
            normal_texture:             None,
            metallic_roughness_texture: None,
            emissive_texture:           None,
            occlusion_texture:          None,
            specular_texture:           None,

            cull_backface:    true,
            alpha_mode:       AlphaMode::Opaque,
            emissive_strength: 1.0,
        }
    }
}

impl Material {
    pub fn new() -> Self { Self::default() }

    // ── Builder helpers ────────────────────────────────────────────────────

    pub fn with_diffuse_color(mut self, color: Vector4<f32>) -> Self {
        self.diffuse_color = color; self
    }
    pub fn with_specular_color(mut self, color: Vector4<f32>) -> Self {
        self.specular_color = color; self
    }
    pub fn with_ambient_color(mut self, color: Vector4<f32>) -> Self {
        self.ambient_color = color; self
    }
    pub fn with_emissive_color(mut self, color: Vector4<f32>) -> Self {
        self.emissive_color = color; self
    }
    pub fn with_transparent_color(mut self, color: Vector4<f32>) -> Self {
        self.transparent_color = color; self
    }
    pub fn with_diffuse_texture(mut self, texture: Rc<dyn Texture>) -> Self {
        self.diffuse_texture = Some(texture); self
    }
    pub fn with_normal_texture(mut self, texture: Rc<dyn Texture>) -> Self {
        self.normal_texture = Some(texture); self
    }
    pub fn enable_backface_culling(mut self) -> Self {
        self.cull_backface = true; self
    }
    pub fn disable_backface_culling(mut self) -> Self {
        self.cull_backface = false; self
    }

    /// Convenience: does this material have any emissive contribution?
    pub fn has_emissive(&self) -> bool {
        self.emissive_texture.is_some()
            || self.emissive_color.x > 0.001
            || self.emissive_color.y > 0.001
            || self.emissive_color.z > 0.001
    }

    /// Configure GL state before rendering this material.
    pub fn preconfigure(&self) {
        unsafe {
            if self.cull_backface {
                gl::Enable(gl::CULL_FACE);
            } else {
                gl::Disable(gl::CULL_FACE);
            }
            match self.alpha_mode {
                AlphaMode::Blend => {
                    gl::Enable(gl::BLEND);
                    gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                }
                _ => {
                    gl::Disable(gl::BLEND);
                }
            }
        }
    }
}
