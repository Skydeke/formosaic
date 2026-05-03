use crate::architecture::scene::scene_context::SceneContext;

/// Which pass a renderer participates in.
#[derive(PartialEq, Eq)]
pub enum RenderPass {
    Geometry,
    Late,
    Overlay,
}

/// Generic renderer interface — knows nothing about game state.
///
/// Per-frame state is pushed to concrete renderers via their own typed setter
/// methods *before* `Pipeline::draw` is called.  There is no generic prepare
/// hook here; that would force the abstraction to depend on game-specific types.
pub trait IRenderer {
    fn pass(&self) -> RenderPass {
        RenderPass::Geometry
    }
    fn render(&mut self, context: &SceneContext);
    fn any_processed(&self) -> bool;
    fn finish(&mut self);

    /// Called before rendering a model to set up material-specific GL state.
    fn prepare_material(
        &mut self,
        material: Option<&crate::architecture::models::material::Material>,
    ) {
        match material {
            Some(mat) => {
                unsafe {
                    if mat.cull_backface {
                        gl::Enable(gl::CULL_FACE);
                    } else {
                        gl::Disable(gl::CULL_FACE);
                    }
                    // Always enable depth testing for proper 3D rendering
                    gl::Enable(gl::DEPTH_TEST);

                    // Enable/disable blending based on alpha mode
                    match mat.alpha_mode {
                        crate::architecture::models::material::AlphaMode::Blend => {
                            gl::Enable(gl::BLEND);
                            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                        }
                        _ => {
                            gl::Disable(gl::BLEND);
                        }
                    }
                }
            }
            None => unsafe {
                gl::Disable(gl::CULL_FACE);
                gl::Disable(gl::BLEND);
                gl::Enable(gl::DEPTH_TEST);
            },
        }
    }

    /// Downcasting support — lets the Pipeline expose typed renderer accessors
    /// without storing separate handles.
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }
}
