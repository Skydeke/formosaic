//! Entropy-based puzzle quality scoring grounded in information theory.
//!
//! # Background
//!
//! A good Formosaic puzzle has exactly one "clear solution":
//! there should be one viewing direction where the model looks complete,
//! and every other direction should look obviously wrong.
//!
//! We measure this with Shannon entropy over a discretised sphere of viewpoints:
//!
//!   H = -Σ p(θ) · log₂(p(θ))
//!
//! where p(θ) is the "alignment score" for viewpoint θ, normalised over the
//! sphere so the sum is 1.  A good puzzle has **low entropy** — almost all the
//! probability mass sits at the one correct viewpoint.
//!
//! ## Alignment score
//!
//! For a given camera direction `d`, we measure how well the scrambled triangles
//! appear aligned when projected along `d`.  Concretely, for each triangle we
//! project its scramble-offset vector onto `d` and compute the variance of those
//! projected values.  A small variance means all triangles have a similar
//! apparent depth — they look aligned from that direction.  We invert this to
//! get a positive score: `score(d) = 1 / (1 + projected_variance(d))`.
//!
//! ## Mutual information axis selection
//!
//! `best_scramble_axis` samples N candidate axes, scores each one by running a
//! candidate scramble, then returns the axis whose entropy is lowest (most
//! distinctive puzzle).

use cgmath::{InnerSpace, Vector3};
use std::f32::consts::PI;

use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::architecture::models::simple_model::SimpleModel;

// ─── Public types ────────────────────────────────────────────────────────────

/// Quality descriptor returned by `analyse_axis`.
#[derive(Debug, Clone, Copy)]
pub struct EntropyReport {
    /// Shannon entropy (bits) of the viewpoint-score distribution.
    /// Lower = more distinctive / easier to identify the solution.
    pub entropy_bits: f32,

    /// Peak alignment score (should be ≈1.0 for a clean solution).
    pub peak_score: f32,

    /// Angular distance (radians) between the top-2 candidate viewpoints.
    /// Large = the solution is well isolated → better puzzle.
    pub solution_isolation_rad: f32,

    /// Estimated puzzle difficulty in [0,1]: 0 = trivial, 1 = very hard.
    pub difficulty: f32,
}

