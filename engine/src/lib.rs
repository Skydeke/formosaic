//! Formosaic Engine — generic rendering engine.
//!
//! Zero game-specific knowledge. Depends only on OpenGL ES bindings, cgmath,
//! and imgui (for the renderer abstraction). Nothing here imports from
//! `formosaic-game`.
//!
//! # Module structure
//!
//! - `opengl`        — Raw OpenGL ES 3.1 wrappers (VAO, VBO, FBO, shaders, textures)
//! - `architecture`  — Scene graph, models, entities, transforms
//! - `rendering`     — Pipeline, deferred renderer, camera, IRenderer abstraction

pub mod app;
pub mod architecture;
pub mod input;
pub mod opengl;
pub mod rendering;
