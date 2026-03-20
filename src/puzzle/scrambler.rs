//! Scrambler: the puzzle mechanic.
//!
//! A random unit vector (the solution axis) is chosen. Every triangle is offset
//! along that axis by a random amount derived from the model's own geometry.
//! The camera starts at a random position far from that axis.
//!
//! All parameters (displacement range, orbit distance) are computed from the
//! model's bounding geometry so the puzzle scales correctly to any model size.

use cgmath::{InnerSpace, Vector3};
use rand::Rng;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

use crate::engine::architecture::models::simple_model::{PuzzleParams, SimpleModel};
use crate::engine::rendering::instances::camera::orbit_controller::OrbitController;

pub struct ScrambleState {
    /// The camera must look along this direction (or its opposite) to solve.
    pub solution_dir: Vector3<f32>,
    /// Puzzle parameters computed from this model's geometry.
    pub params: PuzzleParams,
}

/// Analyse the model, choose puzzle parameters, scramble, and return state.
/// `fov_radians` is the camera's vertical field of view.
pub fn scramble(
    model: &Rc<RefCell<SimpleModel>>,
    target_world_radius: f32,
    fov_radians: f32,
) -> ScrambleState {
    // Compute geometry-aware parameters from the model itself.
    let params = model.borrow().compute_puzzle_params(target_world_radius, fov_radians);

    log::info!(
        "[Scrambler] model_space_radius={:.1}  entity_scale={:.5}  \
         orbit_dist={:.2}  disp=[{:.1}, {:.1}]",
        params.model_space_radius, params.entity_scale,
        params.orbit_distance, params.min_disp, params.max_disp
    );

    let mut rng = rand::rng();
    let theta: f32 = rng.random_range(0.0..2.0 * PI);
    let phi: f32   = rng.random_range(-PI / 2.0..PI / 2.0);
    let solution_dir = Vector3::new(
        phi.cos() * theta.cos(),
        phi.sin(),
        phi.cos() * theta.sin(),
    ).normalize();

    log::info!(
        "[Scrambler] solution direction: ({:.3}, {:.3}, {:.3})",
        solution_dir.x, solution_dir.y, solution_dir.z
    );

    model.borrow_mut().scramble_along_axis(solution_dir, params.min_disp, params.max_disp);

    ScrambleState { solution_dir, params }
}

/// Build an OrbitController starting ≥60° from the solution axis.
/// Returns `(controller, camera_start_position)`.
pub fn make_scrambled_orbit(
    target: Vector3<f32>,
    distance: f32,
    solution_dir: Vector3<f32>,
) -> (OrbitController, Vector3<f32>) {
    let mut rng = rand::rng();

    let start_dir = loop {
        let theta: f32 = rng.random_range(0.0..2.0 * PI);
        let phi: f32   = rng.random_range(-PI / 2.0..PI / 2.0);
        let candidate  = Vector3::new(
            phi.cos() * theta.cos(),
            phi.sin(),
            phi.cos() * theta.sin(),
        ).normalize();

        if candidate.dot(solution_dir).abs() < (PI / 3.0_f32).cos() {
            break candidate;
        }
    };

    let camera_pos = target + start_dir * distance;
    let mut ctrl   = OrbitController::new(target, distance);
    ctrl.set_initial_position(camera_pos);
    (ctrl, camera_pos)
}
