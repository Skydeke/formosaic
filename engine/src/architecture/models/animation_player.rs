use cgmath::{Matrix4, SquareMatrix};

use crate::architecture::models::animation::{evaluate_clip, AnimationClip};
use crate::architecture::models::skeleton::Skeleton;

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
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            clip: None,
            playing: false,
            local_time_sec: 0.0,
            loop_mode: LoopMode::Loop,
            speed: 1.0,
        }
    }

    /// Start playing a clip from the beginning.
    pub fn play(&mut self, clip: AnimationClip) {
        log::debug!(
            "[AnimationPlayer] play name='{}' duration_ticks={:.3} tps={:.3} channels={}",
            clip.name,
            clip.duration_ticks,
            clip.ticks_per_second,
            clip.channels.len(),
        );
        self.clip = Some(clip);
        self.local_time_sec = 0.0;
        self.playing = true;
    }

    /// Stop playback and reset time.
    pub fn stop(&mut self) {
        self.playing = false;
        self.local_time_sec = 0.0;
    }

    /// Pause without resetting time.
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Resume from where paused.
    pub fn resume(&mut self) {
        self.playing = true;
    }

    /// Advance the internal clock by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        if !self.playing {
            return;
        }
        let clip = match &self.clip {
            Some(c) => c,
            None => return,
        };

        self.local_time_sec += dt as f64 * self.speed as f64;
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "[AnimationPlayer] update dt={:.4} local_time_sec={:.4} clip='{}'",
                dt,
                self.local_time_sec,
                clip.name,
            );
        }

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
    pub fn evaluate(&self, skeleton: &mut Skeleton) -> Vec<Matrix4<f32>> {
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
                // No clip — return identity matrices for all bones
                let count = skeleton.bone_count();
                return vec![Matrix4::identity(); count];
            }
        };

        let bone_names: Vec<String> = skeleton.bones.iter().map(|b| b.name.clone()).collect();
        let bind_local_transforms: Vec<Matrix4<f32>> = skeleton
            .bones
            .iter()
            .map(|b| b.bind_local_transform)
            .collect();
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "[AnimationPlayer] evaluate clip='{}' time_ticks={:.4} bones={} channels={}",
                clip.name,
                time_ticks,
                bone_names.len(),
                clip.channels.len(),
            );
            for probe in ["Hips", "Pelvis", "Spine", "LeftArm", "RightArm", "CharacterArmature"] {
                if let Some(idx) = bone_names.iter().position(|n| n == probe) {
                    log::debug!(
                        "[AnimationPlayer] probe bone='{}' idx={} bind_local={:?}",
                        probe,
                        idx,
                        bind_local_transforms.get(idx),
                    );
                }
            }
        }
        let local_transforms = evaluate_clip(clip, time_ticks, &bone_names, &bind_local_transforms);
        let finals = skeleton.compute_final_matrices(&local_transforms).to_vec();
        if log::log_enabled!(log::Level::Debug) {
            for probe in ["Hips", "Pelvis", "Spine", "LeftArm", "RightArm", "CharacterArmature"] {
                if let Some(idx) = bone_names.iter().position(|n| n == probe) {
                    log::debug!(
                        "[AnimationPlayer] probe bone='{}' idx={} local={:?} final={:?}",
                        probe,
                        idx,
                        local_transforms.get(idx),
                        finals.get(idx),
                    );
                }
            }
        }
        finals
    }

    pub fn is_finished(&self) -> bool {
        match &self.clip {
            Some(c) => self.loop_mode == LoopMode::Once && self.local_time_sec >= c.duration_seconds(),
            None => true,
        }
    }

    pub fn has_clip(&self) -> bool {
        self.clip.is_some()
    }
}
