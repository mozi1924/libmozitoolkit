// BSD 3-Clause License
//
// Copyright (c) 2024, Bram Stout Productions
// Upstream: https://github.com/BramStoutProductions/MiEx
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice, this
//    list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
//    this list of conditions and the following disclaimer in the documentation
//    and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its
//    contributors may be used to endorse or promote products derived from
//    this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

use std::collections::HashMap;
use crate::parser::blockstate::BlockState;
use crate::parser::model_json::FaceJson;

/// Evaluates standard Minecraft entity box UV mappings into standard face UV coordinates.
///
/// Ported from upstream MiEx `BuiltInModel.java`:
/// Given entity UV bounding box `[min_u, min_v, max_u, max_v]` and 3D bounds `[min_x, min_y, min_z, max_x, max_y, max_z]`:
pub fn entity_uv_to_faces(
    bounds: [f32; 6],
    entity_uv: [f32; 4],
    tex_ref: &str,
) -> HashMap<String, FaceJson> {
    let min_u = entity_uv[0];
    let min_v = entity_uv[1];
    let max_u = entity_uv[2];
    let max_v = entity_uv[3];

    let width = (bounds[3] - bounds[0]).abs();
    let height = (bounds[4] - bounds[1]).abs();
    let depth = (bounds[5] - bounds[2]).abs();

    let uv_span_w = max_u - min_u;
    let uv_span_h = max_v - min_v;

    let uv_w_unit = uv_span_w / (depth + width + depth + width).max(1e-5);
    let uv_h_unit = uv_span_h / (depth + height).max(1e-5);

    let mut faces = HashMap::new();

    // Directions: Down, Up, North, South, East, West
    let dir_uvs: [(&str, [f32; 4]); 6] = [
        // down: (depth + width) * uvWidth + minU, 0 * uvHeight + minV
        (
            "down",
            [
                (depth + width) * uv_w_unit + min_u,
                min_v,
                (depth + width + width) * uv_w_unit + min_u,
                depth * uv_h_unit + min_v,
            ],
        ),
        // up: depth * uvWidth + minU, 0 * uvHeight + minV
        (
            "up",
            [
                depth * uv_w_unit + min_u,
                min_v,
                (depth + width) * uv_w_unit + min_u,
                depth * uv_h_unit + min_v,
            ],
        ),
        // north: (depth + width + depth) * uvWidth + minU, depth * uvHeight + minV
        (
            "north",
            [
                (depth + width + depth) * uv_w_unit + min_u,
                depth * uv_h_unit + min_v,
                (depth + width + depth + width) * uv_w_unit + min_u,
                (depth + height) * uv_h_unit + min_v,
            ],
        ),
        // south: depth * uvWidth + minU, depth * uvHeight + minV
        (
            "south",
            [
                depth * uv_w_unit + min_u,
                depth * uv_h_unit + min_v,
                (depth + width) * uv_w_unit + min_u,
                (depth + height) * uv_h_unit + min_v,
            ],
        ),
        // east: (depth + width) * uvWidth + minU, depth * uvHeight + minV
        (
            "east",
            [
                (depth + width) * uv_w_unit + min_u,
                depth * uv_h_unit + min_v,
                (depth + width + depth) * uv_w_unit + min_u,
                (depth + height) * uv_h_unit + min_v,
            ],
        ),
        // west: 0 * uvWidth + minU, depth * uvHeight + minV
        (
            "west",
            [
                min_u,
                depth * uv_h_unit + min_v,
                depth * uv_w_unit + min_u,
                (depth + height) * uv_h_unit + min_v,
            ],
        ),
    ];

    for (dir, uv) in dir_uvs {
        faces.insert(
            dir.to_string(),
            FaceJson {
                uv: Some(uv),
                texture: tex_ref.to_string(),
                cullface: None,
                rotation: None,
                tintindex: None,
            },
        );
    }

    faces
}

