//! Сообщение коммита по решению 8: тема `[ТИП](область): суть`, пустая строка
//! после темы, трейлер основания из дерева коммита — `Slipway-Work: wNNNN` или,
//! для коммита закрытия среза, `Slipway-Slice: sNNNN` (решение 15).
//!
//! ```text
//! cargo slipway msg-check [--form-only] <message>
//! cargo slipway msg-check --range <range>
//! ```
//!
//! Без `--form-only` проверяется ещё и то, что каждая работа и каждый срез из
//! трейлеров есть в индексе — в дереве будущего коммита. Хук commit-msg
//! вызывает полный вариант, команда commit — только форму и до блокировки,
//! чтобы ошибка в теме не стоила прогона калитки.
//!
//! `--range` проверяет сообщение каждого коммита диапазона `git rev-list` по
//! дереву этого коммита. Так CI проверяет путь в основную ветку, мимо которого
//! хук мог пройти: правку на сайте, коммит без хуков, слияние кнопкой
//! (решение 18). Последняя строка вывода — `MSG-CHECK OK (<n>)` или
//! `MSG-CHECK REFUSED: <k> из <n>`.
//!
//! Правила темы — набор типов, предел длины, каталоги работ и срезов и ссылка
//! на правила коммитов продукта — берутся из настройки того же дерева, что и
//! таксономия: из индекса или из дерева проверяемого коммита (решение 20).
//! Негодная настройка — ошибка запуска, а не отказ проверки.
//!
//! Заданная в настройке `message_command` забирает форму темы себе: команда
//! проекта получает путь к файлу сообщения аргументом, и код 0 означает
//! «принято». Тогда Slipway не проверяет ни тип, ни область, ни предел длины,
//! ни точку в конце, ни пустую строку после темы — и таксономии в дереве может
//! не быть вовсе. Трейлер основания остаётся за Slipway: он есть, он по форме
//! `wNNNN` или `sNNNN`, и работа или срез лежат в том же дереве. Делегируется
//! форма, а не правило.
//!
//! Код возврата: 0 — принято; 1 — отвергнуто (причины в stderr); 2 — ошибка
//! запуска.

use crate::config::Config;
use crate::{git, layout};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Ключ трейлера основания — единица работы.
const WORK_TRAILER: &str = "Slipway-Work:";

/// Ключ трейлера основания — срез, для коммита его закрытия.
const SLICE_TRAILER: &str = "Slipway-Slice:";

/// `cargo slipway msg-check`.
pub fn run(args: &[OsString]) -> u8 {
    let (form_only, file) = match args {
        [flag, range] if flag.to_str() == Some("--range") => return run_range(range),
        [flag, file] if flag.to_str() == Some("--form-only") => (true, file),
        [file] => (false, file),
        _ => {
            eprintln!(
                "msg-check: a message file or a range is required — msg-check [--form-only] <file> | --range <range>"
            );
            return 2;
        }
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        eprintln!("msg-check: a readable message file is required");
        return 2;
    };
    match check_in_index(Path::new("."), &text, form_only, Some(Path::new(file))) {
        Ok(checked) if checked.problems.is_empty() => 0,
        Ok(checked) => {
            eprint!("{}", checked.report());
            1
        }
        Err(problem) => {
            eprintln!("msg-check: {problem}");
            2
        }
    }
}

