/// Tests for the LLM-added animation system.
///
/// Coverage:
///   • `find_keyframe_interval` — boundary conditions, single-key, past-last
///   • `lerp_position` / `lerp_scaling` — linear interpolation at t=0,½,1
///   • `slerp_rotation` — quaternion interpolation at t=0,½,1
///   • `evaluate_channel` — empty / single / multi-key channels
///   • `evaluate_clip` — bind-pose fallback, channel match, multi-bone
///   • `AnimationPlayer` state machine — play, pause, resume, stop, loop,
///     once-mode finish, speed scaling
///   • `Skeleton::make_local_transform` — composition order
///   • `Skeleton::compute_final_matrices` — flat list, parent-child chain,
///     depth-order invariant
use cgmath::{InnerSpace, Matrix4, Quaternion, SquareMatrix, Vector3, Vector4, Zero};
use formosaic_engine::architecture::models::animation::{
    evaluate_channel, evaluate_clip, lerp_position, lerp_scaling, slerp_rotation, AnimationClip,
    BoneChannel, PositionKey, RotationKey, ScalingKey,
};
use formosaic_engine::architecture::models::animation_player::{AnimationPlayer, LoopMode};
use formosaic_engine::architecture::models::skeleton::{BoneData, Skeleton};

// ── helpers ─────────────────────────────────────────────────────────────────

fn pos_key(t: f64, x: f32, y: f32, z: f32) -> PositionKey {
    PositionKey {
        time: t,
        value: Vector3::new(x, y, z),
    }
}

fn rot_key(t: f64, w: f32, x: f32, y: f32, z: f32) -> RotationKey {
    let q = Quaternion::new(w, x, y, z);
    // Normalise so tests don't depend on caller precision.
    let len = (q.v.magnitude2() + q.s * q.s).sqrt();
    RotationKey {
        time: t,
        value: Quaternion::new(q.s / len, q.v.x / len, q.v.y / len, q.v.z / len),
    }
}

fn scl_key(t: f64, x: f32, y: f32, z: f32) -> ScalingKey {
    ScalingKey {
        time: t,
        value: Vector3::new(x, y, z),
    }
}

fn identity_rot() -> Quaternion<f32> {
    Quaternion::new(1.0, 0.0, 0.0, 0.0)
}

fn unit_scale() -> Vector3<f32> {
    Vector3::new(1.0, 1.0, 1.0)
}

fn bone(name: &str, parent: Option<usize>) -> BoneData {
    BoneData {
        name: name.to_string(),
        bind_local_transform: Matrix4::identity(),
        offset_matrix: Matrix4::identity(),
        parent_index: parent,
    }
}

fn bone_with_local(name: &str, parent: Option<usize>, local: Matrix4<f32>) -> BoneData {
    BoneData {
        name: name.to_string(),
        bind_local_transform: local,
        offset_matrix: Matrix4::identity(),
        parent_index: parent,
    }
}

fn simple_channel(name: &str) -> BoneChannel {
    BoneChannel {
        bone_name: name.to_string(),
        position_keys: vec![pos_key(0.0, 0.0, 0.0, 0.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0)],
    }
}

fn simple_clip(name: &str, channels: Vec<BoneChannel>) -> AnimationClip {
    AnimationClip {
        name: name.to_string(),
        duration_ticks: 100.0,
        ticks_per_second: 25.0,
        channels,
    }
}

fn approx_vec3(a: Vector3<f32>, b: Vector3<f32>) -> bool {
    (a - b).magnitude() < 1e-4
}

fn approx_mat4_identity(m: Matrix4<f32>) -> bool {
    let id = Matrix4::<f32>::identity();
    for c in 0..4 {
        for r in 0..4 {
            if (m[c][r] - id[c][r]).abs() > 1e-4 {
                return false;
            }
        }
    }
    true
}

// ── lerp_position ────────────────────────────────────────────────────────────

#[test]
fn lerp_position_t0_returns_a() {
    let a = pos_key(0.0, 0.0, 0.0, 0.0);
    let b = pos_key(1.0, 10.0, 20.0, 30.0);
    let r = lerp_position(&a, &b, 0.0);
    assert!(approx_vec3(r, Vector3::new(0.0, 0.0, 0.0)));
}

#[test]
fn lerp_position_t1_returns_b() {
    let a = pos_key(0.0, 0.0, 0.0, 0.0);
    let b = pos_key(1.0, 10.0, 20.0, 30.0);
    let r = lerp_position(&a, &b, 1.0);
    assert!(approx_vec3(r, Vector3::new(10.0, 20.0, 30.0)));
}

