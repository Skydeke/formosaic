use cgmath::{InnerSpace, Vector3};
use std::{cell::RefCell, rc::Rc};

use crate::{
    engine::{
        architecture::{
            models::{model_loader::ModelLoader, simple_model::SimpleModel},
            scene::{
                entity::simple_entity::SimpleEntity, node::node::NodeBehavior,
                scene_context::SceneContext,
            },
        },
        rendering::instances::camera::orbit_controller::OrbitController,
    },
    input::{Event, Key},
    puzzle::scrambler::{make_scrambled_orbit, scramble, ScrambleState},
};

pub trait Application {
    fn on_init(&mut self, context: &mut SceneContext);
    fn on_update(&mut self, delta_time: f32, context: &mut SceneContext);
    fn on_event(&mut self, event: &Event, context: &mut SceneContext);
}

/// The model's bounding sphere is scaled to this world-space radius.
/// All other parameters (orbit distance, displacement) derive from this.
const TARGET_WORLD_RADIUS: f32 = 1.0;

/// Camera vertical FOV in radians — must match Camera::new().
const CAMERA_FOV: f32 = 75.0 * std::f32::consts::PI / 180.0;

const SNAP_THRESHOLD_DOT: f32 = 0.996;

/// Duration of the restore + camera-align animation in seconds.
const RESTORE_DURATION: f32 = 1.8;

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GameState {
    Playing,
    Restoring {
        elapsed: f32,
        cam_start: Vector3<f32>,
        cam_end: Vector3<f32>,
    },
    Solved,
}

pub struct Formosaic {
    model: Option<Rc<RefCell<SimpleModel>>>,
    entity: Option<Rc<RefCell<SimpleEntity>>>,
    scramble_state: Option<ScrambleState>,
    state: GameState,
    orbit: Option<OrbitController>,
}

impl Formosaic {
    pub fn new() -> Self {
        Self {
            model: None,
            entity: None,
            scramble_state: None,
            state: GameState::Playing,
            orbit: None,
        }
    }

    fn trigger_solve(&mut self, context: &mut SceneContext) {
        if self.state != GameState::Playing {
            return;
        }

        let solution_dir = match &self.scramble_state {
            Some(s) => s.solution_dir,
            None => return,
        };

        log::info!("[Formosaic] SOLVED!");

        if let Some(camera) = context.camera() {
            let cam = camera.borrow();
            let target = self
                .orbit
                .as_ref()
                .map(|o| o.target)
                .unwrap_or(cam.transform.position);
            let distance = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);

            let fwd = cam.transform.forward().normalize();
            let dir = if fwd.dot(solution_dir) > 0.0 {
                solution_dir
            } else {
                -solution_dir
            };

            let cam_start = cam.transform.position;
            let cam_end = target - dir * distance;
            drop(cam);

            camera.borrow_mut().set_controller(None);
            self.state = GameState::Restoring {
                elapsed: 0.0,
                cam_start,
                cam_end,
            };
        }
    }

    /// Rescramble the current model and reset the camera to a new start position.
    fn trigger_rescramble(&mut self, context: &mut SceneContext) {
        // Only rescramble if we're playing or solved (not mid-animation).
        match self.state {
            GameState::Restoring { .. } => return,
            _ => {}
        }

        let model = match &self.model {
            Some(m) => m.clone(),
            None => return,
        };

        // Reset triangles to original before rescrambling.
        model.borrow().upload_lerp(0.0);

        let state = scramble(&model, TARGET_WORLD_RADIUS, CAMERA_FOV);

        // Re-derive world-space solution direction through entity rotation.
        let solution_dir = if let Some(entity) = &self.entity {
            let rot = entity.borrow().transform().rotation;
            (rot * state.solution_dir).normalize()
        } else {
            state.solution_dir
        };

        self.scramble_state = Some(ScrambleState {
            solution_dir,
            params: state.params,
        });

        self.state = GameState::Playing;

        // Reset camera to a new random start far from the new solution.
        if let Some(camera) = context.camera() {
            let target = self
                .orbit
                .as_ref()
                .map(|o| o.target)
                .unwrap_or(Vector3::new(0.0, 0.0, 0.0));
            let distance = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);

            let (ctrl, start_pos) = make_scrambled_orbit(target, distance, solution_dir);
            camera.borrow_mut().transform.position = start_pos;
            camera.borrow_mut().set_controller(Some(Box::new(ctrl)));
        }

        log::info!("[Formosaic] Rescrambled.");
    }
}

