//! Dynamic properties, keyframe definitions, and easing functions.

use serde::{Deserialize, Serialize};

/// Animation easing curve governing interpolation between the current keyframe and the subsequent keyframe.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(tag = "type")]
pub enum Easing {
    /// Step transition: holds the current value until the exact instant of the next keyframe.
    Hold,
    /// Constant-speed straight linear interpolation.
    #[default]
    Linear,
    /// Cubic Bezier curve defined by two control points (CSS cubic-bezier curve).
    CubicBezier {
        /// X coordinate of first control handle (0.0 to 1.0).
        control_1_x: f32,
        /// Y coordinate of first control handle.
        control_1_y: f32,
        /// X coordinate of second control handle (0.0 to 1.0).
        control_2_x: f32,
        /// Y coordinate of second control handle.
        control_2_y: f32,
    },
}

impl Easing {
    /// Standard ease (0.25, 0.1, 0.25, 1.0).
    pub const fn ease() -> Self {
        Self::CubicBezier {
            control_1_x: 0.25,
            control_1_y: 0.1,
            control_2_x: 0.25,
            control_2_y: 1.0,
        }
    }

    /// Standard ease-in (0.42, 0.0, 1.0, 1.0).
    pub const fn ease_in() -> Self {
        Self::CubicBezier {
            control_1_x: 0.42,
            control_1_y: 0.0,
            control_2_x: 1.0,
            control_2_y: 1.0,
        }
    }

    /// Standard ease-out (0.0, 0.0, 0.58, 1.0).
    pub const fn ease_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.0,
            control_1_y: 0.0,
            control_2_x: 0.58,
            control_2_y: 1.0,
        }
    }

    /// Standard ease-in-out (0.42, 0.0, 0.58, 1.0).
    pub const fn ease_in_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.42,
            control_1_y: 0.0,
            control_2_x: 0.58,
            control_2_y: 1.0,
        }
    }

    /// Quadratic ease-in approximation.
    pub const fn quad_in() -> Self {
        Self::CubicBezier {
            control_1_x: 0.11,
            control_1_y: 0.0,
            control_2_x: 0.5,
            control_2_y: 0.0,
        }
    }

    /// Quadratic ease-out approximation.
    pub const fn quad_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.5,
            control_1_y: 1.0,
            control_2_x: 0.89,
            control_2_y: 1.0,
        }
    }

    /// Quadratic ease-in-out approximation.
    pub const fn quad_in_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.45,
            control_1_y: 0.0,
            control_2_x: 0.55,
            control_2_y: 1.0,
        }
    }

    /// Sine ease-in approximation.
    pub const fn sine_in() -> Self {
        Self::CubicBezier {
            control_1_x: 0.12,
            control_1_y: 0.0,
            control_2_x: 0.39,
            control_2_y: 0.0,
        }
    }

    /// Sine ease-out approximation.
    pub const fn sine_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.61,
            control_1_y: 1.0,
            control_2_x: 0.88,
            control_2_y: 1.0,
        }
    }

    /// Sine ease-in-out approximation.
    pub const fn sine_in_out() -> Self {
        Self::CubicBezier {
            control_1_x: 0.37,
            control_1_y: 0.0,
            control_2_x: 0.63,
            control_2_y: 1.0,
        }
    }

    /// Evaluates the easing curve for a normalized progress `t` in `[0.0, 1.0]`.
    /// Returns the eased output progress.
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            Self::Hold => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Linear => t,
            Self::CubicBezier {
                control_1_x,
                control_1_y,
                control_2_x,
                control_2_y,
            } => solve_cubic_bezier(t, control_1_x, control_1_y, control_2_x, control_2_y),
        }
    }
}

