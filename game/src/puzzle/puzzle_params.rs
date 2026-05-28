use cgmath::{InnerSpace, Matrix4, Vector3, Vector4};

use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::architecture::models::simple_model::SimpleModel;

#[derive(Debug, Clone, Copy)]
pub struct PuzzleParams {
    pub entity_scale: f32,
    pub orbit_distance: f32,
    pub min_disp: f32,
    pub max_disp: f32,
    pub model_space_radius: f32,
}

impl PuzzleParams {
    pub fn from_model(model: &SimpleModel, target_world_radius: f32, fov_radians: f32) -> Self {
        let mesh_positions: Vec<&[f32]> =
            model.get_meshes().iter().map(|m| m.positions()).collect();
        let mesh_transforms: Vec<Matrix4<f32>> = (0..mesh_positions.len())
            .map(|i| {
                model
                    .mesh_transform(i)
                    .unwrap_or_else(|| Matrix4::from_scale(1.0))
            })
            .collect();
        Self::from_raw_positions(
            &mesh_positions,
            &mesh_transforms,
            target_world_radius,
            fov_radians,
        )
    }

    pub fn from_raw_positions(
        mesh_positions: &[&[f32]],
        mesh_transforms: &[Matrix4<f32>],
        target_world_radius: f32,
        fov_radians: f32,
    ) -> Self {
        let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for (mesh_idx, pos) in mesh_positions.iter().enumerate() {
            let mesh_transform = mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let mut i = 0;
            while i + 2 < pos.len() {
                let p = mesh_transform * Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);
                let (x, y, z) = (p.x, p.y, p.z);
                if x < min.x {
                    min.x = x;
                }
                if y < min.y {
                    min.y = y;
                }
                if z < min.z {
                    min.z = z;
                }
                if x > max.x {
                    max.x = x;
                }
                if y > max.y {
                    max.y = y;
                }
                if z > max.z {
                    max.z = z;
                }
                i += 3;
            }
        }

        if min.x == f32::INFINITY {
            return Self::default_for(target_world_radius);
        }

        let extent = max - min;
        let model_radius = (extent.magnitude() * 0.5).max(0.001);
        let entity_scale = target_world_radius / model_radius;
        let half_fov = fov_radians * 0.5;
        let orbit_distance = target_world_radius / (half_fov.tan() * 0.65);

        Self {
            entity_scale,
            orbit_distance,
            min_disp: model_radius * 0.02,
            max_disp: model_radius * 0.12,
            model_space_radius: model_radius,
        }
    }

    pub fn default_for(target_world_radius: f32) -> Self {
        Self {
            entity_scale: 0.005,
            orbit_distance: target_world_radius * 3.0,
            min_disp: 3.0,
            max_disp: 15.0,
            model_space_radius: 1.0,
        }
    }
}
