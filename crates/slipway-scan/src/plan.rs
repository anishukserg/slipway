//! Скан реестров слоя работы: направления, срезы, единицы работы.
//!
//! Устроен как скан решений: один документ — один файл с каноническим
//! именем, константа-ссылка на каждый, сверка идентификатора с компилятором.
//! Сверх того порождается правило, связывающее две записи: радиус работы не
//! выше потолка её среза. Оно вычисляется в `const`, поэтому нарушение —
//! ошибка компиляции, а не находка валидатора.

use crate::{ScanError, ScannedDecision};
use std::{fmt::Write as _, path::Path};

/// Вид реестра слоя работы и имена того, что для него порождается.
pub struct PlanKind {
    macro_name: &'static str,
    prefix: char,
    module: &'static str,
    reference: &'static str,
    record: &'static str,
    item: &'static str,
    all: &'static str,
}

pub const THRUSTS: PlanKind = PlanKind {
    macro_name: "thrust",
    prefix: 't',
    module: "thrust",
    reference: "ThrustRef",
    record: "Thrust",
    item: "THRUST",
    all: "ALL_THRUSTS",
};

pub const SLICES: PlanKind = PlanKind {
    macro_name: "slice",
    prefix: 's',
    module: "slice",
    reference: "SliceRef",
    record: "Slice",
    item: "SLICE",
    all: "ALL_SLICES",
};

pub const WORK: PlanKind = PlanKind {
    macro_name: "work",
    prefix: 'w',
    module: "work",
    reference: "WorkRef",
    record: "WorkItem",
    item: "WORK",
    all: "ALL_WORK",
};

/// Сканирует каталог реестра одного вида.
pub fn scan_plan(dir: &Path, kind: &PlanKind) -> Result<Vec<ScannedDecision>, ScanError> {
    crate::scan_dir(dir, kind.macro_name, kind.prefix)
}

/// Порождает модули файлов, модуль констант-ссылок, сверку идентификаторов
/// и список записей вида.
pub fn emit_plan(entries: &[ScannedDecision], kind: &PlanKind) -> String {
    let mut out = String::from("// ПОРОЖДЕНО slipway-scan. Не редактировать.\n\n");
    for e in entries {
        let _ = writeln!(out, "#[path = {:?}]\npub mod {};", e.file, e.module);
    }

    let _ = writeln!(
        out,
        "\n#[allow(non_upper_case_globals)]\npub mod {} {{\n    use slipway_core::{};",
        kind.module, kind.reference
    );
    for e in entries {
        let _ = writeln!(
            out,
            "    pub const {m}: {r} = {r}::__from_scan({id});",
            m = e.module,
            r = kind.reference,
            id = e.id
        );
    }
    out.push_str("}\n\n");

    for e in entries {
        let _ = writeln!(
            out,
            "const _: () = assert!({m}::{item}.id == {id}, \"slipway-scan: у {m} идентификатор разошёлся со сканом\");",
            m = e.module,
            item = kind.item,
            id = e.id
        );
    }

    let _ = writeln!(out, "\npub static {}: &[&slipway_work::{}] = &[", kind.all, kind.record);
    for e in entries {
        let _ = writeln!(out, "    &{}::{},", e.module, kind.item);
    }
    out.push_str("];\n\n");
    out
}

/// Утверждения «радиус работы не выше потолка среза». Порождаются после
/// списков срезов и работ: ссылаются на `ALL_SLICES`.
pub fn emit_work_checks(work: &[ScannedDecision]) -> String {
    let mut out = String::from("// Радиус работы не выше потолка её среза — вычисляет компилятор.\n");
    for w in work {
        let _ = writeln!(
            out,
            "const _: () = assert!(slipway_work::radius_within_slice(&{m}::WORK, ALL_SLICES), \"slipway: радиус {m} выше потолка его среза\");",
            m = w.module
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Status;
    use std::fs;

    fn registry(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("slipway-plan-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            fs::write(dir.join(name), text).unwrap();
        }
        dir
    }

    const WORK_1: &str = "slipway_work::work!(1, title: NonEmptyStr::new(\"x\"),);";

    #[test]
    fn scans_work_with_canonical_names() {
        let dir = registry("ok", &[("w0001.rs", WORK_1)]);
        let found = scan_plan(&dir, &WORK).unwrap();
        assert_eq!(found[0].module, "w0001");
    }

    #[test]
    fn rejects_non_canonical_work_file() {
        let dir = registry("bad", &[("w1.rs", WORK_1)]);
        let err = scan_plan(&dir, &WORK).expect_err("w1.rs принят");
        assert!(err.to_string().contains("w0001.rs"), "{err}");
    }

    #[test]
    fn emits_refs_list_and_radius_checks() {
        let entries = vec![ScannedDecision {
            id: 1,
            module: "w0001".into(),
            status: Status::Draft,
            file: "/x/w0001.rs".into(),
        }];
        let code = emit_plan(&entries, &WORK);
        assert!(code.contains("pub const w0001: WorkRef = WorkRef::__from_scan(1);"), "{code}");
        assert!(code.contains("pub static ALL_WORK: &[&slipway_work::WorkItem]"), "{code}");
        assert!(emit_work_checks(&entries).contains("radius_within_slice(&w0001::WORK, ALL_SLICES)"));
    }
}
