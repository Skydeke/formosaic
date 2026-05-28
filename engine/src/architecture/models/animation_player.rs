use cgmath::Matrix4;

use crate::architecture::models::animation::{evaluate_clip, AnimationClip};
use crate::architecture::models::skeleton::Skeleton;
use cgmath::Zero;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopMode {
    Once,
    Loop,
}

/// Drives a single animation clip against a skeleton, producing bone matrices
/// for GPU skinning each frame.
pub struct AnimationPlayer {
    pub clip: Option<AnimationClip>,
    pub playing: bool,
    pub local_time_sec: f64,
    pub loop_mode: LoopMode,
    pub speed: f32,
    /// Per-mesh from-poses captured on play() for smooth crossfade.
    pub blend_from: Vec<Vec<Matrix4<f32>>>,
    /// How far through the blend we are (seconds).
    pub blend_elapsed: f32,
    /// Total duration of the blend in seconds.
    pub blend_duration: f32,
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            clip: None,
            playing: false,
            local_time_sec: 0.0,
            loop_mode: LoopMode::Loop,
            speed: 1.0,
            blend_from: Vec::new(),
            blend_elapsed: 0.0,
            blend_duration: 0.2,
        }
    }

    /// Start playing a clip from the beginning, crossfading from `current_matrices`.
    /// `current_matrices` should be the per-mesh bone matrices currently displayed.
    pub fn play(&mut self, clip: AnimationClip, current_matrices: &[Vec<Matrix4<f32>>]) {
        self.blend_from = current_matrices.to_vec();
        self.blend_elapsed = 0.0;
        self.clip = Some(clip);
        self.local_time_sec = 0.0;
        self.playing = true;
    }

    /// Stop playback and reset time.
    pub fn stop(&mut self) {
        self.playing = false;
        self.local_time_sec = 0.0;
        self.blend_from.clear();
    }

    /// Pause without resetting time.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume from where paused.
    pub fn resume(&mut self) {
        self.playing = true;
    }

    fn lerp_matrices(a: &[Matrix4<f32>], b: &[Matrix4<f32>], t: f32) -> Vec<Matrix4<f32>> {
        a.iter()
            .zip(b.iter())
            .map(|(fa, fb)| {
                let mut m = Matrix4::zero();
                for c in 0..4 {
                    for r in 0..4 {
                        m[c][r] = fa[c][r] + (fb[c][r] - fa[c][r]) * t;
                    }
                }
                m
            })
            .collect()
    }

    /// Advance the internal clock by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if self.blend_elapsed < self.blend_duration {
            self.blend_elapsed = (self.blend_elapsed + dt).min(self.blend_duration);
        }
        if !self.playing {
            return;
        }
        let clip = match &self.clip {
            Some(c) => c,
            None => return,
        };

        self.local_time_sec += dt as f64 * self.speed as f64;

        let duration = clip.duration_seconds();
        if duration > 0.0 {
            match self.loop_mode {
                LoopMode::Loop => {
                    self.local_time_sec = self.local_time_sec % duration;
                }
                LoopMode::Once => {
                    if self.local_time_sec >= duration {
                        self.local_time_sec = duration;
                        self.playing = false;
                    }
                }
            }
        }
    }

    /// Convert local time to ticks, evaluate the clip, and produce final skinning matrices.
    /// Returns the bone matrices ready for GPU upload.
    /// `mesh_index` selects which per-mesh offset matrix to use for each bone.
    pub fn evaluate(&self, skeleton: &mut Skeleton, mesh_index: usize) -> Vec<Matrix4<f32>> {
        let (clip, time_ticks) = match &self.clip {
            Some(c) => {
                let ticks = if c.ticks_per_second > 0.0 {
                    self.local_time_sec * c.ticks_per_second
                } else {
                    self.local_time_sec
                };
                (c, ticks)
            }
            None => {
                let bind_poses: Vec<Matrix4<f32>> = skeleton
                    .bones
                    .iter()
                    .map(|b| b.bind_local_transform)
                    .collect();
                return skeleton
                    .compute_final_matrices(&bind_poses, mesh_index)
                    .to_vec();
            }
        };

        let bone_names: Vec<String> = skeleton.bones.iter().map(|b| b.name.clone()).collect();
        let bind_local_transforms: Vec<Matrix4<f32>> = skeleton
            .bones
            .iter()
            .map(|b| b.bind_local_transform)
            .collect();
        let local_transforms = evaluate_clip(clip, time_ticks, &bone_names, &bind_local_transforms);
        let mut finals = skeleton
            .compute_final_matrices(&local_transforms, mesh_index)
            .to_vec();

        if self.blend_elapsed < self.blend_duration {
            if let Some(from) = self.blend_from.get(mesh_index) {
                if from.len() == finals.len() {
                    let t = self.blend_elapsed / self.blend_duration;
                    finals = Self::lerp_matrices(from, &finals, t);
                }
            }
        }

        finals
    }

    pub fn is_finished(&self) -> bool {
        match &self.clip {
            Some(c) => {
                self.loop_mode == LoopMode::Once && self.local_time_sec >= c.duration_seconds()
            }
            None => true,
        }
    }

    pub fn has_clip(&self) -> bool {
        self.clip.is_some()
    }
}
