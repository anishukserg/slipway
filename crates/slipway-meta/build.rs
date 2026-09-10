//! Скан собственного реестра. Slipway ведёт свои решения по своим же правилам.

use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let decisions = unwrap_scan(slipway_scan::scan_decisions(&manifest.join("src/adr")));
    fs::write(out.join("adr.rs"), slipway_scan::emit_refs(&decisions)).unwrap();

    let specs = unwrap_scan(slipway_scan::scan_specs(&manifest.join("src/rfc")));
    fs::write(out.join("rfc.rs"), slipway_scan::emit_spec_refs(&specs)).unwrap();

    println!("cargo::rerun-if-changed=src/adr");
    println!("cargo::rerun-if-changed=src/rfc");
    println!(
        "cargo::warning=slipway-meta: решений {}, спецификаций {}",
        decisions.len(),
        specs.len()
    );
}

fn unwrap_scan<T>(r: Result<T, slipway_scan::ScanError>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => {
            println!("cargo::error=slipway: {e}");
            std::process::exit(1);
        }
    }
}