/// Evaluates simple variable expressions like `thisBlock.name.contains('...')` or `'.../prefix' + var`.
pub fn eval_miex_string(
    expr: &str,
    vars: &HashMap<String, String>,
    blockstate: &BlockState,
) -> String {
    let trimmed = expr.trim();
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].to_string();
    }

    // Strip outer parentheses if balanced
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        let mut depth = 0;
        let mut all_enclosed = true;
        for (i, c) in trimmed.char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 && i < trimmed.len() - 1 {
                    all_enclosed = false;
                    break;
                }
            }
        }
        if all_enclosed {
            return eval_miex_string(&trimmed[1..trimmed.len() - 1], vars, blockstate);
        }
    }

    // Simple ternary check: condition ? val1 : val2
    // We need to find '?' that is not inside quotes or parentheses
    let mut qmark_pos = None;
    let mut depth = 0;
    let mut in_quote = false;
    for (i, c) in trimmed.char_indices() {
        if c == '\'' {
            in_quote = !in_quote;
        } else if !in_quote {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            } else if c == '?' && depth == 0 {
                qmark_pos = Some(i);
                break;
            }
        }
    }

    if let Some(pos) = qmark_pos {
        let cond_part = &trimmed[..pos];
        let rest = &trimmed[pos + 1..];

        // Find ':' that is not inside quotes or parentheses
        let mut colon_pos = None;
        let mut colon_depth = 0;
        let mut c_in_quote = false;
        for (i, c) in rest.char_indices() {
            if c == '\'' {
                c_in_quote = !c_in_quote;
            } else if !c_in_quote {
                if c == '(' {
                    colon_depth += 1;
                } else if c == ')' {
                    colon_depth -= 1;
                } else if c == ':' && colon_depth == 0 {
                    colon_pos = Some(i);
                    break;
                }
            }
        }

        if let Some(c_pos) = colon_pos {
            let true_val = &rest[..c_pos];
            let false_val = &rest[c_pos + 1..];
            let cond_res = eval_miex_condition(cond_part.trim(), vars, blockstate);
            if cond_res {
                return eval_miex_string(true_val.trim(), vars, blockstate);
            } else {
                return eval_miex_string(false_val.trim(), vars, blockstate);
            }
        }
    }

    // String concatenation with '+'
    // Check if contains '+' outside quotes/parens
    let mut plus_parts = Vec::new();
    let mut last_idx = 0;
    let mut plus_in_quote = false;
    let mut p_depth = 0;
    for (i, c) in trimmed.char_indices() {
        if c == '\'' {
            plus_in_quote = !plus_in_quote;
        } else if !plus_in_quote {
            if c == '(' {
                p_depth += 1;
            } else if c == ')' {
                p_depth -= 1;
            } else if c == '+' && p_depth == 0 {
                plus_parts.push(&trimmed[last_idx..i]);
                last_idx = i + 1;
            }
        }
    }

    if !plus_parts.is_empty() {
        plus_parts.push(&trimmed[last_idx..]);
        let mut result = String::new();
        for p in plus_parts {
            result.push_str(&eval_miex_string(p.trim(), vars, blockstate));
        }
        return result;
    }

    // Direct variable lookup
    if let Some(val) = vars.get(trimmed) {
        return val.clone();
    }

    // thisBlock.name
    if trimmed == "thisBlock.name" {
        return blockstate.name.clone();
    }

    // thisBlock.state.<prop>
    if let Some(prop) = trimmed.strip_prefix("thisBlock.state.") {
        return blockstate.properties.get(prop.trim()).cloned().unwrap_or_else(|| "null".to_string());
    }

    // String method: substring, indexOf, length
    // e.g. thisBlock.name.substring(thisBlock.name.indexOf(':') + 1)
    // or color.substring(0, color.length() - 3)
    if trimmed.contains(".substring(") {
        if let Some((target, sub_arg)) = trimmed.split_once(".substring(") {
            if let Some(arg_str) = sub_arg.strip_suffix(')') {
                let target_eval = eval_miex_string(target.trim(), vars, blockstate);
                let args: Vec<&str> = arg_str.split(',').collect();
                if args.len() == 1 {
                    let mut start_idx = 0usize;
                    let arg0 = args[0].trim();
                    if arg0.contains(".indexOf(':') + 1") {
                        start_idx = target_eval.find(':').map(|i| i + 1).unwrap_or(0);
                    } else if let Ok(s) = arg0.parse::<usize>() {
                        start_idx = s;
                    }
                    if start_idx <= target_eval.len() {
                        return target_eval[start_idx..].to_string();
                    }
                    return String::new();
                } else if args.len() == 2 {
                    let start_eval = eval_miex_string(args[0].trim(), vars, blockstate);
                    let start_idx = start_eval.parse::<usize>().unwrap_or(0);
                    let arg1 = args[1].trim();
                    let end_idx = if arg1.contains(".length() -") {
                        if let Some((_, minus_val)) = arg1.split_once('-') {
                            let m = minus_val.trim().parse::<usize>().unwrap_or(0);
                            target_eval.len().saturating_sub(m)
                        } else {
                            target_eval.len()
                        }
                    } else if let Ok(e) = arg1.parse::<usize>() {
                        e
                    } else {
                        target_eval.len()
                    };
                    let start_clamped = start_idx.min(target_eval.len());
                    let end_clamped = end_idx.clamp(start_clamped, target_eval.len());
                    return target_eval[start_clamped..end_clamped].to_string();
                }
            }
        }
    }

    trimmed.to_string()
}

