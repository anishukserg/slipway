//! Сценарии импорта истории в журнал (решение 15): работа с коммитом по
//! трейлеру приземляется из истории, работа без коммита остаётся
//! запланированной, основание импорта не импортируется, а срез, все работы
//! которого завершены, закрывается по флагу.

mod common;

use common::TempRepo;

/// Репозиторий с работами w0001 (коммит по трейлеру), w0002 (без коммита) и
/// w0003 — основанием импорта; срезы s0001 = {w0001}, s0002 = {w0002, w0003}.
fn history_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.write(
        "doc/taxonomy.rs",
        "slipway_core::declare_taxonomy! {\n    Subsystem => [Cli, Work],\n}\n",
    );
    for (id, title) in [("s0001", "Первый срез"), ("s0002", "Второй срез")] {
        repo.write(
            &format!("doc/slice/{id}.rs"),
            &format!("slipway_work::slice!(1,\n    title: NonEmptyStr::new(\"{title}\"),\n);\n"),
        );
    }
    for (id, slice) in [("w0001", "s0001"), ("w0002", "s0002"), ("w0003", "s0002")] {
        repo.write(
            &format!("doc/work/{id}.rs"),
            &format!(
                "slipway_work::work!(1,\n    title: NonEmptyStr::new(\n        \"Работа {id}\"\n    ),\n    slice: crate::slice::{slice},\n    taxon: taxon!(Subsystem, Work),\n);\n"
            ),
        );
    }
    repo.write("doc/journal/README.md", "журнал\n");
    repo.executable(
        "hooks/commit-msg",
        &format!("#!/bin/sh\nexec '{}' hook commit-msg \"$1\"\n", common::BIN),
    );
    repo.git(&["config", "core.hooksPath", "hooks"]);
    repo.git(&["add", "-A"]);
    repo.git(&[
        "commit",
        "-q",
        "-m",
        "[FEAT](work): первая работа",
        "-m",
        "Slipway-Work: w0001",
    ]);
    repo.write("code.txt", "код основания\n");
    repo.git(&["add", "code.txt"]);
    repo.git(&[
        "commit",
        "-q",
        "-m",
        "[FEAT](work): импорт",
        "-m",
        "Slipway-Work: w0003",
    ]);
    repo
}

#[test]
fn history_lands_traced_works_and_closes_finished_slices() {
    let repo = history_repo("journal-import");
    let first = repo
        .git(&["rev-list", "--max-parents=0", "HEAD"])
        .trim()
        .to_owned();

    let run = repo.tool(&[
        "journal",
        "import",
        "--work",
        "w0003",
        "--close-finished-slices",
    ]);
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(run.verdict().starts_with("COMMIT OK "), "{}", run.output());

    let state = repo.tool(&["work", "state"]).stdout;
    assert!(state.contains("w0001  landed from history"), "{state}");
    assert!(state.contains("w0002  planned"), "{state}");
    assert!(
        state.contains("w0003  planned"),
        "основание импортировано: {state}"
    );
    assert!(state.contains("closed slices: s0001"), "{state}");
    assert!(
        state.contains("Работа w0001"),
        "название с переносом строки не прочитано: {state}"
    );

    let landed = repo.git(&["show", "HEAD:doc/journal", "--name-only"]);
    assert!(landed.contains("w0001/"), "{landed}");
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert!(body.contains("Slipway-Work: w0003"), "{body}");
    let event = repo.git(&["grep", "-h", "commit =", "HEAD", "--", "doc/journal/w0001"]);
    assert!(
        event.contains(&first),
        "приземление не на коммите с трейлером: {event}"
    );

    let run = repo.tool(&["journal", "import", "--work", "w0003"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.verdict().contains("nothing to import"),
        "{}",
        run.output()
    );
}
