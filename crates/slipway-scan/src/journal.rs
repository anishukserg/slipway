//! Журнал слоя работы при сборке реестра (решение 15).
//!
//! Скан сворачивает каталог журнала и порождает код: нарушение автомата —
//! `compile_error!` с именем файла события; событие о работе или срезе вне
//! плана — неразрешимый путь к константе; незавершённая работа в закрытом срезе
//! — ошибка вычисления константы. Состояние — `WORK_STATES` и `CLOSED_SLICES`.

use crate::{ScanError, ScannedDecision};
use slipway_journal::{fold, Event, Stage, Subject, Violation};
use std::{collections::BTreeSet, fmt::Write as _, path::Path};

/// Читает каталог журнала и порождает модуль состояния. Отсутствующий каталог
/// — ошибка: опечатка в пути не превращается в пустой журнал.
pub fn scan_journal(dir: &Path, work: &[ScannedDecision]) -> Result<String, ScanError> {
    let (events, violations) = slipway_journal::read_dir(dir)
        .map_err(|error| ScanError::Io(format!("{}: {error}", dir.display())))?;
    Ok(emit_journal(&events, &violations, work))
}

/// Порождает код состояния по событиям и нарушениям разбора.
pub fn emit_journal(
    events: &[Event],
    read_violations: &[Violation],
    work: &[ScannedDecision],
) -> String {
    let (journal, fold_violations) = fold(events);
    let mut out =
        String::from("// ПОРОЖДЕНО slipway-scan из каталога журнала. Не редактировать.\n\n");

    for violation in read_violations.iter().chain(&fold_violations) {
        let message = format!("журнал: {}: {}", violation.file, violation.reason);
        let _ = writeln!(out, "compile_error!({message:?});");
    }

    let subjects: BTreeSet<Subject> = events.iter().map(|event| event.subject).collect();
    if !subjects.is_empty() {
        out.push_str("\n// События ссылаются на план путями: предмет вне плана не разрешается.\n");
    }
    for subject in &subjects {
        let _ = match subject {
            Subject::Work(_) => writeln!(
                out,
                "const _: slipway_core::WorkRef = crate::work::{};",
                subject.id()
            ),
            Subject::Slice(_) => writeln!(
                out,
                "const _: slipway_core::SliceRef = crate::slice::{};",
                subject.id()
            ),
        };
    }

    out.push_str(
        "\n/// Состояние каждой единицы работы — свёртка журнала (решение 15).\npub static WORK_STATES: &[(slipway_core::WorkRef, slipway_work::WorkState)] = &[\n",
    );
    for entry in work {
        let _ = writeln!(
            out,
            "    (crate::work::{}, slipway_work::WorkState::{}),",
            entry.module,
            state_name(journal.stage(entry.id))
        );
    }
    out.push_str("];\n\n/// Срезы, закрытые событием журнала.\npub static CLOSED_SLICES: &[slipway_core::SliceRef] = &[\n");
    for slice in journal.closed_slices.keys() {
        let _ = writeln!(out, "    crate::slice::{},", Subject::Slice(*slice).id());
    }
    out.push_str("];\n");

    if !journal.closed_slices.is_empty() {
        out.push_str("\n// Закрытый срез не несёт незавершённой работы — вычисляет компилятор.\n");
    }
    for (slice, file) in &journal.closed_slices {
        for entry in work
            .iter()
            .filter(|entry| !journal.stage(entry.id).is_finished())
        {
            let message = format!(
                "журнал: срез s{slice:04} закрыт событием {file}, а работа {} не завершена",
                entry.module
            );
            let _ = writeln!(
                out,
                "const _: () = assert!(crate::{}::WORK.slice.index() != {slice}, {message:?});",
                entry.module
            );
        }
    }
    out
}

fn state_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Planned => "Planned",
        Stage::Started => "Started",
        Stage::Landed => "Landed",
        Stage::LandedFromHistory => "LandedFromHistory",
        Stage::Abandoned => "Abandoned",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Status;
    use slipway_journal::{Evidence, Kind};

    const TREE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn plan(ids: &[u32]) -> Vec<ScannedDecision> {
        ids.iter()
            .map(|id| ScannedDecision {
                id: *id,
                module: format!("w{id:04}"),
                status: Status::Draft,
                file: String::new(),
            })
            .collect()
    }

    fn event(subject: Subject, second: u32, kind: Kind) -> Event {
        Event::new(subject, format!("2026-09-11T03:15:{second:02}Z"), kind)
    }

    #[test]
    fn states_cover_the_plan_and_subjects_are_paths() {
        let events = [event(
            Subject::Work(1),
            1,
            Kind::Landed {
                commit: TREE.into(),
                tree: TREE.into(),
                evidence: Evidence::History,
            },
        )];
        let code = emit_journal(&events, &[], &plan(&[1, 2]));
        assert!(
            code.contains("const _: slipway_core::WorkRef = crate::work::w0001;"),
            "{code}"
        );
        assert!(
            code.contains("(crate::work::w0001, slipway_work::WorkState::LandedFromHistory),"),
            "{code}"
        );
        assert!(
            code.contains("(crate::work::w0002, slipway_work::WorkState::Planned),"),
            "{code}"
        );
        assert!(!code.contains("compile_error!"), "{code}");
    }

    #[test]
    fn violations_become_compile_errors_naming_the_event_file() {
        let events = [event(
            Subject::Work(1),
            1,
            Kind::Gate {
                gate: "commit".into(),
                tree: TREE.into(),
                verdict: "ok".into(),
            },
        )];
        let unreadable = Violation {
            file: "w0002/x.toml".into(),
            reason: "строка 1: ожидалось".into(),
        };
        let code = emit_journal(&events, &[unreadable], &plan(&[1, 2]));
        assert!(code.contains("compile_error!(\"журнал: w0001/20260911T031501Z-gate.toml: проверка работы w0001 до её начала\");"), "{code}");
        assert!(
            code.contains("compile_error!(\"журнал: w0002/x.toml: строка 1: ожидалось\");"),
            "{code}"
        );
    }

    #[test]
    fn closed_slice_asserts_every_unfinished_work() {
        let events = [
            event(Subject::Work(1), 1, Kind::Abandoned { reason: "x".into() }),
            event(Subject::Slice(3), 2, Kind::Closed),
        ];
        let code = emit_journal(&events, &[], &plan(&[1, 2]));
        assert!(code.contains("    crate::slice::s0003,"), "{code}");
        assert!(code.contains("assert!(crate::w0002::WORK.slice.index() != 3, \"журнал: срез s0003 закрыт событием s0003/20260911T031502Z-closed.toml, а работа w0002 не завершена\")"), "{code}");
        assert!(!code.contains("crate::w0001::WORK.slice"), "{code}");
    }
}
