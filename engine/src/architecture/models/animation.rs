use cgmath::{Matrix4, Quaternion, Vector3};

#[derive(Clone, Debug)]
pub struct PositionKey {
    pub time: f64,
    pub value: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub struct RotationKey {
    pub time: f64,
    pub value: Quaternion<f32>,
}

#[derive(Clone, Debug)]
pub struct ScalingKey {
    pub time: f64,
    pub value: Vector3<f32>,
}

#[derive(Clone, Debug)]
pub struct BoneChannel {
    pub bone_name: String,
    pub position_keys: Vec<PositionKey>,
    pub rotation_keys: Vec<RotationKey>,
    pub scaling_keys: Vec<ScalingKey>,
}

#[derive(Clone, Debug)]
pub struct AnimationClip {
    pub name: String,
    pub duration_ticks: f64,
    pub ticks_per_second: f64,
    pub channels: Vec<BoneChannel>,
}

impl AnimationClip {
    pub fn duration_seconds(&self) -> f64 {
        if self.ticks_per_second > 0.0 {
            self.duration_ticks / self.ticks_per_second
        } else {
            self.duration_ticks
        }
    }
}

// ── Keyframe interpolation ───────────────────────────────────────────────

/// Linearly interpolate between two position keys.
pub fn lerp_position(a: &PositionKey, b: &PositionKey, t: f64) -> Vector3<f32> {
    let tt = t as f32;
    a.value * (1.0 - tt) + b.value * tt
}

/// Spherical linear interpolation between two rotation keys.
pub fn slerp_rotation(a: &RotationKey, b: &RotationKey, t: f64) -> Quaternion<f32> {
    a.value.nlerp(b.value, t as f32)
}

/// Linearly interpolate between two scaling keys.
pub fn lerp_scaling(a: &ScalingKey, b: &ScalingKey, t: f64) -> Vector3<f32> {
    let tt = t as f32;
    a.value * (1.0 - tt) + b.value * tt
}

/// Find the two surrounding keyframes and compute `t` ∈ [0, 1] between them.
/// Returns `(prev_key, next_key, t)` where `t` is the interpolation factor.
/// If `keys` has 0 elements, returns `None`.
/// If `keys` has 1 element, returns `(key, key, 0.0)`.
fn find_keyframe_interval<T>(keys: &[T], time: f64, extract_time: impl Fn(&T) -> f64) -> Option<(usize, usize, f64)> {
    if keys.is_empty() {
        return None;
    }
    if keys.len() == 1 {
        return Some((0, 0, 0.0));
    }

    for i in 0..keys.len() - 1 {
        let t0 = extract_time(&keys[i]);
        let t1 = extract_time(&keys[i + 1]);
        if time >= t0 && time < t1 {
            let dt = t1 - t0;
            let t = if dt > 0.0 { (time - t0) / dt } else { 0.0 };
            return Some((i, i + 1, t));
        }
    }

    // Past the last keyframe — hold at last value
    let last = keys.len() - 1;
    Some((last, last, 0.0))
}

/// Evaluate a BoneChannel at a given time (in ticks), producing a position, rotation, and scale.
pub fn evaluate_channel(channel: &BoneChannel, time_ticks: f64) -> Option<(Vector3<f32>, Quaternion<f32>, Vector3<f32>)> {
    let pos = find_keyframe_interval(&channel.position_keys, time_ticks, |k| k.time)
        .map(|(i, j, t)| lerp_position(&channel.position_keys[i], &channel.position_keys[j], t));

    let rot = find_keyframe_interval(&channel.rotation_keys, time_ticks, |k| k.time)
        .map(|(i, j, t)| slerp_rotation(&channel.rotation_keys[i], &channel.rotation_keys[j], t));

    let scl = find_keyframe_interval(&channel.scaling_keys, time_ticks, |k| k.time)
        .map(|(i, j, t)| lerp_scaling(&channel.scaling_keys[i], &channel.scaling_keys[j], t));

    match (pos, rot, scl) {
        (Some(p), Some(r), Some(s)) => Some((p, r, s)),
        _ => None,
    }
}

/// Evaluate all channels of a clip and produce bone-local transforms.
/// Returns `Vec<Matrix4<f32>>` indexed the same as `Skeleton::bones` (matching by bone name).
/// Bones without animation data keep their bind-pose local transform.
pub fn evaluate_clip(
    clip: &AnimationClip,
    time_ticks: f64,
    bone_names: &[String],
    bind_local_transforms: &[Matrix4<f32>],
) -> Vec<Matrix4<f32>> {
    use crate::architecture::models::skeleton::Skeleton;

    assert_eq!(bone_names.len(), bind_local_transforms.len());
    let mut result = bind_local_transforms.to_vec();
    let mut matched = 0usize;

    for channel in &clip.channels {
        if let Some(bone_idx) = bone_names.iter().position(|n| n == &channel.bone_name) {
            if let Some((pos, rot, scl)) = evaluate_channel(channel, time_ticks) {
                result[bone_idx] = Skeleton::make_local_transform(pos, rot, scl);
                matched += 1;
            }
        }
    }

    if log::log_enabled!(log::Level::Debug) {
        log::debug!(
            "[Animation] evaluate_clip clip='{}' time_ticks={:.4} channels={} matched={} bones={}",
            clip.name,
            time_ticks,
            clip.channels.len(),
            matched,
            bone_names.len(),
        );
    }

    result
}
