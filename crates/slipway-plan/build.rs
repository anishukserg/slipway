//! Скан плана Slipway: направления, срезы, единицы работы.

use slipway_scan::plan::{emit_plan, emit_work_checks, scan_plan, PlanKind, SLICES, THRUSTS, WORK};
use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let scan = |dir: &str, kind: &PlanKind| match scan_plan(&manifest.join(dir), kind) {
        Ok(entries) => entries,
        Err(e) => {
            println!("cargo::error=slipway: {e}");
            std::process::exit(1);
        }
    };
    let thrusts = scan("src/thrust", &THRUSTS);
    let slices = scan("src/slice", &SLICES);
    let work = scan("src/work", &WORK);

    let mut code = emit_plan(&thrusts, &THRUSTS);
    code.push_str(&emit_plan(&slices, &SLICES));
    code.push_str(&emit_plan(&work, &WORK));
    code.push_str(&emit_work_checks(&work));
    fs::write(out.join("plan.rs"), code).unwrap();

    for dir in ["src/thrust", "src/slice", "src/work"] {
        println!("cargo::rerun-if-changed={dir}");
    }
}
