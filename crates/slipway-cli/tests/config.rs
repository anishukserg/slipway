//! Сценарии настройки продукта (решение 20): инструмент читает `slipway.toml`
//! и работает в чужой раскладке — свой корень реестра, свой набор типов темы,
//! свой предел её длины, своя ссылка на правила коммитов и своя тема
//! служебного коммита. Негодная настройка — ошибка запуска, а не отказ
//! проверки.
//!
//! Раскладка без файла настройки проверяется прочими сценариями: их ожидания
//! не менялись.

mod common;

use common::{Run, TempRepo};

/// Настройка чужого продукта: реестр в `docs/registry`, два типа темы, предел
/// в девяносто знаков, свои правила коммитов и своя тема начала работы.
const CONFIG: &str = concat!(
    "# Раскладка продукта\n",
    "doc = \"docs/registry\"\n",
    "commit_types = \"FEAT CHANGE\"\n",
    "subject_limit = \"90\"\n",
    "commit_rules = \"docs/COMMITS.md\"\n",
    "subject_started = \"[CHANGE]({scope}): начата работа {id}\"\n",
);

const TAXONOMY: &str = "slipway_core::declare_taxonomy! {\n    Subsystem => [Cli, Work],\n}\n";

const WORK: &str = concat!(
    "slipway_work::work!(1,\n",
    "    title: NonEmptyStr::new(\"Первая работа\"),\n",
    "    slice: crate::slice::s0001,\n",
    "    taxon: taxon!(Subsystem, Cli),\n",
    ");\n",
);

/// Репозиторий с настройкой, реестром в `docs/registry`, хуком commit-msg и
/// базовым коммитом, прошедшим этот хук по настроенным правилам.
fn configured_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.write("slipway.toml", CONFIG);
    repo.write("docs/COMMITS.md", "Правила коммитов продукта.\n");
    repo.write("docs/registry/taxonomy.rs", TAXONOMY);
    repo.write("docs/registry/work/w0001.rs", WORK);
    repo.write(
        "docs/registry/slice/s0001.rs",
        "slipway_work::slice!(1,\n    title: NonEmptyStr::new(\"Первый срез\"),\n);\n",
    );
    repo.write("docs/registry/journal/README.md", "журнал\n");
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
        "[CHANGE](cli): база",
        "-m",
        "Slipway-Work: w0001",
    ]);
    repo
}

/// `commit -F <сообщение> -- <пути>`.
fn commit(repo: &TempRepo, message: &str, paths: &[&str]) -> Run {
    let file = repo.outside("message.txt", message);
    let mut args = vec!["commit", "-F", file.to_str().expect("путь в UTF-8"), "--"];
    args.extend_from_slice(paths);
    repo.tool(&args)
}

#[test]
fn configured_types_and_limit_replace_the_conventions() {
    let repo = configured_repo("config-subject-rules");
    repo.write("new.txt", "new\n");

    // Пятнадцать знаков «[CHANGE](cli): » и семьдесят «и»: больше прежних
    // семидесяти двух и меньше настроенных девяноста.
    let subject = format!("[CHANGE](cli): {}", "и".repeat(70));
    assert_eq!(subject.chars().count(), 85);
    let run = commit(
        &repo,
        &format!("{subject}\n\nSlipway-Work: w0001\n"),
        &["new.txt"],
    );
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(run.verdict().starts_with("COMMIT OK "), "{}", run.output());

    // Тип вне настроенного набора отвергается, и отказ называет набор продукта
    // и его правила коммитов, а не соглашения Slipway.
    repo.write("other.txt", "other\n");
    let run = commit(
        &repo,
        "[FIX](cli): исправление\n\nSlipway-Work: w0001\n",
        &["other.txt"],
    );
    assert_eq!(run.code, 4, "{}", run.output());
    assert!(
        run.stderr
            .contains("type [FIX] is not in the set: FEAT CHANGE"),
        "{}",
        run.output()
    );
    assert!(
        run.stderr.contains("message refused (docs/COMMITS.md)"),
        "{}",
        run.output()
    );
    assert!(
        run.verdict().starts_with("COMMIT REFUSED: "),
        "{}",
        run.output()
    );

    // Тема длиннее настроенного предела отвергается им, а не прежним.
    let subject = format!("[CHANGE](cli): {}", "и".repeat(80));
    let run = commit(
        &repo,
        &format!("{subject}\n\nSlipway-Work: w0001\n"),
        &["other.txt"],
    );
    assert_eq!(run.code, 4, "{}", run.output());
    assert!(
        run.stderr.contains("longer than 90 characters (95)"),
        "{}",
        run.output()
    );
}

#[test]
fn work_start_writes_into_the_configured_registry() {
    let repo = configured_repo("config-work-start");
    let run = repo.tool(&["work", "start", "w0001"]);
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(run.verdict().starts_with("COMMIT OK "), "{}", run.output());

    // Тема служебного коммита — из шаблона настройки, и она сама проходит
    // настроенные правила: коммит создал хук commit-msg.
    let body = repo.git(&["log", "-1", "--format=%B"]);
    assert!(
        body.starts_with("[CHANGE](cli): начата работа w0001"),
        "{body}"
    );
    assert!(body.contains("Slipway-Work: w0001"), "{body}");

    let files = repo.git(&["show", "--name-only", "--format=", "HEAD"]);
    assert!(
        files.contains("docs/registry/journal/w0001/") && files.contains("-started.toml"),
        "{files}"
    );
    assert!(
        repo.tool(&["work", "state", "w0001"])
            .stdout
            .contains("started"),
        "событие не прочитано из настроенного каталога журнала"
    );
}