/// Result of finding the best axis via mutual-information search.
pub struct AxisSearchResult {
    /// The chosen scramble axis.
    pub axis: Vector3<f32>,
    /// Entropy report for the chosen axis.
    pub report: EntropyReport,
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Analyse how good `axis` is as a Formosaic puzzle solution axis for `model`.
///
/// `scramble_offsets_flat` is the flat list of per-vertex scramble offsets
/// as stored in `Mesh::scramble_offsets` (3 floats per vertex, triangle order).
pub fn analyse_axis(
    scramble_offsets_flat: &[f32],
    axis: Vector3<f32>,
) -> EntropyReport {
    let directions = fibonacci_sphere(64);
    let scores: Vec<f32> = directions
        .iter()
        .map(|&d| alignment_score(scramble_offsets_flat, d))
        .collect();

    entropy_report(&directions, &scores, axis)
}

/// Search for the scramble axis that produces the most informative puzzle.
///
/// Samples `candidates` random axes, evaluates them via a lightweight score,
/// and returns the one with the lowest entropy (most distinctive solution).
pub fn best_scramble_axis(
    model: &SimpleModel,
    candidates: usize,
    target_world_radius: f32,
    fov_radians: f32,
) -> AxisSearchResult {
    use rand::Rng;
    let mut rng = rand::rng();

    let params = model.compute_puzzle_params(target_world_radius, fov_radians);

    let mut best_axis = Vector3::new(0.0, 1.0, 0.0);
    let mut best_report = EntropyReport {
        entropy_bits: f32::INFINITY,
        peak_score: 0.0,
        solution_isolation_rad: 0.0,
        difficulty: 1.0,
    };

    // We work with a throwaway clone of the offsets buffer so we can re-scramble
    // without touching the real GPU-backed mesh data.
    let positions = model_positions_flat(model);

    for _ in 0..candidates {
        let theta: f32 = rng.random_range(0.0..2.0 * PI);
        let phi: f32   = rng.random_range(-PI / 2.0..PI / 2.0);
        let axis = Vector3::new(
            phi.cos() * theta.cos(),
            phi.sin(),
            phi.cos() * theta.sin(),
        ).normalize();

        let offsets = simulate_scramble_offsets(&positions, axis, params.min_disp, params.max_disp);
        let report  = analyse_axis(&offsets, axis);

        // We want: low entropy AND well-isolated solution AND moderate difficulty
        let score = report.entropy_bits - report.solution_isolation_rad * 2.0;
        let best_score = best_report.entropy_bits - best_report.solution_isolation_rad * 2.0;

        if score < best_score {
            best_axis   = axis;
            best_report = report;
        }
    }

    log::info!(
        "[Entropy] best axis entropy={:.3} bits  isolation={:.2}°  difficulty={:.2}",
        best_report.entropy_bits,
        best_report.solution_isolation_rad.to_degrees(),
        best_report.difficulty,
    );

    AxisSearchResult { axis: best_axis, report: best_report }
}

/// Difficulty label for UI display.
pub fn difficulty_label(difficulty: f32) -> &'static str {
    match difficulty {
        d if d < 0.25 => "Easy",
        d if d < 0.50 => "Medium",
        d if d < 0.75 => "Hard",
        _             => "Expert",
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

/// Alignment score for camera direction `d`.
///
/// Projects every triangle's scramble offset onto `d`.
/// Returns 1/(1 + variance) — high score = looks aligned from this direction.
fn alignment_score(offsets_flat: &[f32], d: Vector3<f32>) -> f32 {
    if offsets_flat.is_empty() {
        return 1.0;
    }

    // One projected value per triangle (average of its 3 vertices' offsets).
    let tri_count = offsets_flat.len() / 9;
    if tri_count == 0 {
        return 1.0;
    }

    let projections: Vec<f32> = (0..tri_count)
        .map(|t| {
            let base = t * 9;
            let mut proj_sum = 0.0f32;
            for corner in 0..3 {
                let v = base + corner * 3;
                let ox = offsets_flat[v];
                let oy = offsets_flat[v + 1];
                let oz = offsets_flat[v + 2];
                proj_sum += ox * d.x + oy * d.y + oz * d.z;
            }
            proj_sum / 3.0
        })
        .collect();

    let mean = projections.iter().sum::<f32>() / tri_count as f32;
    let variance = projections.iter().map(|&p| (p - mean).powi(2)).sum::<f32>() / tri_count as f32;

    1.0 / (1.0 + variance)
}

/// Compute the full entropy report over a set of sampled viewpoints.
fn entropy_report(
    directions: &[Vector3<f32>],
    scores: &[f32],
    solution_axis: Vector3<f32>,
) -> EntropyReport {
    let sum: f32 = scores.iter().sum();
    if sum < 1e-12 {
        return EntropyReport {
            entropy_bits: 0.0,
            peak_score: 0.0,
            solution_isolation_rad: 0.0,
            difficulty: 0.5,
        };
    }

    // Normalise → probability distribution
    let probs: Vec<f32> = scores.iter().map(|&s| s / sum).collect();

    // Shannon entropy
    let entropy_bits: f32 = probs
        .iter()
        .filter(|&&p| p > 1e-12)
        .map(|&p| -p * p.log2())
        .sum();

    let peak_score = scores.iter().cloned().fold(0.0_f32, f32::max);

    // Find the angle between the solution direction and the 2nd-best viewpoint
    // that isn't near the solution.
    let solution_dot = solution_axis.normalize();
    let mut second_best_score = 0.0f32;
    let mut second_best_dot   = 0.0f32;
    for (i, &d) in directions.iter().enumerate() {
        let dot = d.dot(solution_dot).abs();
        if dot < 0.95 && scores[i] > second_best_score {
            second_best_score = scores[i];
            second_best_dot   = dot;
        }
    }
    let solution_isolation_rad = second_best_dot.acos().min(PI);

    // Difficulty heuristic:
    //   low entropy + good isolation → easy to find solution → low difficulty
    //   high entropy                 → many plausible viewpoints → hard
    let max_entropy = (directions.len() as f32).log2();
    let normalised_entropy = (entropy_bits / max_entropy).clamp(0.0, 1.0);
    // Also factor in peak score: a weak peak (< 0.5) means the puzzle may not
    // have a satisfying snap moment, increasing perceived difficulty.
    let peak_factor = (1.0 - peak_score).clamp(0.0, 1.0) * 0.3;
    let difficulty  = (normalised_entropy * 0.7 + peak_factor).clamp(0.0, 1.0);

    EntropyReport {
        entropy_bits,
        peak_score,
        solution_isolation_rad,
        difficulty,
    }
}

/// Generate N points roughly evenly distributed on a unit sphere
/// using the Fibonacci / golden-angle spiral method.
fn fibonacci_sphere(n: usize) -> Vec<Vector3<f32>> {
    let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
    (0..n)
        .map(|i| {
            let theta = 2.0 * PI * i as f32 / golden;
            let phi   = (1.0 - 2.0 * (i as f32 + 0.5) / n as f32).acos();
            Vector3::new(
                phi.sin() * theta.cos(),
                phi.cos(),
                phi.sin() * theta.sin(),
            )
        })
        .collect()
}

/// Extract all vertex positions from a model as a flat Vec<f32>.
fn model_positions_flat(model: &SimpleModel) -> Vec<f32> {
    model
        .get_meshes()
        .iter()
        .flat_map(|m| m.positions().to_vec())
        .collect()
}

/// Simulate what the scramble offsets would look like for a given axis,
/// without touching any GPU state.  Uses the same distribution as `scramble_along_axis`.
fn simulate_scramble_offsets(
    positions_flat: &[f32],
    axis: Vector3<f32>,
    min_disp: f32,
    max_disp: f32,
) -> Vec<f32> {
    use rand::Rng;
    let mut rng = rand::rng();

    let n = positions_flat.len();
    let tri_count = n / 9;
    let mut offsets = vec![0.0f32; n];

    for tri in 0..tri_count {
        let amount: f32 = rng.random_range(min_disp..max_disp);
        // Use positive-only displacement — must match `scramble_along_axis` exactly.
        let disp = axis * amount;

        let base = tri * 9;
        for corner in 0..3 {
            let v = base + corner * 3;
            offsets[v]     = disp.x;
            offsets[v + 1] = disp.y;
            offsets[v + 2] = disp.z;
        }
    }

    offsets
}