/// `msg-check --range`: сообщение каждого коммита диапазона — по дереву этого
/// коммита. Отказ называет коммит двенадцатью знаками хэша и темой.
fn run_range(range: &OsString) -> u8 {
    let dir = Path::new(".");
    let Some(range) = range.to_str() else {
        eprintln!("msg-check: range is not UTF-8");
        return 2;
    };
    let Some(listing) = git::read(dir, &["rev-list", "--reverse", range, "--"]) else {
        eprintln!("msg-check: range {range} cannot be read");
        return 2;
    };
    let commits: Vec<&str> = listing.lines().filter(|line| !line.is_empty()).collect();
    let mut refused = 0;
    for &commit in &commits {
        match check_commit(dir, commit) {
            Ok(checked) if checked.problems.is_empty() => {}
            Ok(checked) => {
                refused += 1;
                let short = commit.get(..12).unwrap_or(commit);
                let subject =
                    git::read(dir, &["log", "-1", "--format=%s", commit]).unwrap_or_default();
                eprint!("{short} {}: {}", subject.trim(), checked.report());
            }
            Err(problem) => {
                eprintln!("msg-check: {commit}: {problem}");
                return 2;
            }
        }
    }
    if refused == 0 {
        println!("MSG-CHECK OK ({})", commits.len());
        0
    } else {
        println!("MSG-CHECK REFUSED: {refused} of {}", commits.len());
        1
    }
}

/// Итог проверки: причины отказа и правила, по которым проверяли.
pub struct Checked {
    /// Причины отказа; пустой список — сообщение принято.
    pub problems: Vec<String>,
    config: Config,
    /// Вывод делегированной команды; показывается только при её отказе.
    output: String,
}

impl Checked {
    /// Текст отказа: ссылка на правила коммитов продукта и причины по одной в
    /// строке. Без настройки называются правила Slipway, а не документ чужого
    /// реестра (решения 19 и 20).
    pub fn report(&self) -> String {
        let mut text = format!(
            "msg-check: message refused ({}):\n",
            self.config.commit_rules
        );
        for problem in &self.problems {
            text.push_str("  - ");
            text.push_str(problem);
            text.push('\n');
        }
        // Вывод делегированной команды — с отступом под её причиной: за этот
        // отказ отвечает проект, и объяснить его может только он (решение 20).
        for line in self.output.lines() {
            text.push_str("    ");
            text.push_str(line);
            text.push('\n');
        }
        text
    }
}

/// Проверяет сообщение по индексу репозитория в `dir`: правила — из настройки
/// в индексе, области — из таксономии в индексе, основания — из индекса.
/// `file` — файл сообщения для делегированной команды, если он есть.
/// `Err` — проверку не из чего выполнить.
pub fn check_in_index(
    dir: &Path,
    text: &str,
    form_only: bool,
    file: Option<&Path>,
) -> Result<Checked, String> {
    check_against(dir, "", "in the index", text, form_only, file)
}

/// Проверяет сообщение коммита по дереву этого же коммита. Своего файла у
/// такого сообщения нет: оно взято из git.
pub fn check_commit(dir: &Path, commit: &str) -> Result<Checked, String> {
    let text = git::read(dir, &["log", "-1", "--format=%B", commit])
        .ok_or_else(|| format!("message of commit {commit} cannot be read"))?;
    check_against(
        dir,
        commit,
        &format!("in the tree of {commit}"),
        &text,
        false,
        None,
    )
}

/// Проверка по дереву `tree`: пустая строка — индекс, иначе ревизия; `place`
/// называет это дерево в тексте ошибки.
fn check_against(
    dir: &Path,
    tree: &str,
    place: &str,
    text: &str,
    form_only: bool,
    file: Option<&Path>,
) -> Result<Checked, String> {
    // Правила — из того же дерева, что и таксономия: иначе правка настройки в
    // рабочей копии меняла бы вердикт для чужого дерева (решение 20).
    let config = Config::read_tree(dir, tree)?;
    let taxonomy = format!("{tree}:{}", config.taxonomy());
    let scopes = git::read(dir, &["show", &taxonomy])
        .map(|text| subsystem_scopes(&text))
        .unwrap_or_default();
    // Без таксономии Slipway нечем проверять области темы, и это ошибка
    // запуска. При делегированной форме областей он не проверяет вовсе, и
    // таксономии в дереве может не быть (решение 20).
    if scopes.is_empty() && config.message_command.is_none() {
        return Err(format!(
            "no Subsystem axis values {place} ({})",
            config.taxonomy()
        ));
    }
    let in_tree = |path: &str| {
        let spec = format!("{tree}:{path}");
        git::succeeds(dir, &["cat-file", "-e", &spec])
    };
    let exists: Option<&dyn Fn(&str) -> bool> = if form_only { None } else { Some(&in_tree) };
    let mut found = Vec::new();
    let mut output = String::new();
    if let Some((problem, shown)) = delegated(dir, &config, text, file)? {
        found.push(problem);
        output = shown;
    }
    found.extend(problems(text, &scopes, exists, &config));
    Ok(Checked {
        problems: found,
        config,
        output,
    })
}

