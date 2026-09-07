use std::io::{self, Read, Write};
use glam::IVec3;
use mtk_core::direction::Direction;
use mtk_cull::{FaceCuller, GlassCullMode, LeavesCullMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct TestCase {
    state: String,
    neighbor: Option<String>,
    direction: String,
    leaves_mode: String,
    glass_mode: String,
    pos_a: Option<[i32; 3]>,
    pos_b: Option<[i32; 3]>,
}

#[derive(Debug, Serialize)]
struct TestResult {
    render: bool,
}

fn main() {
    let mut input_str = String::new();
    io::stdin().read_to_string(&mut input_str).unwrap();

    let test_cases: Vec<TestCase> = serde_json::from_str(&input_str).unwrap();
    let mut results: Vec<TestResult> = Vec::with_capacity(test_cases.len());

    let culler = FaceCuller::default();

    for tc in test_cases {
        let leaves_mode = match tc.leaves_mode.as_str() {
            "FANCY" => LeavesCullMode::Fancy,
            "FAST" => LeavesCullMode::Fast,
            "NONE" => LeavesCullMode::None,
            _ => LeavesCullMode::SingleFace,
        };

        let glass_mode = match tc.glass_mode.as_str() {
            "SAME_BLOCK" => GlassCullMode::SameBlock,
            "NONE" => GlassCullMode::None,
            _ => GlassCullMode::Group,
        };

        let mut instance = culler.clone();
        instance.leaves_cull_mode = leaves_mode;
        instance.glass_cull_mode = glass_mode;

        let state_meta = instance.get_meta(&tc.state, None, None);
        let neighbor_meta = tc.neighbor.as_ref().map(|n| instance.get_meta(n, None, None));

        let dir = Direction::parse_loose(&tc.direction).unwrap_or(Direction::East);
        let pos_a = tc.pos_a.map(|p| IVec3::new(p[0], p[1], p[2]));
        let pos_b = tc.pos_b.map(|p| IVec3::new(p[0], p[1], p[2]));

        let render = instance.should_render_face(
            &state_meta,
            neighbor_meta.as_deref(),
            dir,
            None,
            pos_a,
            pos_b,
        );

        results.push(TestResult { render });
    }

    let output_json = serde_json::to_string(&results).unwrap();
    io::stdout().write_all(output_json.as_bytes()).unwrap();
}
