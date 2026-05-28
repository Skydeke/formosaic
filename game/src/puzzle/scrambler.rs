//! Scrambler: the puzzle mechanic.
//!
//! A random unit vector (the solution axis) is chosen. Every triangle is offset
//! along that axis by a random amount derived from the model's own geometry.
//! The camera starts at a random position far from that axis.
//!
//! All parameters (displacement range, orbit distance) are computed from the
//! model's bounding geometry so the puzzle scales correctly to any model size.

use cgmath::{InnerSpace, Matrix4, Vector3};
use rand::Rng;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::architecture::models::simple_model::SimpleModel;
use formosaic_engine::rendering::instances::camera::orbit_controller::OrbitController;

use super::puzzle_params::PuzzleParams;
use super::scramble_math::compute_scramble_offsets;

pub struct ScrambleState {
    /// The camera must look along this direction (or its opposite) to solve.
    pub solution_dir: Vector3<f32>,
    /// Puzzle parameters computed from this model's geometry.
    pub params: PuzzleParams,
    /// Original vertex positions per mesh (before displacement).
    pub original_positions: Vec<Vec<f32>>,
    /// Per-vertex scramble offsets per mesh.
    pub scramble_offsets: Vec<Vec<f32>>,
}

/// Analyse the model, choose puzzle parameters, scramble, and return state.
/// `fov_radians` is the camera's vertical field of view.
pub fn scramble(
    model: &Rc<RefCell<SimpleModel>>,
    target_world_radius: f32,
    fov_radians: f32,
) -> ScrambleState {
    let params = PuzzleParams::from_model(&model.borrow(), target_world_radius, fov_radians);

    log::info!(
        "[Scrambler] model_space_radius={:.1}  entity_scale={:.5}  \
         orbit_dist={:.2}  disp=[{:.1}, {:.1}]",
        params.model_space_radius,
        params.entity_scale,
        params.orbit_distance,
        params.min_disp,
        params.max_disp
    );

    const MAX_ELEV: f32 = 55.0 * PI / 180.0;
    let mut rng = rand::rng();
    let theta: f32 = rng.random_range(0.0..2.0 * PI);
    let phi: f32 = rng.random_range(-MAX_ELEV..MAX_ELEV);
    let solution_dir =
        Vector3::new(phi.cos() * theta.cos(), phi.sin(), phi.cos() * theta.sin()).normalize();

    log::info!(
        "[Scrambler] solution direction: ({:.3}, {:.3}, {:.3})",
        solution_dir.x,
        solution_dir.y,
        solution_dir.z
    );

    let original_positions: Vec<Vec<f32>> = model
        .borrow()
        .get_meshes()
        .iter()
        .map(|m| m.positions().to_vec())
        .collect();
    let scramble_offsets = compute_model_offsets(
        &model.borrow(),
        solution_dir,
        params.min_disp,
        params.max_disp,
    );

    for (mesh_idx, offsets) in scramble_offsets.iter().enumerate() {
        let orig = &original_positions[mesh_idx];
        let displaced: Vec<f32> = orig
            .iter()
            .zip(offsets.iter())
            .map(|(p, o)| p + o)
            .collect();
        model.borrow_mut().upload_mesh_positions(mesh_idx, displaced);
    }

    ScrambleState {
        solution_dir,
        params,
        original_positions,
        scramble_offsets,
    }
}

/// Compute per-mesh scramble offsets for a model without touching GPU state.
/// Apply displacement to a model: positions = original + offset * t for each vertex.
/// `t = 0.0` = solved (original positions), `t = 1.0` = fully scrambled.
pub fn apply_displacement(
    model: &Rc<RefCell<SimpleModel>>,
    state: &ScrambleState,
    t: f32,
) {
    for (mesh_idx, offsets) in state.scramble_offsets.iter().enumerate() {
        let orig = &state.original_positions[mesh_idx];
        let displaced: Vec<f32> = orig
            .iter()
            .zip(offsets.iter())
            .map(|(p, o)| p + o * t)
            .collect();
        model.borrow_mut().upload_mesh_positions(mesh_idx, displaced);
    }
}

pub fn compute_model_offsets(
    model: &SimpleModel,
    axis: Vector3<f32>,
    min_disp: f32,
    max_disp: f32,
) -> Vec<Vec<f32>> {
    let mut per_mesh = Vec::new();
    for (mesh_idx, mesh) in model.get_meshes().iter().enumerate() {
        let transform = model
            .mesh_transform(mesh_idx)
            .unwrap_or_else(|| Matrix4::from_scale(1.0));
        let offsets =
            compute_scramble_offsets(mesh.positions().len(), axis, min_disp, max_disp, transform);
        per_mesh.push(offsets);
    }
    per_mesh
}

/// Build an OrbitController starting ≥60° from the solution axis.
/// Returns `(controller, camera_start_position)`.
pub fn make_scrambled_orbit(
    target: Vector3<f32>,
    distance: f32,
    solution_dir: Vector3<f32>,
) -> (OrbitController, Vector3<f32>) {
    let mut rng = rand::rng();

    // Camera start: sample from a comfortable elevation band (±70°) and at
    // least 60° away from the solution direction so the puzzle is non-trivial.
    const CAM_MAX_ELEV: f32 = 70.0 * PI / 180.0;
    let start_dir = loop {
        let theta: f32 = rng.random_range(0.0..2.0 * PI);
        let phi: f32 = rng.random_range(-CAM_MAX_ELEV..CAM_MAX_ELEV);
        let candidate =
            Vector3::new(phi.cos() * theta.cos(), phi.sin(), phi.cos() * theta.sin()).normalize();

        // Must be ≥60° from solution (neither pole of the solution axis).
        if candidate.dot(solution_dir).abs() < (PI / 3.0_f32).cos() {
            break candidate;
        }
    };

    let camera_pos = target + start_dir * distance;
    let mut ctrl = OrbitController::new(target, distance);
    ctrl.set_initial_position(camera_pos);
    (ctrl, camera_pos)
}