#[test]
fn lerp_position_t_half_is_midpoint() {
    let a = pos_key(0.0, 0.0, 0.0, 0.0);
    let b = pos_key(1.0, 4.0, 8.0, 12.0);
    let r = lerp_position(&a, &b, 0.5);
    assert!(approx_vec3(r, Vector3::new(2.0, 4.0, 6.0)));
}

// ── lerp_scaling ─────────────────────────────────────────────────────────────

#[test]
fn lerp_scaling_t0_returns_a() {
    let a = scl_key(0.0, 1.0, 1.0, 1.0);
    let b = scl_key(1.0, 2.0, 3.0, 4.0);
    let r = lerp_scaling(&a, &b, 0.0);
    assert!(approx_vec3(r, Vector3::new(1.0, 1.0, 1.0)));
}

#[test]
fn lerp_scaling_t1_returns_b() {
    let a = scl_key(0.0, 1.0, 1.0, 1.0);
    let b = scl_key(1.0, 2.0, 3.0, 4.0);
    let r = lerp_scaling(&a, &b, 1.0);
    assert!(approx_vec3(r, Vector3::new(2.0, 3.0, 4.0)));
}

// ── slerp_rotation ───────────────────────────────────────────────────────────

#[test]
fn slerp_rotation_t0_returns_a() {
    let a = rot_key(0.0, 1.0, 0.0, 0.0, 0.0); // identity
    let b = rot_key(1.0, 0.0, 1.0, 0.0, 0.0); // 180° around X
    let r = slerp_rotation(&a, &b, 0.0);
    assert!(
        (r.s - 1.0).abs() < 1e-4,
        "expected identity quaternion at t=0"
    );
}

#[test]
fn slerp_rotation_t1_approaches_b() {
    let a = rot_key(0.0, 1.0, 0.0, 0.0, 0.0);
    let b = rot_key(1.0, 0.0, 1.0, 0.0, 0.0);
    let r = slerp_rotation(&a, &b, 1.0);
    // nlerp at t=1 should be very close to b
    assert!(
        r.v.magnitude() > 0.9,
        "expected unit-X quaternion at t=1, got {:?}",
        r
    );
}

#[test]
fn slerp_rotation_same_quaternion_is_stable() {
    let a = rot_key(0.0, 1.0, 0.0, 0.0, 0.0);
    let b = rot_key(1.0, 1.0, 0.0, 0.0, 0.0);
    let r = slerp_rotation(&a, &b, 0.5);
    assert!((r.s - 1.0).abs() < 1e-4);
}

// ── evaluate_channel ─────────────────────────────────────────────────────────

#[test]
fn evaluate_channel_empty_returns_none() {
    let ch = BoneChannel {
        bone_name: "bone".to_string(),
        position_keys: vec![],
        rotation_keys: vec![],
        scaling_keys: vec![],
    };
    assert!(evaluate_channel(&ch, 0.0).is_none());
}

#[test]
fn evaluate_channel_single_key_any_time() {
    let ch = BoneChannel {
        bone_name: "b".to_string(),
        position_keys: vec![pos_key(0.0, 1.0, 2.0, 3.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 2.0, 2.0, 2.0)],
    };
    // Far beyond the key — should hold the single key value
    let r = evaluate_channel(&ch, 9999.0);
    assert!(r.is_some());
    let (pos, _rot, scl) = r.unwrap();
    assert!(approx_vec3(pos, Vector3::new(1.0, 2.0, 3.0)));
    assert!(approx_vec3(scl, Vector3::new(2.0, 2.0, 2.0)));
}

#[test]
fn evaluate_channel_midpoint_interpolates() {
    let ch = BoneChannel {
        bone_name: "b".to_string(),
        position_keys: vec![pos_key(0.0, 0.0, 0.0, 0.0), pos_key(10.0, 10.0, 0.0, 0.0)],
        rotation_keys: vec![
            rot_key(0.0, 1.0, 0.0, 0.0, 0.0),
            rot_key(10.0, 1.0, 0.0, 0.0, 0.0),
        ],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0), scl_key(10.0, 1.0, 1.0, 1.0)],
    };
    let (pos, _rot, _scl) = evaluate_channel(&ch, 5.0).unwrap();
    assert!(approx_vec3(pos, Vector3::new(5.0, 0.0, 0.0)));
}

