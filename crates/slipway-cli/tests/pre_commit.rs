//! Сценарии pre-commit (решение 8): калитка проверяет дерево коммита, а не
//! рабочее дерево, а выгрузка дерева сохраняет неизменённые файлы и убирает
//! удалённые.

mod common;

use common::TempRepo;
use std::fs;
use std::time::Duration;

/// Репозиторий с хуком pre-commit, вызывающим инструмент, и списком внешних
/// имён.
fn repo_with_pre_commit(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.executable(
        "hooks/pre-commit",
        &format!("#!/bin/sh\nexec '{}' hook pre-commit\n", common::BIN),
    );
    repo.git(&["config", "core.hooksPath", "hooks"]);
    fs::write(
        repo.path(".git/info/slipway-external-names"),
        "zzvneshniy\n",
    )
    .expect("список записан");
    repo
}

/// Попытка коммита; возвращает последнюю строку калитки из вывода хука.
fn attempt(repo: &TempRepo) -> String {
    let output = common::clean("git")
        .current_dir(&repo.root)
        .args(["commit", "-q", "-m", "попытка"])
        .output()
        .expect("git не запустился");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    text.lines()
        .rev()
        .find(|line| line.starts_with("GATE "))
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn gate_sees_the_commit_tree_not_the_working_tree() {
    let repo = repo_with_pre_commit("pre-commit-tree");
    repo.write("clean.txt", "чисто\n");
    repo.write("leak.txt", "след ZZVneshniy\n");
    repo.git(&["add", "clean.txt"]);

    // След есть только в рабочем дереве: калитка проходит внешние имена и
    // останавливается на отсутствующем манифесте.
    assert_eq!(
        attempt(&repo),
        "GATE FAIL: no Cargo.toml in the tree — the build does not start"
    );

    // Тот же след в индексе — отказ на внешних именах.
    repo.git(&["add", "leak.txt"]);
    assert_eq!(
        attempt(&repo),
        "GATE FAIL: external names in the tree (decision 9)"
    );
}

#[test]
fn exported_tree_keeps_unchanged_files_and_drops_deleted_ones() {
    let repo = repo_with_pre_commit("pre-commit-sync");
    repo.write("kept.txt", "без изменений\n");
    repo.write("gone.txt", "будет удалён\n");
    repo.git(&["add", "-A"]);
    attempt(&repo);

    let tree = repo.path(".git/slipway-gate/tree");
    let kept = tree.join("kept.txt");
    let exported_at = fs::metadata(&kept)
        .and_then(|meta| meta.modified())
        .expect("kept.txt выгружен");
    assert!(tree.join("gone.txt").exists(), "gone.txt не выгружен");

    // Пауза делает перезапись файла заметной по времени изменения.
    std::thread::sleep(Duration::from_millis(50));
    // Файл добавлен в индекс, но не закоммичен: без -f git rm отказывает.
    repo.git(&["rm", "-q", "-f", "gone.txt"]);
    attempt(&repo);

    let again = fs::metadata(&kept)
        .and_then(|meta| meta.modified())
        .expect("kept.txt на месте");
    assert_eq!(again, exported_at, "неизменённый файл перезаписан");
    assert!(
        !tree.join("gone.txt").exists(),
        "удалённый файл остался в дереве коммита"
    );
}
