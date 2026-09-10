//! Сценарии команд work и slice (решение 15): события пишутся и коммитятся
//! инструментом, незаконный переход и приземление без доказательства
//! отвергаются до записи события.

mod common;

use common::{Run, TempRepo};
use std::fs;

/// Репозиторий с таксономией, срезом s0001, работами w0001 и w0002 в нём,
/// каталогом журнала и хуком commit-msg.
fn planned_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.write(
        "doc/taxonomy.rs",
        "slipway_core::declare_taxonomy! {\n    Subsystem => [Cli, Work],\n}\n",
    );
    repo.write(
        "doc/slice/s0001.rs",
        "slipway_work::slice!(1,\n    title: NonEmptyStr::new(\"Первый срез\"),\n);\n",
    );
    for (id, title) in [("w0001", "Первая работа"), ("w0002", "Вторая работа")]
    {
        repo.write(
            &format!("doc/work/{id}.rs"),
            &format!(
                "slipway_work::work!(1,\n    title: NonEmptyStr::new(\"{title}\"),\n    slice: crate::slice::s0001,\n    taxon: taxon!(Subsystem, Cli),\n);\n"
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
        "[CHORE](cli): база",
        "-m",
        "Slipway-Work: w0001",
    ]);
    repo
}

/// Доказательство для дерева HEAD — как после прохождения калитки.
fn prove_head(repo: &TempRepo) {
    let hash = repo
        .tool(&["journal", "hash", "HEAD"])
        .stdout
        .trim()
        .to_owned();
    let proofs = repo.path(".git/slipway-proofs");
    fs::create_dir_all(&proofs).expect("каталог доказательств");
    fs::write(proofs.join(hash), "2026-09-11T10:00:00Z\nGATE OK (проба)\n")
        .expect("доказательство записано");
}

fn state_of(repo: &TempRepo, id: &str) -> String {
    repo.tool(&["work", "state", id]).stdout
}

fn assert_ok(run: &Run) {
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(run.verdict().starts_with("COMMIT OK "), "{}", run.output());
}

#[test]
fn start_land_drop_and_close_write_events_and_commit_them() {
    let repo = planned_repo("work-lifecycle");
    let run = repo.tool(&[
        "work",
        "start",
        "w0001",
        "--trailer",
        "Co-Authored-By: Проба <proba@localhost>",
    ]);
    assert_ok(&run);
    assert!(state_of(&repo, "w0001").contains("начата"));
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert!(
        body.starts_with("[PLAN](cli): начата работа w0001"),
        "{body}"
    );
    assert!(
        body.contains("Slipway-Work: w0001\nCo-Authored-By: Проба"),
        "{body}"
    );

    // Без доказательства для дерева коммита работы приземление отвергается.
    let run = repo.tool(&["work", "land", "w0001"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.verdict().contains("нет доказательства"),
        "{}",
        run.output()
    );

    prove_head(&repo);
    let run = repo.tool(&["work", "land", "w0001"]);
    assert_ok(&run);
    let files = repo.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert!(
        files.contains("-gate.toml") && files.contains("-landed.toml"),
        "{files}"
    );
    assert!(state_of(&repo, "w0001").contains("приземлена"));

    // Срез не закрывается, пока в нём есть незавершённая работа.
    let run = repo.tool(&["slice", "close", "s0001"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.verdict().contains("не завершены w0002"),
        "{}",
        run.output()
    );

    assert_ok(&repo.tool(&["work", "drop", "w0002", "--reason", "замещена"]));
    assert!(state_of(&repo, "w0002").contains("снята"));
    assert_ok(&repo.tool(&["slice", "close", "s0001"]));
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert!(body.contains("Slipway-Slice: s0001"), "{body}");
    assert!(repo
        .tool(&["work", "state"])
        .stdout
        .contains("закрытые срезы: s0001"));
}

#[test]
fn illegal_requests_are_refused_before_any_event() {
    let repo = planned_repo("work-illegal");
    let head = repo.git(&["rev-parse", "HEAD"]);
    for (args, code, reason) in [
        (vec!["work", "land", "w0001"], 1, "только начатую"),
        (vec!["work", "start", "w0009"], 1, "нет в плане"),
        (
            vec!["work", "drop", "w0001", "--reason", " "],
            2,
            "непустая причина",
        ),
        (vec!["slice", "close", "s0009"], 1, "нет в плане"),
        (vec!["work", "begin", "w0001"], 2, "work start"),
    ] {
        let run = repo.tool(&args);
        assert_eq!(run.code, code, "{args:?}: {}", run.output());
        assert!(run.verdict().contains(reason), "{args:?}: {}", run.output());
    }
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        head,
        "отказ создал коммит"
    );

    assert_ok(&repo.tool(&["work", "start", "w0001"]));
    let run = repo.tool(&["work", "start", "w0001"]);
    assert!(run.verdict().contains("уже начата"), "{}", run.output());

    // Коммит без трейлера этой работы не приземляет её, даже с доказательством.
    repo.write("x.txt", "x\n");
    repo.git(&["add", "x.txt"]);
    repo.git(&[
        "commit",
        "-q",
        "-m",
        "[CHORE](cli): чужая работа",
        "-m",
        "Slipway-Work: w0002",
    ]);
    prove_head(&repo);
    let run = repo.tool(&["work", "land", "w0001"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.verdict().contains("не основан на работе w0001"),
        "{}",
        run.output()
    );
    assert_eq!(
        repo.git(&["status", "--short", "doc/journal"]),
        "",
        "отказ оставил файлы журнала"
    );
}
