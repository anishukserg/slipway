//! Скан документов Slipway: решения, спецификации, направления, срезы,
//! единицы работы и журнал. Пишет только в OUT_DIR.
//!
//! Типы документов перечислены здесь явно. Отсутствующий каталог
//! перечисленного типа — ошибка сборки, а не пустой реестр: опечатка в пути
//! не превращается в реестр без записей (решение 11).

use slipway_scan::plan::{emit_plan, emit_work_checks, scan_plan, SLICES, THRUSTS, WORK};
use slipway_scan::ScanError;
use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let decisions = unwrap_scan(slipway_scan::scan_decisions(&manifest.join("adr")));
    fs::write(out.join("adr.rs"), slipway_scan::emit_refs(&decisions)).unwrap();

    let specs = unwrap_scan(slipway_scan::scan_specs(&manifest.join("rfc")));
    fs::write(out.join("rfc.rs"), slipway_scan::emit_spec_refs(&specs)).unwrap();

    let thrusts = unwrap_scan(scan_plan(&manifest.join("thrust"), &THRUSTS));
    let slices = unwrap_scan(scan_plan(&manifest.join("slice"), &SLICES));
    let work = unwrap_scan(scan_plan(&manifest.join("work"), &WORK));
    let mut plan = emit_plan(&thrusts, &THRUSTS);
    plan.push_str(&emit_plan(&slices, &SLICES));
    plan.push_str(&emit_plan(&work, &WORK));
    plan.push_str(&emit_work_checks(&work));
    fs::write(out.join("plan.rs"), plan).unwrap();

    // Журнал сворачивается при сборке (решение 15): нарушение автомата не
    // собирается и называет файл события.
    let journal = unwrap_scan(slipway_scan::journal::scan_journal(
        &manifest.join("journal"),
        &work,
    ));
    fs::write(out.join("journal.rs"), journal).unwrap();

    for dir in ["adr", "rfc", "thrust", "slice", "work", "journal"] {
        println!("cargo::rerun-if-changed={dir}");
    }
}

fn unwrap_scan<T>(r: Result<T, ScanError>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => {
            println!("cargo::error=slipway: {e}");
            std::process::exit(1);
        }
    }
}
