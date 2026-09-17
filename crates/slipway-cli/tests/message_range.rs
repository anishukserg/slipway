//! Сценарий проверки оснований диапазона коммитов (решение 18): сообщение
//! каждого коммита сверяется с деревом этого коммита, а отказ называет коммит.

mod common;

use common::TempRepo;

/// Коммит всех изменений рабочего дерева; каждый абзац сообщения — отдельный
/// `-m`. Возвращает хэш коммита.
fn commit(repo: &TempRepo, paragraphs: &[&str]) -> String {
    repo.git(&["add", "-A"]);
    let mut args = vec!["commit", "-q"];
    for paragraph in paragraphs {
        args.extend(["-m", *paragraph]);
    }
    repo.git(&args);
    repo.git(&["rev-parse", "HEAD"]).trim().to_owned()
}

#[test]
fn range_checks_each_commit_against_its_own_tree() {
    let repo = TempRepo::new("msg-range");
    repo.write(
        "doc/taxonomy.rs",
        "slipway_core::declare_taxonomy! {\n    Subsystem => [Cli],\n}\n",
    );
    repo.write("doc/work/w0001.rs", "work\n");
    let base = commit(&repo, &["[CHORE](cli): база", "Slipway-Work: w0001"]);

    repo.write("a.txt", "a\n");
    let good = commit(&repo, &["[FEAT](cli): первый", "Slipway-Work: w0001"]);

    let run = repo.tool(&["msg-check", "--range", &format!("{base}..{good}")]);
    assert_eq!(run.code, 0, "{}", run.output());
    assert_eq!(run.verdict(), "MSG-CHECK OK (1)", "{}", run.output());

    // Работа из трейлера появляется только в следующем коммите: основание
    // сверяется с деревом своего коммита, а не с вершиной диапазона.
    repo.write("b.txt", "b\n");
    let unplanned = commit(&repo, &["[FEAT](cli): без плана", "Slipway-Work: w0002"]);
    repo.write("doc/work/w0002.rs", "work\n");
    let web = commit(&repo, &["правка на сайте"]);

    let run = repo.tool(&["msg-check", "--range", &format!("{base}..HEAD")]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert_eq!(
        run.verdict(),
        "MSG-CHECK REFUSED: 2 of 3",
        "{}",
        run.output()
    );
    for refused in [&unplanned, &web] {
        assert!(run.stderr.contains(&refused[..12]), "{}", run.output());
    }
    assert!(!run.stderr.contains(&good[..12]), "{}", run.output());

    let run = repo.tool(&["msg-check", "--range", "нет..такого"]);
    assert_eq!(run.code, 2, "{}", run.output());
}