#[test]
fn evaluate_channel_past_last_key_holds() {
    let ch = BoneChannel {
        bone_name: "b".to_string(),
        position_keys: vec![pos_key(0.0, 0.0, 0.0, 0.0), pos_key(10.0, 99.0, 0.0, 0.0)],
        rotation_keys: vec![
            rot_key(0.0, 1.0, 0.0, 0.0, 0.0),
            rot_key(10.0, 1.0, 0.0, 0.0, 0.0),
        ],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0), scl_key(10.0, 1.0, 1.0, 1.0)],
    };
    // Time 9999 is way past the last key; position should hold at last value
    let (pos, _rot, _scl) = evaluate_channel(&ch, 9999.0).unwrap();
    assert!(approx_vec3(pos, Vector3::new(99.0, 0.0, 0.0)));
}

// ── evaluate_clip ────────────────────────────────────────────────────────────

#[test]
fn evaluate_clip_no_matching_channel_returns_bind_pose() {
    // Clip animates "ghost_bone" but skeleton only has "real_bone"
    let channel = BoneChannel {
        bone_name: "ghost_bone".to_string(),
        position_keys: vec![pos_key(0.0, 99.0, 99.0, 99.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 99.0, 99.0, 99.0)],
    };
    let clip = simple_clip("test", vec![channel]);

    let bone_names = vec!["real_bone".to_string()];
    let bind = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0));
    let bind_poses = vec![bind];

    let result = evaluate_clip(&clip, 0.0, &bone_names, &bind_poses);
    assert_eq!(result.len(), 1);
    // Should keep the bind pose, not the animation channel data
    for c in 0..4 {
        for r in 0..4 {
            assert!((result[0][c][r] - bind[c][r]).abs() < 1e-4);
        }
    }
}

#[test]
fn evaluate_clip_matching_channel_overrides_bind_pose() {
    let channel = BoneChannel {
        bone_name: "arm".to_string(),
        // Animate a clear translation
        position_keys: vec![pos_key(0.0, 10.0, 0.0, 0.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0)],
    };
    let clip = simple_clip("test", vec![channel]);

    let bone_names = vec!["arm".to_string()];
    let bind_poses = vec![Matrix4::identity()];

    let result = evaluate_clip(&clip, 0.0, &bone_names, &bind_poses);
    assert_eq!(result.len(), 1);
    // Translation component: column 3, rows 0-2 in cgmath column-major
    let tx = result[0][3][0]; // col 3, row 0
    assert!(
        (tx - 10.0).abs() < 1e-4,
        "expected translation x=10 but got {}",
        tx
    );
}

#[test]
fn evaluate_clip_multiple_bones_independent() {
    let ch_a = BoneChannel {
        bone_name: "bone_a".to_string(),
        position_keys: vec![pos_key(0.0, 1.0, 0.0, 0.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0)],
    };
    let ch_b = BoneChannel {
        bone_name: "bone_b".to_string(),
        position_keys: vec![pos_key(0.0, 0.0, 2.0, 0.0)],
        rotation_keys: vec![rot_key(0.0, 1.0, 0.0, 0.0, 0.0)],
        scaling_keys: vec![scl_key(0.0, 1.0, 1.0, 1.0)],
    };
    let clip = simple_clip("test", vec![ch_a, ch_b]);

    let bone_names = vec!["bone_a".to_string(), "bone_b".to_string()];
    let bind_poses = vec![Matrix4::identity(), Matrix4::identity()];

    let result = evaluate_clip(&clip, 0.0, &bone_names, &bind_poses);
    assert_eq!(result.len(), 2);
    let tx_a = result[0][3][0];
    let ty_b = result[1][3][1];
    assert!((tx_a - 1.0).abs() < 1e-4, "bone_a tx wrong: {}", tx_a);
    assert!((ty_b - 2.0).abs() < 1e-4, "bone_b ty wrong: {}", ty_b);
}

// ── AnimationPlayer ───────────────────────────────────────────────────────────

fn make_clip() -> AnimationClip {
    simple_clip("walk", vec![simple_channel("root")])
}

#[test]
fn player_starts_with_no_clip_not_playing() {
    let p = AnimationPlayer::new();
    assert!(!p.playing);
    assert!(p.clip.is_none());
    assert!(!p.has_clip());
    assert!(p.is_finished());
}

#[test]
fn player_play_sets_clip_and_playing() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    assert!(p.playing);
    assert!(p.has_clip());
    assert!(
        (p.local_time_sec - 0.0).abs() < 1e-9,
        "play() must reset time"
    );
}

