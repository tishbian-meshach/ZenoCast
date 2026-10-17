//! Motion-graphics and mathematical built-in functions for Rhai expressions.

/// Fast 1D deterministic hash returning a pseudo-random value in [0.0..1.0].
#[inline]
pub fn noise_hash(seed: u64, i: i64) -> f64 {
    let mut h = seed
        .wrapping_add((i as u64) << 32)
        .wrapping_add(0x9e3779b97f4a7c15);
    h ^= h >> 30;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94d049bb133111eb);
    h ^= h >> 31;
    ((h & 0xFFFFFFFF) as f64) / (u32::MAX as f64)
}

/// Evaluates a smooth continuous random oscillation (After Effects wiggle).
///
/// Returns a value centered around 0 with amplitude `amp` and oscillation rate `freq` Hz.
pub fn calculate_wiggle(time: f64, freq: f64, amp: f64, seed: u64) -> f64 {
    let f = freq.max(0.001);
    let x = time * f;
    let i0 = x.floor() as i64;
    let i1 = i0 + 1;
    let frac = x - x.floor();

    // Smooth cubic Hermite interpolation curve: 3s^2 - 2s^3
    let s = frac * frac * (3.0 - 2.0 * frac);

    let r0 = (noise_hash(seed, i0) - 0.5) * 2.0;
    let r1 = (noise_hash(seed, i1) - 0.5) * 2.0;

    let noise = r0 + s * (r1 - r0);
    noise * amp
}

/// Maps input `t` from `[t_min, t_max]` linearly to `[val1, val2]`.
pub fn calculate_linear(t: f64, t_min: f64, t_max: f64, val1: f64, val2: f64) -> f64 {
    if (t_max - t_min).abs() < 1e-6 {
        return val1;
    }
    let u = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    val1 + u * (val2 - val1)
}

/// Maps input `t` from `[t_min, t_max]` with smooth cubic ease curve to `[val1, val2]`.
pub fn calculate_ease(t: f64, t_min: f64, t_max: f64, val1: f64, val2: f64) -> f64 {
    if (t_max - t_min).abs() < 1e-6 {
        return val1;
    }
    let u = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    let s = u * u * (3.0 - 2.0 * u);
    val1 + s * (val2 - val1)
}

/// Clamps value to `[min_val, max_val]`.
pub fn calculate_clamp(val: f64, min_val: f64, max_val: f64) -> f64 {
    val.clamp(min_val, max_val)
}
