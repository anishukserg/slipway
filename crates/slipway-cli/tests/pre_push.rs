//! Сценарии pre-push (решения 9 и 13), перенесённые из самотеста правил
//! коммитов, и новые: неизвестная удалённая вершина и несколько ссылок в одном
//! вводе.

mod common;

use common::{Run, TempRepo};

const ZERO: &str = "0000000000000000000000000000000000000000";

/// Репозиторий с чистым базовым коммитом, списком внешних имён и коммитом со
/// следом имени поверх. Возвращает репозиторий, чистую и «грязную» вершины.
fn history_with_leak(name: &str) -> (TempRepo, String, String) {
    let repo = TempRepo::new(name);
    repo.write("clean.txt", "чисто\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "база"]);
    let clean = repo.git(&["rev-parse", "HEAD"]).trim().to_owned();
    std::fs::write(
        repo.path(".git/info/slipway-external-names"),
        "zzvneshniy\n",
    )
    .expect("список записан");
    repo.write("leak.txt", "след ZZVneshniy\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "след"]);
    let leak = repo.git(&["rev-parse", "HEAD"]).trim().to_owned();
    (repo, clean, leak)
}

/// pre-push со строками `<ссылка> <локальный sha> <ссылка> <удалённый sha>`.
fn push(repo: &TempRepo, refs: &[(&str, &str, &str)]) -> Run {
    let input: String = refs
        .iter()
        .map(|(reference, local, remote)| format!("{reference} {local} {reference} {remote}\n"))
        .collect();
    repo.tool_with_input(&["hook", "pre-push", "origin", "selftest"], &input)
}

#[test]
fn history_with_an_external_name_is_not_published() {
    let (repo, _, leak) = history_with_leak("push-leak");
    let run = push(&repo, &[("refs/heads/master", leak.as_str(), ZERO)]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stderr.contains("содержит внешнее имя"),
        "{}",
        run.output()
    );
    assert!(run.stderr.contains("  файл: leak.txt"), "{}", run.output());
    assert!(run.stderr.contains("PUSH REFUSED"), "{}", run.output());
}

#[test]
fn archive_branch_is_not_published() {
    let (repo, clean, _) = history_with_leak("push-archive");
    let run = push(&repo, &[("refs/heads/archive/old", clean.as_str(), ZERO)]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(run.stderr.contains("ветка архива"), "{}", run.output());
}

#[test]
fn clean_history_is_published() {
    let (repo, clean, _) = history_with_leak("push-clean");
    let run = push(&repo, &[("refs/heads/master", clean.as_str(), ZERO)]);
    assert_eq!(run.code, 0, "{}", run.output());
    // Во временном репозитории нет Cargo.toml: шаг зависимостей назван
    // невыполненным, а не пройден молча.
    assert!(
        run.stderr.contains("зависимости не проверялись"),
        "{}",
        run.output()
    );
}

#[test]
fn unknown_remote_tip_checks_the_whole_history() {
    // Удалённой вершины нет локально: диапазон не строится, и это не должно
    // превращаться в «нечего проверять».
    let (repo, _, leak) = history_with_leak("push-unknown-remote");
    let unknown = "1111111111111111111111111111111111111111";
    let run = push(&repo, &[("refs/heads/master", leak.as_str(), unknown)]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stderr.contains("содержит внешнее имя"),
        "{}",
        run.output()
    );
}

#[test]
fn every_reference_in_the_input_is_checked() {
    let (repo, clean, _) = history_with_leak("push-several");
    let run = push(
        &repo,
        &[
            ("refs/heads/master", clean.as_str(), ZERO),
            ("refs/heads/archive/old", clean.as_str(), ZERO),
        ],
    );
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stderr.contains("ветка архива refs/heads/archive/old"),
        "{}",
        run.output()
    );
}