#[test]
fn player_stop_clears_playing_and_resets_time() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    p.update(1.0);
    p.stop();
    assert!(!p.playing);
    assert!((p.local_time_sec - 0.0).abs() < 1e-9);
}

#[test]
fn player_pause_does_not_reset_time() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    p.update(0.5);
    let t = p.local_time_sec;
    p.pause();
    assert!(!p.playing);
    assert!(
        (p.local_time_sec - t).abs() < 1e-9,
        "pause must preserve time"
    );
}

#[test]
fn player_resume_after_pause_continues() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    p.update(0.5);
    p.pause();
    p.resume();
    assert!(p.playing);
    p.update(0.5);
    assert!(p.local_time_sec > 0.5, "time should advance after resume");
}

#[test]
fn player_update_advances_time_when_playing() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    p.update(1.0);
    assert!(p.local_time_sec > 0.0);
}

#[test]
fn player_update_does_nothing_when_paused() {
    let mut p = AnimationPlayer::new();
    p.play(make_clip());
    p.pause();
    let t_before = p.local_time_sec;
    p.update(1.0);
    assert!((p.local_time_sec - t_before).abs() < 1e-9);
}

#[test]
fn player_loop_mode_wraps_time() {
    let mut p = AnimationPlayer::new();
    p.loop_mode = LoopMode::Loop;
    let clip = make_clip();
    let duration = clip.duration_seconds();
    p.play(clip);
    // Advance well past the end
    p.update((duration * 3.0) as f32);
    // Time should have wrapped around and stay within [0, duration)
    assert!(
        p.local_time_sec < duration && p.local_time_sec >= 0.0,
        "expected wrapped time in [0, {:.3}), got {:.3}",
        duration,
        p.local_time_sec
    );
    // Still playing
    assert!(p.playing);
}

#[test]
fn player_once_mode_stops_at_end() {
    let mut p = AnimationPlayer::new();
    p.loop_mode = LoopMode::Once;
    let clip = make_clip();
    let duration = clip.duration_seconds();
    p.play(clip);
    // Advance past the end
    p.update((duration * 2.0) as f32);
    assert!(!p.playing, "Once mode should stop");
    assert!(p.is_finished(), "is_finished() should be true");
    assert!(
        (p.local_time_sec - duration).abs() < 1e-4,
        "time should be clamped to duration"
    );
}

#[test]
fn player_speed_scaling_affects_advance_rate() {
    let mut p_normal = AnimationPlayer::new();
    p_normal.speed = 1.0;
    p_normal.play(make_clip());

    let mut p_fast = AnimationPlayer::new();
    p_fast.speed = 2.0;
    p_fast.play(make_clip());

    p_normal.update(0.5);
    p_fast.update(0.5);

    let ratio = p_fast.local_time_sec / p_normal.local_time_sec;
    assert!(
        (ratio - 2.0).abs() < 1e-4,
        "2× speed should advance time 2× faster, got ratio {:.4}",
        ratio
    );
}

// ── Skeleton::make_local_transform ───────────────────────────────────────────

#[test]
fn make_local_transform_identity_inputs() {
    let m = Skeleton::make_local_transform(
        Vector3::zero(),
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
    );
    assert!(
        approx_mat4_identity(m),
        "identity TRS should produce identity matrix"
    );
}

#[test]
fn make_local_transform_pure_translation() {
    let m = Skeleton::make_local_transform(
        Vector3::new(3.0, 5.0, 7.0),
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Vector3::new(1.0, 1.0, 1.0),
    );
    // Translate a point and check result: column 3 should be (3,5,7,1)
    let p = m * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!((p.x - 3.0).abs() < 1e-4 && (p.y - 5.0).abs() < 1e-4 && (p.z - 7.0).abs() < 1e-4);
}

#[test]
fn make_local_transform_pure_scale() {
    let m = Skeleton::make_local_transform(
        Vector3::zero(),
        Quaternion::new(1.0, 0.0, 0.0, 0.0),
        Vector3::new(2.0, 3.0, 4.0),
    );
    let p = m * Vector4::new(1.0, 1.0, 1.0, 1.0);
    assert!((p.x - 2.0).abs() < 1e-4);
    assert!((p.y - 3.0).abs() < 1e-4);
    assert!((p.z - 4.0).abs() < 1e-4);
}

// ── Skeleton::compute_final_matrices ─────────────────────────────────────────