/// Solves cubic bezier curve for given `x` in `[0.0, 1.0]` using Newton-Raphson with bisection fallback.
fn solve_cubic_bezier(x: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Bézier coefficients for X(t) = 3*(1-t)^2*t*x1 + 3*(1-t)*t^2*x2 + t^3
    // = t * (3*x1 + t * (3*(x2 - 2*x1) + t * (1 - 3*x2 + 3*x1)))
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;

    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;

    let sample_curve_x = |t: f32| ((ax * t + bx) * t + cx) * t;
    let sample_curve_y = |t: f32| ((ay * t + by) * t + cy) * t;
    let sample_curve_derivative_x = |t: f32| (3.0 * ax * t + 2.0 * bx) * t + cx;

    // Newton-Raphson iteration
    let mut t = x;
    for _ in 0..8 {
        let x_current = sample_curve_x(t) - x;
        if x_current.abs() < 1e-5 {
            return sample_curve_y(t);
        }
        let d = sample_curve_derivative_x(t);
        if d.abs() < 1e-6 {
            break;
        }
        t -= x_current / d;
        if !(0.0..=1.0).contains(&t) {
            break;
        }
    }

    // Bisection search fallback
    let mut t0 = 0.0;
    let mut t1 = 1.0;
    t = x;

    for _ in 0..16 {
        let x_current = sample_curve_x(t);
        if (x_current - x).abs() < 1e-5 {
            return sample_curve_y(t);
        }
        if x > x_current {
            t0 = t;
        } else {
            t1 = t;
        }
        t = (t0 + t1) * 0.5;
    }

    sample_curve_y(t)
}

/// Trait for values that can be linearly interpolated or stepped between two animation states.
pub trait Interpolate: Clone {
    /// Linearly interpolate between `self` and `target` with normalized progress `t` in `[0.0, 1.0]`.
    fn interpolate(&self, target: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        *self + (*target - *self) * t
    }
}

impl Interpolate for f64 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        *self + (*target - *self) * (t as f64)
    }
}

impl Interpolate for glam::Vec2 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        self.lerp(*target, t)
    }
}

impl Interpolate for glam::Vec3 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        self.lerp(*target, t)
    }
}

impl Interpolate for glam::Vec4 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        self.lerp(*target, t)
    }
}

impl Interpolate for crate::color::Color {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        self.lerp(target, t)
    }
}

impl Interpolate for u32 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        ((*self as f64) + ((*target as f64) - (*self as f64)) * (t as f64)).round() as u32
    }
}

impl Interpolate for i32 {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        ((*self as f64) + ((*target as f64) - (*self as f64)) * (t as f64)).round() as i32
    }
}

impl Interpolate for bool {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        if t >= 1.0 {
            *target
        } else {
            *self
        }
    }
}

impl Interpolate for String {
    #[inline]
    fn interpolate(&self, target: &Self, t: f32) -> Self {
        if t >= 1.0 {
            target.clone()
        } else {
            self.clone()
        }
    }
}

/// An individual keyframe anchored at a specific timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe<Value> {
    /// Time position of this keyframe in seconds.
    pub time_seconds: f64,
    /// Value at this keyframe.
    pub value: Value,
    /// Easing curve applied between this keyframe and the next keyframe in timeline order.
    pub easing_to_next: Easing,
}

impl<Value> Keyframe<Value> {
    /// Create a new keyframe with specified easing.
    #[inline]
    pub const fn new(time_seconds: f64, value: Value, easing_to_next: Easing) -> Self {
        Self {
            time_seconds,
            value,
            easing_to_next,
        }
    }

    /// Create a keyframe with linear easing.
    #[inline]
    pub const fn linear(time_seconds: f64, value: Value) -> Self {
        Self::new(time_seconds, value, Easing::Linear)
    }

    /// Create a keyframe with hold easing.
    #[inline]
    pub const fn hold(time_seconds: f64, value: Value) -> Self {
        Self::new(time_seconds, value, Easing::Hold)
    }
}

