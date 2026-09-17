//! Сценарии калитки (решения 8 и 9), перенесённые из самотеста правил
//! коммитов. Все они отказывают до запуска cargo: во временном дереве нет
//! Cargo.toml, и калитка обязана остановиться на этом, а не искать рабочее
//! пространство в родительских каталогах.

mod common;

use common::TempRepo;
use std::fs;

/// Список внешних имён в каталоге git временного репозитория.
fn with_external_names(repo: &TempRepo, names: &str) {
    fs::write(repo.path(".git/info/slipway-external-names"), names).expect("список записан");
}

#[test]
fn external_name_in_the_tree_is_refused() {
    let repo = TempRepo::new("gate-external-name");
    with_external_names(&repo, "# внешние проекты\nzzvneshniy\n");
    repo.write("leak.txt", "текст с именем ZZVneshniy внутри\n");
    let run = repo.tool(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stdout.contains("  внешнее имя в файле: leak.txt"),
        "{}",
        run.output()
    );
    assert!(
        run.verdict().starts_with("GATE FAIL: внешние имена"),
        "{}",
        run.output()
    );
}

#[test]
fn missing_list_is_a_skipped_step_not_a_passed_one() {
    let repo = TempRepo::new("gate-no-list");
    repo.write("leak.txt", "текст с именем ZZVneshniy внутри\n");
    let run = repo.tool(&["gate"]);
    assert!(run.stdout.contains("шаг не выполнялся"), "{}", run.output());
}

#[test]
fn tree_without_manifest_does_not_start_cargo() {
    let repo = TempRepo::new("gate-no-manifest");
    let run = repo.tool(&["gate"]);
    assert_eq!(run.code, 2, "{}", run.output());
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );
}

#[test]
fn broken_markdown_link_is_refused_even_inside_a_git_directory() {
    // Дерево коммита выгружается внутрь каталога git: исключение путей по
    // подстроке «/.git/» отсекало бы все файлы, и шаг молча оставался бы без
    // предмета.
    let repo = TempRepo::inside("gate-markdown", ".git");
    repo.write("doc.md", "# Документ\n\nСм. [файл](missing.md).\n");
    let run = repo.tool(&["gate"]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stdout.contains("  битая ссылка: doc.md: missing.md"),
        "{}",
        run.output()
    );
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: относительные ссылки в markdown"),
        "{}",
        run.output()
    );

    // Контроль: ссылка на существующий файл проходит шаг, и калитка доходит до
    // проверки манифеста.
    repo.write("present.md", "есть\n");
    repo.write("doc.md", "# Документ\n\nСм. [файл](present.md#раздел).\n");
    let run = repo.tool(&["gate"]);
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );

    // Код в документе — не ссылка: форма темы коммита в обратных кавычках и
    // пример в огороженном блоке кода не отвергают документ (работа 31).
    repo.write(
        "doc.md",
        "# Документ\n\nТема `[ТИП](область): суть`.\n\n```text\n[FEAT](cli): суть\n```\n",
    );
    let run = repo.tool(&["gate"]);
    assert!(!run.stdout.contains("битая ссылка"), "{}", run.output());
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );
}

#[test]
fn explicit_tree_is_checked_instead_of_the_working_tree() {
    let repo = TempRepo::new("gate-explicit-tree");
    with_external_names(&repo, "zzvneshniy\n");
    repo.write("outside.txt", "ZZVneshniy вне проверяемого дерева\n");
    repo.write("tree/clean.txt", "чисто\n");
    let root = repo.root.to_str().expect("путь в UTF-8").to_owned();
    let tree = repo.path("tree");
    let tree = tree.to_str().expect("путь в UTF-8");

    // Внешнее имя лежит вне дерева: шаг пройден, калитка дошла до манифеста.
    let run = repo.tool(&["gate", "--repo", &root, tree]);
    assert!(
        !run.stdout.contains("внешнее имя в файле"),
        "{}",
        run.output()
    );
    assert!(
        run.verdict()
            .starts_with("GATE FAIL: в дереве нет Cargo.toml"),
        "{}",
        run.output()
    );

    let run = repo.tool(&["gate", tree, "лишний"]);
    assert_eq!(run.code, 2, "{}", run.output());
    assert!(
        run.verdict().contains("лишний аргумент"),
        "{}",
        run.output()
    );
}