#[test]
fn compute_final_matrices_single_root_identity() {
    let mut skel = Skeleton::new(vec![bone("root", None)]);
    let locals = vec![Matrix4::identity()];
    let finals = skel.compute_final_matrices(&locals);
    assert_eq!(finals.len(), 1);
    assert!(approx_mat4_identity(finals[0]));
}

#[test]
fn compute_final_matrices_single_root_with_offset() {
    let offset = Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0));
    let mut b = bone("root", None);
    b.offset_matrix = offset;
    let mut skel = Skeleton::new(vec![b]);

    let local = Matrix4::identity();
    let finals = skel.compute_final_matrices(&[local]);
    // final = world(=identity) * offset = offset
    let p = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!((p.x - 1.0).abs() < 1e-4, "expected offset to be applied");
}

#[test]
fn compute_final_matrices_parent_child_chain() {
    // Parent translates +5 on X; child translates +3 on X; offset = identity.
    // Expected child world = (5+3, 0, 0).
    let parent_local = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0));
    let child_local = Matrix4::from_translation(Vector3::new(3.0, 0.0, 0.0));

    let mut skel = Skeleton::new(vec![bone("parent", None), bone("child", Some(0))]);
    let finals = skel.compute_final_matrices(&[parent_local, child_local]);

    // Parent final: parent_world * identity_offset = parent_local
    let pp = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (pp.x - 5.0).abs() < 1e-4,
        "parent world x should be 5, got {}",
        pp.x
    );

    // Child final: child_world * identity_offset = parent_local * child_local
    let cp = finals[1] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (cp.x - 8.0).abs() < 1e-4,
        "child world x should be 8, got {}",
        cp.x
    );
}

#[test]
fn compute_final_matrices_out_of_order_bones_still_correct() {
    // Bones listed child-first; `compute_final_matrices` must sort by depth
    // so the parent is computed before the child regardless of list order.
    let parent_local = Matrix4::from_translation(Vector3::new(10.0, 0.0, 0.0));
    let child_local = Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0));

    // bone index 0 = child (parent_index = 1), bone index 1 = parent
    let bones = vec![
        bone("child", Some(1)), // index 0 references parent at index 1
        bone("parent", None),   // index 1 is the root
    ];
    let mut skel = Skeleton::new(bones);
    let locals = vec![child_local, parent_local];
    let finals = skel.compute_final_matrices(&locals);

    // Parent world = parent_local (index 1)
    let pp = finals[1] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!((pp.x - 10.0).abs() < 1e-4, "parent world x={}", pp.x);

    // Child world = parent_local * child_local (index 0)
    let cp = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!((cp.x - 11.0).abs() < 1e-4, "child world x={}", cp.x);
}

#[test]
fn compute_final_matrices_three_level_chain() {
    // grandparent → parent → child, each translating +1 on X.
    // Final child world should be +3 on X (cumulative).
    let tx = Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0));
    let mut skel = Skeleton::new(vec![
        bone("gp", None),
        bone("p", Some(0)),
        bone("c", Some(1)),
    ]);
    let locals = vec![tx, tx, tx];
    let finals = skel.compute_final_matrices(&locals);

    let cp = finals[2] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (cp.x - 3.0).abs() < 1e-4,
        "child world x should be 3, got {}",
        cp.x
    );
}

#[test]
fn compute_final_matrices_result_count_matches_bone_count() {
    let mut skel = Skeleton::new(vec![
        bone("a", None),
        bone("b", Some(0)),
        bone("c", Some(0)),
        bone("d", Some(1)),
    ]);
    let locals = vec![Matrix4::identity(); 4];
    let finals = skel.compute_final_matrices(&locals);
    assert_eq!(finals.len(), 4);
}

// ── Skeleton root_ancestor_transform ────────────────────────────────────

#[test]
fn compute_final_matrices_ancestor_default_identity() {
    let mut skel = Skeleton::new(vec![bone("root", None)]);
    let finals = skel.compute_final_matrices(&[Matrix4::identity()]);
    assert!(approx_mat4_identity(finals[0]));
}

#[test]
fn compute_final_matrices_ancestor_translation() {
    let ancestor = Matrix4::from_translation(Vector3::new(10.0, 0.0, 0.0));
    let mut skel = Skeleton::new(vec![bone("root", None)]);
    skel.root_ancestor_transform = ancestor;

    let finals = skel.compute_final_matrices(&[Matrix4::identity()]);
    let p = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (p.x - 10.0).abs() < 1e-4,
        "expected ancestor translation x=10, got {}",
        p.x
    );
}

