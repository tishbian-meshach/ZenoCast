//! Sandboxed expression execution runtime with instruction limits,
//! caching, and motion-graphics built-ins.

use parking_lot::RwLock;
use rhai::{Dynamic, Engine, Scope, AST};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use zeno_core::{Vec2, Vec3};

use crate::builtins::{calculate_clamp, calculate_ease, calculate_linear, calculate_wiggle};
use crate::error::ScriptError;

/// Maximum instruction operations permitted per expression execution (infinite loop cutoff).
pub const MAX_EXPRESSION_OPERATIONS: u64 = 10_000;

thread_local! {
    static CURRENT_TIME: Cell<f64> = const { Cell::new(0.0) };
    static CURRENT_DATA: RefCell<HashMap<String, serde_json::Value>> = RefCell::new(HashMap::new());
}

/// Sandboxed expression engine for compiling and evaluating dynamic motion-graphics scripts.
#[derive(Clone)]
pub struct ScriptRuntime {
    engine: Arc<Engine>,
    ast_cache: Arc<RwLock<HashMap<String, AST>>>,
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRuntime {
    /// Initializes a new sandboxed expression runtime with instruction limits and motion-graphics built-ins.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // 1. Sandbox constraints: strict instruction limit & call level limits
        engine.set_max_operations(MAX_EXPRESSION_OPERATIONS);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(4096);
        engine.set_max_array_size(1024);

        // 2. Register motion-graphics built-ins
        engine.register_fn(
            "linear",
            |t: f64, t_min: f64, t_max: f64, v1: f64, v2: f64| -> f64 {
                calculate_linear(t, t_min, t_max, v1, v2)
            },
        );

        engine.register_fn(
            "ease",
            |t: f64, t_min: f64, t_max: f64, v1: f64, v2: f64| -> f64 {
                calculate_ease(t, t_min, t_max, v1, v2)
            },
        );

        engine.register_fn("wiggle", |freq: f64, amp: f64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq, amp, 1337)
        });
        engine.register_fn("wiggle", |freq: i64, amp: i64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq as f64, amp as f64, 1337)
        });
        engine.register_fn("wiggle", |freq: f64, amp: i64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq, amp as f64, 1337)
        });
        engine.register_fn("wiggle", |freq: i64, amp: f64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq as f64, amp, 1337)
        });

        engine.register_fn("clamp", |val: f64, min_v: f64, max_v: f64| -> f64 {
            calculate_clamp(val, min_v, max_v)
        });
        engine.register_fn("clamp", |val: i64, min_v: i64, max_v: i64| -> i64 {
            val.clamp(min_v, max_v)
        });

        engine.register_fn("wiggle_seeded", |freq: f64, amp: f64, seed: i64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq, amp, seed as u64)
        });
        engine.register_fn("wiggle_seeded", |freq: i64, amp: i64, seed: i64| -> f64 {
            let t = CURRENT_TIME.with(|c| c.get());
            calculate_wiggle(t, freq as f64, amp as f64, seed as u64)
        });

        engine.register_fn("data", |path: &str| -> Dynamic {
            CURRENT_DATA.with(|d| {
                let map = d.borrow();
                if let Some(val) = map.get(path) {
                    if let Some(n) = val.as_f64() {
                        Dynamic::from(n)
                    } else if let Some(s) = val.as_str() {
                        Dynamic::from(s.to_string())
                    } else if let Some(b) = val.as_bool() {
                        Dynamic::from(b)
                    } else {
                        Dynamic::from(val.to_string())
                    }
                } else {
                    Dynamic::from(0.0)
                }
            })
        });

        engine.register_fn("deg2rad", |deg: f64| -> f64 { deg.to_radians() });
        engine.register_fn("rad2deg", |rad: f64| -> f64 { rad.to_degrees() });

        // Math utilities
        engine.register_fn("abs", |x: f64| -> f64 { x.abs() });
        engine.register_fn("sin", |x: f64| -> f64 { x.sin() });
        engine.register_fn("cos", |x: f64| -> f64 { x.cos() });
        engine.register_fn("tan", |x: f64| -> f64 { x.tan() });
        engine.register_fn("sqrt", |x: f64| -> f64 { x.sqrt() });
        engine.register_fn("floor", |x: f64| -> f64 { x.floor() });
        engine.register_fn("ceil", |x: f64| -> f64 { x.ceil() });
        engine.register_fn("round", |x: f64| -> f64 { x.round() });
        engine.register_fn("min", |a: f64, b: f64| -> f64 { a.min(b) });
        engine.register_fn("max", |a: f64, b: f64| -> f64 { a.max(b) });

        Self {
            engine: Arc::new(engine),
            ast_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compiles a script into an AST or retrieves it from the compilation cache.
    pub fn compile(&self, script: &str) -> Result<AST, ScriptError> {
        let trimmed = script.trim();
        {
            let cache = self.ast_cache.read();
            if let Some(ast) = cache.get(trimmed) {
                return Ok(ast.clone());
            }
        }

        let ast = self
            .engine
            .compile(trimmed)
            .map_err(|e| ScriptError::CompileError(e.to_string()))?;

        let mut cache = self.ast_cache.write();
        cache.insert(trimmed.to_string(), ast.clone());
        Ok(ast)
    }

    /// Prepares contextual thread-local state prior to expression evaluation.
    fn prepare_context(
        &self,
        time: f64,
        data_snapshot: Option<&HashMap<String, serde_json::Value>>,
    ) {
        CURRENT_TIME.with(|c| c.set(time));
        CURRENT_DATA.with(|d| {
            let mut map = d.borrow_mut();
            map.clear();
            if let Some(data) = data_snapshot {
                map.clone_from(data);
            }
        });
    }

    /// Evaluates a numeric expression returning an `f64`.
    pub fn eval_f64(
        &self,
        script: &str,
        time: f64,
        index: i64,
        value: f64,
        data_snapshot: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<f64, ScriptError> {
        let ast = self.compile(script)?;
        self.prepare_context(time, data_snapshot);

        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("index", index);
        scope.push("value", value);

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
            .map_err(map_rhai_error)?;

        if let Ok(num) = result.as_float() {
            Ok(num)
        } else if let Ok(int_val) = result.as_int() {
            Ok(int_val as f64)
        } else {
            Err(ScriptError::TypeMismatch {
                expected: "number",
                actual: result.type_name().to_string(),
            })
        }
    }

    /// Evaluates a text expression returning a `String`.
    pub fn eval_string(
        &self,
        script: &str,
        time: f64,
        index: i64,
        value: &str,
        data_snapshot: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<String, ScriptError> {
        let ast = self.compile(script)?;
        self.prepare_context(time, data_snapshot);

        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("index", index);
        scope.push("value", value.to_string());

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
            .map_err(map_rhai_error)?;

        if let Ok(s) = result.clone().into_string() {
            Ok(s)
        } else {
            Ok(result.to_string())
        }
    }

    /// Evaluates a 2D vector expression returning a `Vec2`.
    pub fn eval_vec2(
        &self,
        script: &str,
        time: f64,
        index: i64,
        value: Vec2,
        data_snapshot: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<Vec2, ScriptError> {
        let ast = self.compile(script)?;
        self.prepare_context(time, data_snapshot);

        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("index", index);
        let val_arr: rhai::Array =
            vec![Dynamic::from(value.x as f64), Dynamic::from(value.y as f64)];
        scope.push("value", val_arr);

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
            .map_err(map_rhai_error)?;

        if let Some(arr) = result.clone().try_cast::<rhai::Array>() {
            let x = arr
                .first()
                .and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
                .unwrap_or(value.x as f64) as f32;
            let y = arr
                .get(1)
                .and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
                .unwrap_or(value.y as f64) as f32;
            Ok(Vec2::new(x, y))
        } else if let Ok(num) = result.as_float() {
            Ok(Vec2::splat(num as f32))
        } else {
            Err(ScriptError::TypeMismatch {
                expected: "Array [x, y] or number",
                actual: result.type_name().to_string(),
            })
        }
    }

    /// Evaluates a 3D vector expression returning a `Vec3`.
    pub fn eval_vec3(
        &self,
        script: &str,
        time: f64,
        index: i64,
        value: Vec3,
        data_snapshot: Option<&HashMap<String, serde_json::Value>>,
    ) -> Result<Vec3, ScriptError> {
        let ast = self.compile(script)?;
        self.prepare_context(time, data_snapshot);

        let mut scope = Scope::new();
        scope.push("time", time);
        scope.push("index", index);
        let val_arr: rhai::Array = vec![
            Dynamic::from(value.x as f64),
            Dynamic::from(value.y as f64),
            Dynamic::from(value.z as f64),
        ];
        scope.push("value", val_arr);

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &ast)
            .map_err(map_rhai_error)?;

        if let Some(arr) = result.clone().try_cast::<rhai::Array>() {
            let x = arr
                .first()
                .and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
                .unwrap_or(value.x as f64) as f32;
            let y = arr
                .get(1)
                .and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
                .unwrap_or(value.y as f64) as f32;
            let z = arr
                .get(2)
                .and_then(|v| {
                    v.as_float()
                        .ok()
                        .or_else(|| v.as_int().ok().map(|i| i as f64))
                })
                .unwrap_or(value.z as f64) as f32;
            Ok(Vec3::new(x, y, z))
        } else if let Ok(num) = result.as_float() {
            Ok(Vec3::splat(num as f32))
        } else {
            Err(ScriptError::TypeMismatch {
                expected: "Array [x, y, z] or number",
                actual: result.type_name().to_string(),
            })
        }
    }
}

/// Maps Rhai evaluation error variants into our typed `ScriptError`.
#[allow(clippy::boxed_local)]
fn map_rhai_error(err: Box<rhai::EvalAltResult>) -> ScriptError {
    match *err {
        rhai::EvalAltResult::ErrorTooManyOperations(_) => ScriptError::InstructionLimitExceeded(
            "Execution operation limit exceeded (infinite loop prevented)".into(),
        ),
        other => ScriptError::RuntimeError(other.to_string()),
    }
}
