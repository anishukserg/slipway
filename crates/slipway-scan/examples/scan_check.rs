fn main() {
    let anchors = slipway_scan::scan_anchors(&[std::path::Path::new("examples/demo-product/src")]).unwrap();
    for a in &anchors {
        println!("{:<20} {}:{}-{} mode={}", a.id, a.file, a.line_start, a.line_end, a.mode.as_str());
    }
    println!("всего: {}", anchors.len());
}