#[test]
fn compute_final_matrices_ancestor_propagates_to_child() {
    let ancestor = Matrix4::from_translation(Vector3::new(10.0, 0.0, 0.0));
    let root_local = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0));
    let child_local = Matrix4::from_translation(Vector3::new(3.0, 0.0, 0.0));

    let mut skel = Skeleton::new(vec![
        bone("root", None),
        bone("child", Some(0)),
    ]);
    skel.root_ancestor_transform = ancestor;

    let finals = skel.compute_final_matrices(&[root_local, child_local]);

    let rp = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (rp.x - 15.0).abs() < 1e-4,
        "root x should be ancestor(10) + root(5) = 15, got {}",
        rp.x
    );

    let cp = finals[1] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (cp.x - 18.0).abs() < 1e-4,
        "child x should be ancestor(10) + root(5) + child(3) = 18, got {}",
        cp.x
    );
}

#[test]
fn compute_final_matrices_ancestor_with_offset() {
    let ancestor = Matrix4::from_translation(Vector3::new(10.0, 0.0, 0.0));
    let offset = Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0));
    let mut b = bone("root", None);
    b.offset_matrix = offset;
    let mut skel = Skeleton::new(vec![b]);
    skel.root_ancestor_transform = ancestor;

    let finals = skel.compute_final_matrices(&[Matrix4::identity()]);
    let p = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (p.x - 11.0).abs() < 1e-4,
        "expected ancestor(10) + offset(1) = 11, got {}",
        p.x
    );
}

#[test]
fn compute_final_matrices_ancestor_scale() {
    let ancestor = Matrix4::from_scale(2.0);
    let root_local = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0));

    let mut skel = Skeleton::new(vec![bone("root", None)]);
    skel.root_ancestor_transform = ancestor;

    let finals = skel.compute_final_matrices(&[root_local]);
    let p = finals[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (p.x - 10.0).abs() < 1e-4,
        "expected scale(2) * translate(5,0,0) x=10, got {}",
        p.x
    );
}

// ── AnimationPlayer::evaluate bind pose with ancestor ────────────────────

#[test]
fn player_evaluate_no_clip_returns_bind_pose_including_ancestor() {
    let ancestor = Matrix4::from_translation(Vector3::new(10.0, 0.0, 0.0));
    let bind_local = Matrix4::from_translation(Vector3::new(5.0, 0.0, 0.0));
    let mut skel = Skeleton::new(vec![bone_with_local("root", None, bind_local)]);
    skel.root_ancestor_transform = ancestor;

    let player = AnimationPlayer::new();

    let matrices = player.evaluate(&mut skel);
    assert_eq!(matrices.len(), 1);
    let p = matrices[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (p.x - 15.0).abs() < 1e-4,
        "bind pose x should be ancestor(10) + bind_local(5) = 15, got {}",
        p.x
    );
}

#[test]
fn player_evaluate_no_clip_includes_ancestor_in_all_bones() {
    let ancestor = Matrix4::from_scale(2.0);
    let root_local = Matrix4::from_translation(Vector3::new(3.0, 4.0, 0.0));
    let child_local = Matrix4::from_translation(Vector3::new(1.0, 0.0, 0.0));

    let mut skel = Skeleton::new(vec![
        bone_with_local("root", None, root_local),
        bone_with_local("child", Some(0), child_local),
    ]);
    skel.root_ancestor_transform = ancestor;

    let player = AnimationPlayer::new();
    let matrices = player.evaluate(&mut skel);

    // Root final = ancestor * root_local * offset(identity)
    // = scale(2) * translate(3,4,0)
    // Vertex at (0,0,0) → (6, 8, 0)
    let rp = matrices[0] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (rp.x - 6.0).abs() < 1e-4 && (rp.y - 8.0).abs() < 1e-4,
        "root bind pose should be (6, 8, 0), got ({}, {}, {})",
        rp.x, rp.y, rp.z
    );

    // Child final = ancestor * root_local * child_local * offset(identity)
    // = scale(2) * translate(3,4,0) * translate(1,0,0)
    // Vertex at (0,0,0) → scale(2) * (4, 4, 0) = (8, 8, 0)
    let cp = matrices[1] * Vector4::new(0.0, 0.0, 0.0, 1.0);
    assert!(
        (cp.x - 8.0).abs() < 1e-4 && (cp.y - 8.0).abs() < 1e-4,
        "child bind pose should be (8, 8, 0), got ({}, {}, {})",
        cp.x, cp.y, cp.z
    );
}