/// Репозиторий с делегированной проверкой формы темы: команда продукта
/// принимает тему, начинающуюся с `OK `, и отвергает любую другую. Таксономии в
/// дереве нет вовсе — области темы Slipway больше не проверяет.
fn delegating_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.executable(
        "scripts/subject",
        concat!(
            "#!/bin/sh\n",
            "if head -1 \"$1\" | grep -q '^OK '; then exit 0; fi\n",
            "echo 'subject must start with OK' >&2\n",
            "exit 1\n",
        ),
    );
    let checker = repo.path("scripts/subject");
    repo.write(
        "slipway.toml",
        &format!(
            "message_command = \"{}\"\n",
            checker.to_str().expect("путь в UTF-8")
        ),
    );
    repo.write("doc/work/w0001.rs", WORK);
    repo
}

/// `msg-check <сообщение>`.
fn msg_check(repo: &TempRepo, message: &str) -> Run {
    let file = repo.outside("message.txt", message);
    repo.tool(&["msg-check", file.to_str().expect("путь в UTF-8")])
}

#[test]
fn a_delegated_check_takes_the_form_and_leaves_the_basis_to_slipway() {
    let repo = delegating_repo("config-message-command");
    repo.git(&["add", "-A"]);

    // Тему, которую Slipway принял бы сам, отвергает команда продукта, и её
    // вывод показан в отказе.
    let run = msg_check(&repo, "[FEAT](cli): суть\n\nSlipway-Work: w0001\n");
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stderr.contains("subject refused by"),
        "{}",
        run.output()
    );
    assert!(
        run.stderr.contains("subject must start with OK"),
        "{}",
        run.output()
    );

    // Тема, невозможная по правилам Slipway — свой тип и длина больше
    // семидесяти двух знаков, — проходит, раз команда её приняла. Значений оси
    // подсистем в дереве нет, и это не ошибка запуска.
    let subject = format!("OK {}", "и".repeat(80));
    let run = msg_check(&repo, &format!("{subject}\n\nSlipway-Work: w0001\n"));
    assert_eq!(run.code, 0, "{}", run.output());

    // Трейлер основания и наличие работы в дереве остаются за Slipway: команда
    // довольна, а сообщение всё равно отвергнуто.
    let run = msg_check(&repo, &format!("{subject}\n\nбез основания\n"));
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(run.stderr.contains("no Slipway-Work"), "{}", run.output());

    let run = msg_check(&repo, &format!("{subject}\n\nSlipway-Work: w0099\n"));
    assert_eq!(run.code, 1, "{}", run.output());
    assert!(
        run.stderr.contains("work w0099 is not in the commit tree"),
        "{}",
        run.output()
    );
}

#[test]
fn a_delegated_check_of_a_range_gives_the_command_a_file_and_removes_it() {
    let repo = delegating_repo("config-message-range");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "OK база", "-m", "Slipway-Work: w0001"]);
    let base = repo.git(&["rev-parse", "HEAD"]).trim().to_owned();

    // Сообщение коммита берётся из git, а команде нужен файл: он пишется в
    // каталог git и убирается за собой.
    repo.write("a.txt", "a\n");
    repo.git(&["add", "-A"]);
    repo.git(&[
        "commit",
        "-q",
        "-m",
        "тема без начала",
        "-m",
        "Slipway-Work: w0001",
    ]);

    let run = repo.tool(&["msg-check", "--range", &format!("{base}..HEAD")]);
    assert_eq!(run.code, 1, "{}", run.output());
    assert_eq!(
        run.verdict(),
        "MSG-CHECK REFUSED: 1 of 1",
        "{}",
        run.output()
    );
    assert!(
        run.stderr.contains("subject must start with OK"),
        "{}",
        run.output()
    );
    assert!(
        !repo.path(".git/slipway-checked-message").exists(),
        "временный файл сообщения остался в каталоге git"
    );
}

#[test]
fn a_bad_configuration_is_a_startup_error_naming_the_file_and_the_key() {
    for (name, text, expected) in [
        (
            "config-unknown-key",
            "docs = \"registry\"\n",
            "unknown key `docs`",
        ),
        (
            "config-not-a-number",
            "subject_limit = \"много\"\n",
            "key `subject_limit` takes a number",
        ),
        (
            "config-empty-types",
            "commit_types = \"\"\n",
            "key `commit_types` has an empty value",
        ),
    ] {
        let repo = TempRepo::new(name);
        repo.write("doc/taxonomy.rs", TAXONOMY);
        repo.write("doc/work/w0001.rs", WORK);
        repo.write("slipway.toml", text);
        repo.git(&["add", "-A"]);
        repo.git(&[
            "commit",
            "-q",
            "-m",
            "[CHORE](cli): база",
            "-m",
            "Slipway-Work: w0001",
        ]);

        // Проверка сообщения читает настройку из индекса: негодная настройка —
        // ошибка запуска (2), а не отказ проверки (1).
        let message = repo.outside("message.txt", "[FEAT](cli): суть\n\nSlipway-Work: w0001\n");
        let run = repo.tool(&["msg-check", message.to_str().expect("путь в UTF-8")]);
        assert_eq!(run.code, 2, "{name}: {}", run.output());
        assert!(
            run.stderr.contains("slipway.toml") && run.stderr.contains(expected),
            "{name}: {}",
            run.output()
        );

        // Команда работы читает настройку из рабочего дерева — тот же отказ и
        // тот же код.
        let run = repo.tool(&["work", "state"]);
        assert_eq!(run.code, 2, "{name}: {}", run.output());
        assert!(
            run.verdict().contains("slipway.toml") && run.verdict().contains(expected),
            "{name}: {}",
            run.output()
        );
    }
}
