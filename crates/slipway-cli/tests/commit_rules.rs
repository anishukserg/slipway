//! Сценарии команд commit и msg-check (решение 8), перенесённые из самотеста
//! правил коммитов: каждая проверка обязана отвергать то, против чего написана,
//! а корректный коммит — проходить ровно с перечисленными путями.

mod common;

use common::{Run, TempRepo};
use std::fs;
use std::path::Path;

const OK: &str = "[FEAT](cli): новый файл и удаление старого\n\nSlipway-Work: w0001\n";

/// Репозиторий с таксономией, работой w0001, хуком commit-msg и базовым
/// коммитом, прошедшим этот хук.
fn planned_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.write(
        "doc/taxonomy.rs",
        "slipway_core::declare_taxonomy! {\n    Subsystem => [Knowledge, Cli],\n}\n",
    );
    repo.write("doc/work/w0001.rs", "work\n");
    repo.write("old/file.txt", "old\n");
    repo.write("unrelated.txt", "unrelated\n");
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

/// `commit -F <сообщение> <доп. аргументы> -- <пути>`.
fn commit(repo: &TempRepo, message: &str, extra: &[&str], paths: &[&str]) -> Run {
    let file = repo.outside("message.txt", message);
    let mut args = vec!["commit", "-F", file.to_str().expect("путь в UTF-8")];
    args.extend_from_slice(extra);
    args.push("--");
    args.extend_from_slice(paths);
    repo.tool(&args)
}

#[test]
fn malformed_message_is_refused_before_commit() {
    let repo = planned_repo("malformed-message");
    repo.write("new.txt", "new\n");
    let head = repo.git(&["rev-parse", "HEAD"]);
    for (label, message) in [
        ("тема без типа", "новый файл\n\nSlipway-Work: w0001\n"),
        (
            "тип вне набора",
            "[FEATURE](cli): новый файл\n\nSlipway-Work: w0001\n",
        ),
        (
            "область вне таксономии",
            "[FEAT](wal): новый файл\n\nSlipway-Work: w0001\n",
        ),
        (
            "точка в конце темы",
            "[FEAT](cli): новый файл.\n\nSlipway-Work: w0001\n",
        ),
        ("без основания", "[FEAT](cli): новый файл\n"),
    ] {
        let run = commit(&repo, message, &[], &["new.txt"]);
        assert_eq!(run.code, 4, "{label}: {}", run.output());
        assert!(
            run.verdict().starts_with("COMMIT REFUSED: "),
            "{label}: {}",
            run.output()
        );
    }
    assert_eq!(
        repo.git(&["rev-parse", "HEAD"]),
        head,
        "отвергнутое сообщение создало коммит"
    );
}

#[test]
fn work_outside_the_plan_is_refused_by_the_hook() {
    let repo = planned_repo("work-outside-plan");
    repo.write("new.txt", "new\n");
    let message = "[FEAT](cli): новый файл\n\nSlipway-Work: w0099\n";
    let run = commit(&repo, message, &[], &["new.txt"]);
    assert_eq!(run.code, 4, "{}", run.output());
    assert!(
        run.stderr.contains("единицы работы w0099"),
        "{}",
        run.output()
    );
    assert_eq!(
        repo.git(&["diff", "--cached", "--name-only"]),
        "",
        "отказ оставил пути в индексе"
    );
}

#[test]
fn log_takes_hook_output_and_terminal_keeps_refusal_lines() {
    let repo = planned_repo("refusal-log");
    repo.write("new.txt", "new\n");
    let log = repo.outside("commit.log", "");
    let message = "[FEAT](cli): новый файл\n\nSlipway-Work: w0099\n";
    let run = commit(
        &repo,
        message,
        &["--log", log.to_str().expect("путь в UTF-8")],
        &["new.txt"],
    );
    assert_eq!(run.code, 4, "{}", run.output());
    assert!(
        run.stdout.contains("  - единицы работы w0099"),
        "{}",
        run.output()
    );
    assert!(run.stdout.contains("полный вывод:"), "{}", run.output());
    assert!(
        run.verdict().starts_with("COMMIT REFUSED: "),
        "{}",
        run.output()
    );
    let logged = fs::read_to_string(&log).expect("журнал");
    assert!(logged.contains("единицы работы w0099"), "{logged}");
}

#[test]
fn paths_are_required() {
    let repo = planned_repo("paths-required");
    let run = commit(&repo, OK, &[], &[]);
    assert_eq!(run.code, 2, "{}", run.output());
    assert!(
        run.verdict().contains("пути не перечислены"),
        "{}",
        run.output()
    );
}

#[test]
fn nothing_to_commit_differs_from_refusal() {
    let repo = planned_repo("nothing-to-commit");
    let run = commit(&repo, OK, &[], &["doc"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.verdict().contains("нечего коммитить"),
        "{}",
        run.output()
    );
}

#[test]
fn commit_takes_exactly_the_listed_paths() {
    let repo = planned_repo("exact-paths");
    repo.write("new.txt", "new\n");
    repo.write("unrelated.txt", "unrelated\nchanged\n");
    fs::remove_dir_all(repo.path("old")).expect("old удалён");
    let run = commit(&repo, OK, &[], &["new.txt", "old"]);
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(run.verdict().starts_with("COMMIT OK "), "{}", run.output());
    let mut changed: Vec<String> = repo
        .git(&["show", "--name-status", "--format=", "HEAD"])
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    changed.sort();
    assert_eq!(changed, ["A\tnew.txt", "D\told/file.txt"]);
    assert_eq!(
        repo.git_code(&["diff", "--quiet", "--", "unrelated.txt"]),
        1,
        "чужое изменение пропало из рабочего дерева"
    );
}

#[test]
fn held_lock_waits_and_times_out() {
    let repo = planned_repo("held-lock");
    repo.write("new.txt", "new\n");
    let lock = repo.path(".git/slipway-commit.lock");
    fs::write(&lock, format!("{}\n", std::process::id())).expect("блокировка записана");
    let run = commit(&repo, OK, &["--timeout", "1"], &["new.txt"]);
    assert_eq!(run.code, 3, "{}", run.output());
    assert!(
        run.verdict().contains("блокировка коммита не получена"),
        "{}",
        run.output()
    );
    assert!(lock.exists(), "ожидающий снял живую блокировку");
}

#[test]
fn lock_of_a_dead_process_is_taken_over() {
    // Без /proc смерть держателя не определяется: блокировка ждёт тайм-аута, и
    // проверять здесь нечего.
    if !Path::new("/proc/self").exists() {
        return;
    }
    let repo = planned_repo("dead-lock");
    repo.write("new.txt", "new\n");
    let lock = repo.path(".git/slipway-commit.lock");
    fs::write(&lock, format!("{}\n", u32::MAX)).expect("блокировка записана");
    let run = commit(&repo, OK, &["--timeout", "5"], &["new.txt"]);
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(!lock.exists(), "блокировка осталась после коммита");
}
