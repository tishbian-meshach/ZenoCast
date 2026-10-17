//! String template parsing, format specifier evaluation, and layer data-binding resolution.

use zeno_engine::{Composition, DataSnapshot, LayerContent};

use crate::store::DataStore;
use crate::value::DataValue;

/// Evaluates a template string replacing `{path | format}` placeholders with data values.
pub fn resolve_template(template: &str, store: &DataStore) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut tag = String::new();
            let mut closed = false;
            for next_ch in chars.by_ref() {
                if next_ch == '}' {
                    closed = true;
                    break;
                }
                tag.push(next_ch);
            }

            if closed {
                let formatted = resolve_tag(&tag, |path| store.get(path));
                result.push_str(&formatted);
            } else {
                result.push('{');
                result.push_str(&tag);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Evaluates a template string against a `DataSnapshot`.
pub fn resolve_template_from_snapshot(template: &str, snapshot: &DataSnapshot) -> String {
    let mut result = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut tag = String::new();
            let mut closed = false;
            for next_ch in chars.by_ref() {
                if next_ch == '}' {
                    closed = true;
                    break;
                }
                tag.push(next_ch);
            }

            if closed {
                let formatted =
                    resolve_tag(&tag, |path| snapshot.get(path).map(DataValue::from_json));
                result.push_str(&formatted);
            } else {
                result.push('{');
                result.push_str(&tag);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parses a single `{path | format}` tag and applies formatters.
fn resolve_tag<F>(tag: &str, lookup: F) -> String
where
    F: Fn(&str) -> Option<DataValue>,
{
    let parts: Vec<&str> = tag.split('|').map(|s| s.trim()).collect();
    let path = parts[0];
    let formatter = parts.get(1).copied();

    let val = match lookup(path) {
        Some(v) => v,
        None => return String::new(),
    };

    if let Some(fmt) = formatter {
        apply_formatter(&val, fmt)
    } else {
        val.to_display_string()
    }
}

/// Applies format specifiers like `number:0.0`, `duration:mm:ss`, `uppercase`.
fn apply_formatter(val: &DataValue, format_spec: &str) -> String {
    let (fmt_type, fmt_arg) = match format_spec.split_once(':') {
        Some((t, a)) => (t.trim(), a.trim()),
        None => (format_spec.trim(), ""),
    };

    match fmt_type {
        "number" => {
            if let Some(num) = val.as_f64() {
                if fmt_arg == "0.0" {
                    format!("{:.1}", num)
                } else if fmt_arg == "0.00" {
                    format!("{:.2}", num)
                } else if fmt_arg == "0" {
                    format!("{:.0}", num)
                } else {
                    num.to_string()
                }
            } else {
                val.to_display_string()
            }
        }
        "duration" => {
            if let Some(secs) = val.as_f64() {
                let total_secs = secs.max(0.0) as u64;
                let minutes = total_secs / 60;
                let seconds = total_secs % 60;
                if fmt_arg == "mm:ss" {
                    format!("{:02}:{:02}", minutes, seconds)
                } else {
                    format!("{}:{:02}", minutes, seconds)
                }
            } else {
                val.to_display_string()
            }
        }
        "uppercase" => val.to_display_string().to_uppercase(),
        "lowercase" => val.to_display_string().to_lowercase(),
        _ => val.to_display_string(),
    }
}

/// Resolves text templates in all text layers of a composition against a live data store.
/// Executed each frame to update bound layers within a single frame interval.
pub fn apply_bindings_to_composition(comp: &mut Composition, store: &DataStore) {
    for layer in &mut comp.layers {
        if let LayerContent::Text(ref mut text_content) = layer.content {
            let cur_text = text_content.text.fallback_value().clone();
            if cur_text.contains('{') && cur_text.contains('}') {
                let resolved = resolve_template(&cur_text, store);
                text_content.text = zeno_core::Property::constant(resolved);
            }
        }
    }
}
