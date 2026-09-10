//! Скан реестра решений и порождение модуля констант в OUT_DIR.
//!
//! Пишет ТОЛЬКО в OUT_DIR: запись в каталог крейта ломала бы
//! `cargo package --locked` и обновляла бы mtime собственного триггера
//! перезапуска.

use std::{env, fs, path::PathBuf};

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let adr_dir = manifest.join("adr");

    let decisions = match slipway_scan::scan_decisions(&adr_dir) {
        Ok(d) => d,
        Err(e) => {
            println!("cargo::error=slipway: {e}");
            std::process::exit(1);
        }
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("registry.rs"), slipway_scan::emit_refs(&decisions)).unwrap();

    // Разметка кода продукта: порождает константы, на которые ссылаются решения.
    let product_src = manifest.join("../demo-product/src");
    let anchors = match slipway_scan::scan_anchors(&[&product_src]) {
        Ok(a) => a,
        Err(e) => {
            println!("cargo::error=slipway: {e}");
            std::process::exit(1);
        }
    };
    fs::write(out_dir.join("anchors.rs"), slipway_scan::anchors::emit_anchor_refs(&anchors)).unwrap();

    println!("cargo::rerun-if-changed=adr");
    println!("cargo::rerun-if-changed=../demo-product/src");
    println!(
        "cargo::warning=slipway: решений {}, размеченных фрагментов {}",
        decisions.len(),
        anchors.len()
    );
}