impl Default for Formosaic {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for Formosaic {
    fn on_init(&mut self, context: &mut SceneContext) {
        log::info!("Initializing scene…");

        let cactus_model: Rc<RefCell<SimpleModel>> = ModelLoader::load("models/Cactus/cactus.fbx");

        // Analyse geometry → choose scale, orbit distance, displacement range.
        let state = scramble(&cactus_model, TARGET_WORLD_RADIUS, CAMERA_FOV);
        let params = state.params;
        self.model = Some(cactus_model.clone());

        log::info!(
            "[Formosaic] entity_scale={:.5}  orbit_dist={:.3}",
            params.entity_scale,
            params.orbit_distance
        );

        if let Some(scene) = context.scene() {
            let entity = Rc::new(RefCell::new(SimpleEntity::new(cactus_model.clone())));

            // Apply geometry-derived scale so the model always fills the view.
            let s = params.entity_scale;
            entity
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(s, s, s));
            entity
                .borrow_mut()
                .transform_mut()
                .add_rotation_euler_world(0.0, 180.0, 0.0);
            // Centre the model at the world origin (centroid becomes orbit target).
            entity
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(0.0, 0.0, 0.0));

            // Convert model-space scramble axis → world space via entity rotation.
            let entity_rot = entity.borrow().transform().rotation;
            let solution_dir = (entity_rot * state.solution_dir).normalize();

            log::info!(
                "[Formosaic] solution direction (world): ({:.3}, {:.3}, {:.3})",
                solution_dir.x,
                solution_dir.y,
                solution_dir.z
            );

            self.scramble_state = Some(ScrambleState {
                solution_dir,
                params,
            });

            scene.add_node(entity.clone());
            self.entity = Some(entity.clone());

            if let Some(camera) = context.camera() {
                // Use geometry-derived orbit distance.
                let centroid = entity.borrow().centroid();
                let orbit_distance = params.orbit_distance;

                let (ctrl, start_pos) =
                    make_scrambled_orbit(centroid, orbit_distance, solution_dir);

                camera.borrow_mut().transform.position = start_pos;
                self.orbit = Some(OrbitController::new(centroid, orbit_distance));
                camera.borrow_mut().set_controller(Some(Box::new(ctrl)));
            }
        }
    }

    fn on_update(&mut self, delta_time: f32, context: &mut SceneContext) {
        let mut do_solve = false;
        let mut do_restore_complete = false;
        let mut cam_pos_this_frame: Option<Vector3<f32>> = None;
        let mut cam_look_target: Option<Vector3<f32>> = None;

        match &mut self.state {
            GameState::Playing => {
                let should_snap =
                    if let (Some(sc), Some(camera)) = (&self.scramble_state, context.camera()) {
                        let fwd = camera.borrow().transform.forward().normalize();
                        fwd.dot(sc.solution_dir.normalize()).abs() >= SNAP_THRESHOLD_DOT
                    } else {
                        false
                    };
                if should_snap {
                    do_solve = true;
                }
            }

            GameState::Restoring {
                elapsed,
                cam_start,
                cam_end,
            } => {
                *elapsed += delta_time;
                let ev = *elapsed;

                let tri_t = if ev >= RESTORE_DURATION {
                    do_restore_complete = true;
                    0.0
                } else {
                    1.0 - smoothstep(ev / RESTORE_DURATION)
                };

                if let Some(model) = &self.model {
                    model.borrow().upload_lerp(tri_t);
                }

                let cam_t = smoothstep((ev / RESTORE_DURATION).min(1.0));
                let start = *cam_start;
                let end = *cam_end;
                cam_pos_this_frame = Some(start + (end - start) * cam_t);
                cam_look_target = self.orbit.as_ref().map(|o| o.target);
            }

            GameState::Solved => {}
        }

        if let (Some(pos), Some(look_at)) = (cam_pos_this_frame, cam_look_target) {
            if let Some(camera) = context.camera() {
                let mut cam = camera.borrow_mut();
                cam.transform.position = pos;
                cam.transform.look_at(look_at, Vector3::unit_y());
            }
        }

        if do_solve {
            self.trigger_solve(context);
        }
        if do_restore_complete {
            self.finish_restore(context);
        }
    }

    fn on_event(&mut self, event: &Event, context: &mut SceneContext) {
        match event {
            Event::KeyDown { key: Key::K } => {
                log::info!("[Formosaic] Cheat — solving.");
                self.trigger_solve(context);
            }
            Event::KeyDown { key: Key::L } => {
                log::info!("[Formosaic] Rescrambling.");
                self.trigger_rescramble(context);
            }
            _ => {
                if let Some(camera) = context.camera() {
                    let width = camera.borrow().resolution.x as f32;
                    let height = camera.borrow().resolution.y as f32;
                    camera
                        .borrow_mut()
                        .controller
                        .handle_event(event, width, height);
                }
            }
        }
    }
}

impl Formosaic {
    fn finish_restore(&mut self, context: &mut SceneContext) {
        if let Some(camera) = context.camera() {
            let target = self
                .orbit
                .as_ref()
                .map(|o| o.target)
                .unwrap_or_else(|| camera.borrow().transform.position);
            let distance = self.orbit.as_ref().map(|o| o.distance).unwrap_or(3.0);
            let pos = camera.borrow().transform.position;

            let mut post_orbit = OrbitController::new(target, distance);
            post_orbit.set_initial_position(pos);
            camera
                .borrow_mut()
                .set_controller(Some(Box::new(post_orbit)));
        }
        log::info!("[Formosaic] Restore complete — orbit re-enabled.");
        self.state = GameState::Solved;
    }
}