/// Отказ делегированной команды, если она задана: команда получает путь к файлу
/// сообщения аргументом, код 0 — принято. `Ok(None)` — команда приняла
/// сообщение или её нет; `Err` — команду нечем выполнить, а это ошибка запуска,
/// а не отказ сообщения.
fn delegated(
    dir: &Path,
    config: &Config,
    text: &str,
    file: Option<&Path>,
) -> Result<Option<(String, String)>, String> {
    let Some(words) = config.message_command.as_deref() else {
        return Ok(None);
    };
    let (program, args) = words.split_first().ok_or_else(|| {
        format!(
            "{}: key `message_command` has an empty value",
            layout::CONFIG
        )
    })?;
    // Своего файла у сообщения из git нет — оно пишется во временный файл в
    // каталоге git и убирается за собой вместе с `written`.
    let written;
    let path = match file {
        Some(path) => path,
        None => {
            written = CheckedMessage::write(dir, text)?;
            written.path.as_path()
        }
    };
    let out = Command::new(program)
        .current_dir(dir)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| format!("message command `{program}` did not start: {error}"))?;
    if out.status.success() {
        return Ok(None);
    }
    let code = out
        .status
        .code()
        .map_or_else(|| "signal".to_owned(), |code| code.to_string());
    let shown = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(Some((
        format!("subject refused by `{}` (code {code})", words.join(" ")),
        shown,
    )))
}

/// Файл сообщения для делегированной команды, когда своего файла нет: пишется в
/// каталог git и удаляется вместе со структурой.
struct CheckedMessage {
    path: PathBuf,
}

impl CheckedMessage {
    fn write(dir: &Path, text: &str) -> Result<CheckedMessage, String> {
        let git_dir = git::read(dir, &["rev-parse", "--absolute-git-dir"]).ok_or_else(|| {
            "not a git repository — the message cannot be given to the command".to_owned()
        })?;
        let path = PathBuf::from(git_dir).join(layout::CHECKED_MESSAGE);
        fs::write(&path, text)
            .map_err(|error| format!("message file {} not written: {error}", path.display()))?;
        Ok(CheckedMessage { path })
    }
}

impl Drop for CheckedMessage {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Значения оси подсистем из текста таксономии, в нижнем регистре.
///
/// Берётся первое `Subsystem => [ … ]`; упоминание имени без списка, например в
/// комментарии, пропускается.
pub fn subsystem_scopes(taxonomy: &str) -> Vec<String> {
    let mut rest = taxonomy;
    while let Some(at) = rest.find("Subsystem") {
        rest = &rest[at + "Subsystem".len()..];
        let Some(list) = rest.trim_start().strip_prefix("=>") else {
            continue;
        };
        let Some(list) = list.trim_start().strip_prefix('[') else {
            continue;
        };
        let Some((values, _)) = list.split_once(']') else {
            continue;
        };
        return values
            .split(',')
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty())
            .collect();
    }
    Vec::new()
}