/// Evaluates condition expressions from MiEx JSON.
pub fn eval_miex_condition(
    cond: &str,
    vars: &HashMap<String, String>,
    blockstate: &BlockState,
) -> bool {
    let mut trimmed = cond.trim();

    // Strip outer parentheses
    while trimmed.starts_with('(') && trimmed.ends_with(')') {
        let mut depth = 0;
        let mut all_enclosed = true;
        for (i, c) in trimmed.char_indices() {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
                if depth == 0 && i < trimmed.len() - 1 {
                    all_enclosed = false;
                    break;
                }
            }
        }
        if all_enclosed {
            trimmed = trimmed[1..trimmed.len() - 1].trim();
        } else {
            break;
        }
    }

    if trimmed == "true" {
        return true;
    }
    if trimmed == "false" || trimmed.is_empty() {
        return false;
    }

    // Logical OR ||
    if trimmed.contains("||") {
        let parts: Vec<&str> = trimmed.split("||").collect();
        return parts.iter().any(|p| eval_miex_condition(p.trim(), vars, blockstate));
    }

    // Logical AND &&
    if trimmed.contains("&&") {
        let parts: Vec<&str> = trimmed.split("&&").collect();
        return parts.iter().all(|p| eval_miex_condition(p.trim(), vars, blockstate));
    }

    if let Some(rest) = trimmed.strip_prefix('!') {
        return !eval_miex_condition(rest.trim(), vars, blockstate);
    }

    if let Some((left, right)) = trimmed.split_once("==") {
        let l = eval_miex_operand(left.trim(), vars, blockstate);
        let r = eval_miex_operand(right.trim(), vars, blockstate);
        return l == r;
    }

    if let Some((left, right)) = trimmed.split_once("!=") {
        let l = eval_miex_operand(left.trim(), vars, blockstate);
        let r = eval_miex_operand(right.trim(), vars, blockstate);
        return l != r;
    }

    if let Some((left, right)) = trimmed.split_once(">=") {
        let l = eval_miex_string(left.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        let r = eval_miex_string(right.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        return l >= r;
    }

    if let Some((left, right)) = trimmed.split_once("<=") {
        let l = eval_miex_string(left.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        let r = eval_miex_string(right.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        return l <= r;
    }

    if let Some((left, right)) = trimmed.split_once('>') {
        let l = eval_miex_string(left.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        let r = eval_miex_string(right.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        return l > r;
    }

    if let Some((left, right)) = trimmed.split_once('<') {
        let l = eval_miex_string(left.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        let r = eval_miex_string(right.trim(), vars, blockstate).parse::<f32>().unwrap_or(0.0);
        return l < r;
    }

    if trimmed.contains(".contains(") && trimmed.ends_with(')') {
        if let Some((target, rest)) = trimmed.split_once(".contains(") {
            let target_str = eval_miex_string(target.trim(), vars, blockstate);
            let arg = &rest[..rest.len() - 1].trim();
            let query = arg.trim_matches('\'');
            return target_str.contains(query);
        }
    }

    if trimmed.contains(".endsWith(") && trimmed.ends_with(')') {
        if let Some((target, rest)) = trimmed.split_once(".endsWith(") {
            let target_str = eval_miex_string(target.trim(), vars, blockstate);
            let arg = &rest[..rest.len() - 1].trim();
            let query = arg.trim_matches('\'');
            return target_str.ends_with(query);
        }
    }

    if let Some(val) = vars.get(trimmed) {
        return val == "true" || (!val.is_empty() && val != "false" && val != "0" && val != "null");
    }

    false
}

fn eval_miex_operand(
    operand: &str,
    vars: &HashMap<String, String>,
    blockstate: &BlockState,
) -> String {
    let trimmed = operand.trim();
    if trimmed == "null" {
        return "null".to_string();
    }
    if let Some(prop) = trimmed.strip_prefix("thisBlock.state.") {
        return blockstate.properties.get(prop).cloned().unwrap_or_else(|| "null".to_string());
    }
    eval_miex_string(trimmed, vars, blockstate)
}

/// Evaluates numerical expressions, e.g. `"-12.0 * scale + offsetX"`.
pub fn eval_miex_num(expr: &str, vars: &HashMap<String, f32>) -> f32 {
    let trimmed = expr.trim();
    if let Ok(val) = trimmed.parse::<f32>() {
        return val;
    }

    if let Some(&val) = vars.get(trimmed) {
        return val;
    }

    // Simple addition / subtraction
    if trimmed.contains('+') {
        let parts: Vec<&str> = trimmed.split('+').collect();
        return parts.iter().map(|p| eval_miex_num(p, vars)).sum();
    }

    // Simple multiplication
    if trimmed.contains('*') {
        let parts: Vec<&str> = trimmed.split('*').collect();
        return parts.iter().fold(1.0f32, |acc, p| acc * eval_miex_num(p, vars));
    }

    0.0
}
