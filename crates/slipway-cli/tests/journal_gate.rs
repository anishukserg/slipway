//! Сценарии журнала в калитке и в pre-commit (решение 15): хэш дерева без
//! журнала, неизменность файлов событий, сверка приземлений с историей и
//! повторное использование доказательства для коммита, меняющего только журнал.

mod common;

use common::TempRepo;
use std::fs;

const AT: &str = "2026-09-11T10:00:00Z";

/// Коммит всего рабочего дерева без хуков; возвращает sha.
fn committed(repo: &TempRepo, message: &str) -> String {
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "--no-verify", "-m", message]);
    repo.git(&["rev-parse", "HEAD"]).trim().to_owned()
}

/// Хэш дерева ревизии без журнала — через инструмент.
fn hash(repo: &TempRepo, revision: &str) -> String {
    let run = repo.tool(&["journal", "hash", revision]);
    assert_eq!(run.code, 0, "{}", run.output());
    run.stdout.trim().to_owned()
}

/// Путь и текст события started.
fn started(work: &str) -> (String, String) {
    (
        format!("doc/journal/{work}/20260911T100000Z-started.toml"),
        format!("event = \"started\"\nwork = \"{work}\"\nat = \"{AT}\"\n"),
    )
}

/// Попытка коммита с хуками; весь вывод.
fn attempt(repo: &TempRepo) -> String {
    let output = common::clean("git")
        .current_dir(&repo.root)
        .args(["commit", "-q", "-m", "попытка"])
        .output()
        .expect("git не запустился");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn tree_hash_ignores_only_the_journal() {
    let repo = TempRepo::new("journal-hash");
    repo.write("a.txt", "a\n");
    let base = committed(&repo, "база");
    let (path, text) = started("w0001");
    repo.write(&path, &text);
    let with_event = committed(&repo, "журнал");
    assert_eq!(hash(&repo, &base), hash(&repo, &with_event));

    repo.write("a.txt", "b\n");
    let changed = committed(&repo, "код");
    assert_ne!(hash(&repo, &with_event), hash(&repo, &changed));
}

#[test]
fn committed_event_file_cannot_change_or_disappear() {
    let repo = TempRepo::new("journal-append-only");
    let (path, text) = started("w0001");
    repo.write(&path, &text);
    committed(&repo, "журнал");

    // Контроль: нетронутый журнал проходит шаг, калитка доходит до манифеста.
    let run = repo.tool(&["gate"]);
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );

    repo.write(&path, &text.replace("started", "started "));
    let run = repo.tool(&["gate"]);
    assert!(
        run.stdout.contains("файл события изменён"),
        "{}",
        run.output()
    );
    assert!(
        run.verdict().starts_with("GATE FAIL: журнал расходится"),
        "{}",
        run.output()
    );

    fs::remove_file(repo.path(&path)).expect("файл удалён");
    let run = repo.tool(&["gate"]);
    assert!(
        run.stdout.contains("файл события удалён"),
        "{}",
        run.output()
    );
}

#[test]
fn landed_event_must_match_its_commit() {
    let repo = TempRepo::new("journal-landed");
    repo.write("a.txt", "a\n");
    let commit = committed(&repo, "работа");
    let right = hash(&repo, &commit);
    let path = "doc/journal/w0001/20260911T100000Z-landed.toml";
    let landed = |tree: &str, commit: &str| {
        format!(
            "event = \"landed\"\nwork = \"w0001\"\nat = \"{AT}\"\ncommit = \"{commit}\"\ntree = \"{tree}\"\nevidence = \"history\"\n"
        )
    };

    // Контроль: верное приземление проходит шаг.
    repo.write(path, &landed(&right, &commit));
    let run = repo.tool(&["gate"]);
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );

    repo.write(path, &landed(&"0".repeat(40), &commit));
    let run = repo.tool(&["gate"]);
    assert!(
        run.stdout.contains("расходится с деревом события"),
        "{}",
        run.output()
    );

    repo.write(path, &landed(&right, &"1".repeat(40)));
    let run = repo.tool(&["gate"]);
    assert!(run.stdout.contains("нет в репозитории"), "{}", run.output());
}

#[test]
fn journal_only_commit_reuses_the_proof_of_its_tree() {
    let repo = TempRepo::new("journal-proof-reuse");
    repo.write("a.txt", "a\n");
    let base = committed(&repo, "база");
    // Доказательство базового дерева — как после прохождения полной калитки.
    let proofs = repo.path(".git/slipway-proofs");
    fs::create_dir_all(&proofs).expect("каталог доказательств");
    fs::write(
        proofs.join(hash(&repo, &base)),
        format!("{AT}\nGATE OK (проба)\n"),
    )
    .expect("доказательство записано");
    repo.executable(
        "hooks/pre-commit",
        &format!("#!/bin/sh\nexec '{}' hook pre-commit\n", common::BIN),
    );
    repo.git(&["config", "core.hooksPath", "hooks"]);

    let (path, text) = started("w0001");
    repo.write(&path, &text);
    repo.git(&["add", &path]);
    let output = attempt(&repo);
    assert!(
        output.contains("дерево без журнала уже проверено"),
        "{output}"
    );
    assert!(
        output.contains("GATE FAIL: в дереве нет doc/Cargo.toml"),
        "{output}"
    );

    // Контроль: изменение вне журнала — полная калитка.
    repo.write("a.txt", "b\n");
    repo.git(&["add", "a.txt"]);
    let output = attempt(&repo);
    assert!(!output.contains("уже проверено"), "{output}");
    assert!(
        output.contains("GATE FAIL: в дереве нет Cargo.toml"),
        "{output}"
    );
}