/// Evaluates an ordered slice of keyframes at the specified timestamp.
///
/// Returns the first keyframe value if `time_seconds <= keys[0].time_seconds`,
/// the last keyframe value if `time_seconds >= keys.last().time_seconds`,
/// or interpolates between adjacent keyframes using the starting keyframe's easing curve.
pub fn evaluate_keyframes<Value: Interpolate>(
    keys: &[Keyframe<Value>],
    time_seconds: f64,
) -> Value {
    if keys.is_empty() {
        panic!("Cannot evaluate empty keyframes list");
    }
    if keys.len() == 1 || time_seconds <= keys[0].time_seconds {
        return keys[0].value.clone();
    }
    let last = keys.last().unwrap();
    if time_seconds >= last.time_seconds {
        return last.value.clone();
    }

    // Binary search to find keyframe interval: keys[i].time_seconds <= time_seconds < keys[i+1].time_seconds
    let idx = match keys.binary_search_by(|k| {
        k.time_seconds
            .partial_cmp(&time_seconds)
            .unwrap_or(std::cmp::Ordering::Equal)
    }) {
        Ok(exact) => exact,
        Err(next_idx) => next_idx.saturating_sub(1),
    };

    if idx >= keys.len() - 1 {
        return last.value.clone();
    }

    let k_curr = &keys[idx];
    let k_next = &keys[idx + 1];

    let span = k_next.time_seconds - k_curr.time_seconds;
    let raw_t = if span <= 0.0 {
        1.0
    } else {
        ((time_seconds - k_curr.time_seconds) / span).clamp(0.0, 1.0) as f32
    };

    let eased_t = k_curr.easing_to_next.evaluate(raw_t);
    k_curr.value.interpolate(&k_next.value, eased_t)
}

/// A dynamic property that can be a constant, keyframed animation, live data binding, or expression.
///
/// Resolution order in the engine:
/// 1. `Binding`
/// 2. `Expression`
/// 3. `Keyframes`
/// 4. `Constant`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Property<Value> {
    /// Fixed static value.
    Constant(Value),
    /// Animated value driven by timeline keyframes.
    Keyframes(Vec<Keyframe<Value>>),
    /// Live data binding referencing a variable path in the data store.
    Binding {
        /// JSON dot-separated data path (e.g. "scoreboard.team1.score").
        data_path: String,
        /// Fallback value used when data path is missing or unbound.
        fallback: Value,
    },
    /// Sandboxed Rhai expression evaluated dynamically per frame.
    Expression {
        /// Script source code.
        source: String,
        /// Fallback value used if evaluation errors or times out.
        fallback: Value,
    },
}

impl<Value: Default> Default for Property<Value> {
    fn default() -> Self {
        Self::Constant(Value::default())
    }
}

impl<Value> Property<Value> {
    /// Create a constant property.
    #[inline]
    pub const fn constant(value: Value) -> Self {
        Self::Constant(value)
    }

    /// Create a keyframed property.
    #[inline]
    pub fn keyframes(keys: Vec<Keyframe<Value>>) -> Self {
        Self::Keyframes(keys)
    }

    /// Create a data-bound property with fallback.
    #[inline]
    pub fn binding(data_path: impl Into<String>, fallback: Value) -> Self {
        Self::Binding {
            data_path: data_path.into(),
            fallback,
        }
    }

    /// Create an expression property with fallback.
    #[inline]
    pub fn expression(source: impl Into<String>, fallback: Value) -> Self {
        Self::Expression {
            source: source.into(),
            fallback,
        }
    }

    /// Extracts the static or fallback value.
    pub fn fallback_value(&self) -> &Value {
        match self {
            Self::Constant(val) => val,
            Self::Keyframes(keys) => {
                if let Some(first) = keys.first() {
                    &first.value
                } else {
                    panic!("Empty keyframes property has no fallback value");
                }
            }
            Self::Binding { fallback, .. } => fallback,
            Self::Expression { fallback, .. } => fallback,
        }
    }
}

impl<Value: Interpolate> Property<Value> {
    /// Evaluates the property value at `time_seconds`.
    ///
    /// Evaluation order (Section 6):
    /// 1. `Binding` -> uses fallback until live data store is attached
    /// 2. `Expression` -> uses fallback until script engine is evaluated
    /// 3. `Keyframes` -> evaluates interpolated keyframes at `time_seconds`
    /// 4. `Constant` -> returns static value
    pub fn evaluate_at(&self, time_seconds: f64) -> Value {
        match self {
            Self::Constant(val) => val.clone(),
            Self::Keyframes(keys) => {
                if keys.is_empty() {
                    panic!("Property::Keyframes is empty");
                }
                evaluate_keyframes(keys, time_seconds)
            }
            Self::Binding { fallback, .. } => fallback.clone(),
            Self::Expression { fallback, .. } => fallback.clone(),
        }
    }
}

impl<Value> From<Value> for Property<Value> {
    fn from(val: Value) -> Self {
        Self::Constant(val)
    }
}