/// Причины отказа сообщения; пустой список — сообщение принято.
///
/// `exists` отвечает, есть ли файл плана — работы или среза — в дереве
/// коммита; без неё проверяется только форма. `config` — правила того дерева,
/// по которым идёт проверка (решение 20).
pub fn problems(
    text: &str,
    scopes: &[String],
    exists: Option<&dyn Fn(&str) -> bool>,
    config: &Config,
) -> Vec<String> {
    let mut errors = Vec::new();
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('#')).collect();
    let subject = lines.first().copied().unwrap_or("");

    // Форму темы проверяет команда проекта, если она задана: тип, область,
    // предел длины, точка в конце и пустая строка после темы — её забота
    // (решение 20). Трейлер основания остаётся за Slipway: это правило, а не
    // форма.
    if config.message_command.is_none() {
        subject_problems(subject, lines.get(1).copied(), scopes, config, &mut errors);
    }

    let works = trailer_ids(text, WORK_TRAILER, 'w', &mut errors);
    let slices = trailer_ids(text, SLICE_TRAILER, 's', &mut errors);
    if works.is_empty() && slices.is_empty() {
        errors.push(
            "no Slipway-Work: wNNNN or Slipway-Slice: sNNNN trailer — the commit has no basis in the plan"
                .to_owned(),
        );
    } else if let Some(exists) = exists {
        for work in works {
            let path = format!("{}/{work}.rs", config.work_dir());
            if !exists(&path) {
                errors.push(format!("work {work} is not in the commit tree ({path})"));
            }
        }
        for slice in slices {
            let path = format!("{}/{slice}.rs", config.slice_dir());
            if !exists(&path) {
                errors.push(format!("slice {slice} is not in the commit tree ({path})"));
            }
        }
    }
    errors
}

/// Причины отказа по форме темы — ровно то, что забирает себе делегированная
/// команда проекта: тип, области, точка в конце, предел длины и пустая строка
/// после темы. `after` — строка за темой, если она есть.
fn subject_problems(
    subject: &str,
    after: Option<&str>,
    scopes: &[String],
    config: &Config,
    errors: &mut Vec<String>,
) {
    match parse_subject(subject) {
        Some((kind, subject_scopes, summary)) => {
            if !config.commit_types.iter().any(|known| known == kind) {
                errors.push(format!(
                    "type [{kind}] is not in the set: {}",
                    config.commit_types.join(" ")
                ));
            }
            for scope in subject_scopes.split(',') {
                if !scopes.iter().any(|known| known == scope) {
                    errors.push(format!(
                        "scope ({scope}) is not a subsystem axis value: {}",
                        scopes.join(" ")
                    ));
                }
            }
            if summary.ends_with('.') {
                errors.push("subject ends with a period".to_owned());
            }
            let length = subject.chars().count();
            if length > config.subject_limit {
                errors.push(format!(
                    "subject is longer than {} characters ({length})",
                    config.subject_limit
                ));
            }
        }
        None => errors.push(format!(
            "subject is not in the form [TYPE](scope): summary — `{subject}`"
        )),
    }

    if after.is_some_and(|line| !line.is_empty()) {
        errors.push("a blank line must follow the subject".to_owned());
    }
}

/// Идентификаторы из трейлеров `key`; трейлер не по форме добавляет причину.
fn trailer_ids<'a>(
    text: &'a str,
    key: &str,
    prefix: char,
    errors: &mut Vec<String>,
) -> Vec<&'a str> {
    let lines: Vec<&str> = text.lines().filter(|line| line.starts_with(key)).collect();
    if lines
        .iter()
        .any(|line| trailer_id(line, key, prefix).is_none())
    {
        let name = key.trim_end_matches(':');
        errors.push(format!("trailer {name} is not in the form {prefix}NNNN"));
    }
    lines
        .iter()
        .filter_map(|line| trailer_id(line, key, prefix))
        .collect()
}

/// Тип, области и суть темы `[ТИП](область): суть`; `None` — тема не по форме.
fn parse_subject(subject: &str) -> Option<(&str, &str, &str)> {
    let rest = subject.strip_prefix('[')?;
    let (kind, rest) = rest.split_once(']')?;
    if kind.is_empty() || !kind.bytes().all(|b| b.is_ascii_uppercase()) {
        return None;
    }
    let (scopes, rest) = rest.strip_prefix('(')?.split_once(')')?;
    let scope_byte = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b',';
    if scopes.is_empty() || !scopes.bytes().all(scope_byte) {
        return None;
    }
    let summary = rest.strip_prefix(": ")?;
    if summary.is_empty() {
        return None;
    }
    Some((kind, scopes, summary))
}

/// Идентификатор из строки трейлера — ровно `<ключ> <префикс>NNNN`.
fn trailer_id<'a>(line: &'a str, key: &str, prefix: char) -> Option<&'a str> {
    let id = line.strip_prefix(key)?.strip_prefix(' ')?;
    let digits = id.strip_prefix(prefix)?;
    (digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes() -> Vec<String> {
        vec!["knowledge".to_owned(), "cli".to_owned()]
    }

    fn planned(path: &str) -> bool {
        path == "doc/work/w0001.rs" || path == "doc/slice/s0001.rs"
    }

    #[test]
    fn taxonomy_values_become_scopes() {
        let text = "// Subsystem — ось подсистем\nslipway_core::declare_taxonomy! {\n    Subsystem => [Knowledge, Cli],\n    Team => [Core],\n}\n";
        assert_eq!(subsystem_scopes(text), ["knowledge", "cli"]);
        assert!(subsystem_scopes("Team => [Core]").is_empty());
    }

    #[test]
    fn well_formed_messages_pass() {
        for message in [
            "[FEAT](cli,knowledge): суть\n\nтело\n\nSlipway-Work: w0001\n",
            "[PLAN](cli): закрыт срез s0001\n\nSlipway-Slice: s0001\n",
        ] {
            assert_eq!(
                problems(message, &scopes(), Some(&planned), &Config::default()),
                Vec::<String>::new(),
                "{message}"
            );
        }
    }

    #[test]
    fn each_rule_names_its_violation() {
        let long = format!("[FEAT](cli): {}\n\nSlipway-Work: w0001", "я".repeat(70));
        let cases = [
            (
                "суть без типа\n\nSlipway-Work: w0001",
                "subject is not in the form",
            ),
            (
                "[FEATURE](cli): суть\n\nSlipway-Work: w0001",
                "type [FEATURE]",
            ),
            ("[FEAT](wal): суть\n\nSlipway-Work: w0001", "scope (wal)"),
            (
                "[FEAT](cli,,knowledge): суть\n\nSlipway-Work: w0001",
                "scope ()",
            ),
            (
                "[FEAT](cli): суть.\n\nSlipway-Work: w0001",
                "subject ends with a period",
            ),
            // Тринадцать символов «[FEAT](cli): » и семьдесят «я».
            (long.as_str(), "subject is longer than 72 characters (83)"),
            (
                "[FEAT](cli): суть\nтело\n\nSlipway-Work: w0001",
                "a blank line must follow the subject",
            ),
            (
                "[FEAT](cli): суть\n\nSlipway-Work: 1",
                "is not in the form wNNNN",
            ),
            (
                "[FEAT](cli): суть\n\nSlipway-Work: w00012",
                "is not in the form wNNNN",
            ),
            (
                "[FEAT](cli): суть",
                "no Slipway-Work: wNNNN or Slipway-Slice: sNNNN trailer",
            ),
            (
                "[FEAT](cli): суть\n\nSlipway-Work: w0099",
                "work w0099 is not in the commit tree",
            ),
            (
                "[PLAN](cli): закрыт срез\n\nSlipway-Slice: 7",
                "trailer Slipway-Slice is not in the form sNNNN",
            ),
            (
                "[PLAN](cli): закрыт срез\n\nSlipway-Slice: s0099",
                "slice s0099 is not in the commit tree",
            ),
        ];
        for (message, expected) in cases {
            let found = problems(message, &scopes(), Some(&planned), &Config::default());
            assert!(
                found.iter().any(|problem| problem.contains(expected)),
                "«{message}»: ожидалось «{expected}», получено {found:?}"
            );
        }
    }

    #[test]
    fn form_only_does_not_look_for_work() {
        let message = "[FEAT](cli): суть\n\nSlipway-Work: w0099";
        assert!(problems(message, &scopes(), None, &Config::default()).is_empty());
    }

    /// Чужая раскладка: свой набор типов, свой предел темы и свой корень
    /// реестра (решение 20).
    #[test]
    fn configured_rules_replace_the_defaults() {
        let config = Config {
            doc: "docs/registry".to_owned(),
            commit_types: vec!["FEAT".to_owned(), "CHANGE".to_owned()],
            subject_limit: 90,
            ..Config::default()
        };
        // Пятнадцать символов «[CHANGE](cli): » и семьдесят «я»: больше прежних
        // семидесяти двух и меньше настроенных девяноста.
        let long = format!("[CHANGE](cli): {}\n\nSlipway-Work: w0001", "я".repeat(70));
        assert_eq!(
            problems(&long, &scopes(), None, &config),
            Vec::<String>::new()
        );
        assert!(
            problems(&long, &scopes(), None, &Config::default())
                .iter()
                .any(|problem| problem.contains("longer than 72 characters (85)")),
            "прежний предел не применён к контрольному сообщению"
        );

        let refused = problems(
            "[FIX](cli): суть\n\nSlipway-Work: w0001",
            &scopes(),
            None,
            &config,
        );
        assert!(
            refused
                .iter()
                .any(|problem| problem == "type [FIX] is not in the set: FEAT CHANGE"),
            "{refused:?}"
        );

        let planned = |path: &str| path == "docs/registry/work/w0001.rs";
        let message = "[FEAT](cli): суть\n\nSlipway-Work: w0002";
        let missing = problems(message, &scopes(), Some(&planned), &config);
        assert!(
            missing
                .iter()
                .any(|problem| problem.contains("docs/registry/work/w0002.rs")),
            "{missing:?}"
        );
        let message = "[FEAT](cli): суть\n\nSlipway-Work: w0001";
        assert!(problems(message, &scopes(), Some(&planned), &config).is_empty());
    }

    /// Делегированная проверка формы (решение 20): тему забирает команда
    /// проекта, а трейлер основания и наличие работы в дереве остаются за
    /// Slipway. Значений оси подсистем при этом может не быть вовсе.
    #[test]
    fn a_delegated_form_leaves_the_basis_to_slipway() {
        let config = Config {
            message_command: Some(vec!["true".to_owned()]),
            ..Config::default()
        };
        // Тема, невозможная по правилам Slipway: свой тип, своя область, длина
        // больше предела, точка в конце и тело сразу за темой.
        let subject = format!("CHANGE: {}.", "и".repeat(80));
        let accepted = format!("{subject}\nтело\n\nSlipway-Work: w0001\n");
        assert_eq!(
            problems(&accepted, &[], Some(&planned), &config),
            Vec::<String>::new()
        );
        // Без делегирования та же тема отвергается — контроль на то, что дело в
        // настройке, а не в самой теме.
        assert!(!problems(&accepted, &[], Some(&planned), &Config::default()).is_empty());

        for (message, expected) in [
            (
                format!("{subject}\n"),
                "no Slipway-Work: wNNNN or Slipway-Slice: sNNNN trailer",
            ),
            (
                format!("{subject}\n\nSlipway-Work: 7\n"),
                "trailer Slipway-Work is not in the form wNNNN",
            ),
            (
                format!("{subject}\n\nSlipway-Work: w0099\n"),
                "work w0099 is not in the commit tree",
            ),
        ] {
            let found = problems(&message, &[], Some(&planned), &config);
            assert!(
                found.iter().any(|problem| problem.contains(expected)),
                "«{message}»: ожидалось «{expected}», получено {found:?}"
            );
        }
    }

    #[test]
    fn comment_lines_are_not_the_subject() {
        let message = "# комментарий git\n[FEAT](cli): суть\n\nSlipway-Work: w0001";
        assert!(problems(message, &scopes(), Some(&planned), &Config::default()).is_empty());
    }
}
