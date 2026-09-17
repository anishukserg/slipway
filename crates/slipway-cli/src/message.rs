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
//! Код возврата: 0 — принято; 1 — отвергнуто (причины в stderr); 2 — ошибка
//! запуска.

use crate::{git, layout};
use std::ffi::OsString;
use std::path::Path;

/// Типы темы — закрытый набор.
pub const TYPES: [&str; 8] = [
    "FEAT", "FIX", "REFACTOR", "TEST", "DOCS", "ADR", "PLAN", "CHORE",
];

/// Предел длины темы в символах.
const SUBJECT_LIMIT: usize = 72;

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
    match check_in_index(Path::new("."), &text, form_only) {
        Ok(errors) if errors.is_empty() => 0,
        Ok(errors) => {
            eprint!("{}", report(&errors));
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
            Ok(errors) if errors.is_empty() => {}
            Ok(errors) => {
                refused += 1;
                let short = commit.get(..12).unwrap_or(commit);
                let subject =
                    git::read(dir, &["log", "-1", "--format=%s", commit]).unwrap_or_default();
                eprint!("{short} {}: {}", subject.trim(), report(&errors));
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

/// Проверяет сообщение по индексу репозитория в `dir`: области — из таксономии
/// в индексе, основания — из индекса. `Err` — проверку не из чего выполнить.
pub fn check_in_index(dir: &Path, text: &str, form_only: bool) -> Result<Vec<String>, String> {
    check_against(dir, "", "in the index", text, form_only)
}

/// Проверяет сообщение коммита по дереву этого же коммита.
pub fn check_commit(dir: &Path, commit: &str) -> Result<Vec<String>, String> {
    let text = git::read(dir, &["log", "-1", "--format=%B", commit])
        .ok_or_else(|| format!("message of commit {commit} cannot be read"))?;
    check_against(
        dir,
        commit,
        &format!("in the tree of {commit}"),
        &text,
        false,
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
) -> Result<Vec<String>, String> {
    let taxonomy = format!("{tree}:{}", layout::TAXONOMY);
    let scopes = git::read(dir, &["show", &taxonomy])
        .map(|text| subsystem_scopes(&text))
        .unwrap_or_default();
    if scopes.is_empty() {
        return Err(format!(
            "no Subsystem axis values {place} ({})",
            layout::TAXONOMY
        ));
    }
    let in_tree = |path: &str| {
        let spec = format!("{tree}:{path}");
        git::succeeds(dir, &["cat-file", "-e", &spec])
    };
    let exists: Option<&dyn Fn(&str) -> bool> = if form_only { None } else { Some(&in_tree) };
    Ok(problems(text, &scopes, exists))
}

/// Текст отказа: ссылка на правило и причины по одной в строке.
pub fn report(errors: &[String]) -> String {
    let mut text = format!("msg-check: message refused ({}):\n", layout::COMMIT_RULES);
    for error in errors {
        text.push_str("  - ");
        text.push_str(error);
        text.push('\n');
    }
    text
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
/// коммита; без неё проверяется только форма.
pub fn problems(
    text: &str,
    scopes: &[String],
    exists: Option<&dyn Fn(&str) -> bool>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('#')).collect();
    let subject = lines.first().copied().unwrap_or("");

    match parse_subject(subject) {
        Some((kind, subject_scopes, summary)) => {
            if !TYPES.contains(&kind) {
                errors.push(format!(
                    "type [{kind}] is not in the set: {}",
                    TYPES.join(" ")
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
            if length > SUBJECT_LIMIT {
                errors.push(format!(
                    "subject is longer than {SUBJECT_LIMIT} characters ({length})"
                ));
            }
        }
        None => errors.push(format!(
            "subject is not in the form [TYPE](scope): summary — `{subject}`"
        )),
    }

    if lines.get(1).is_some_and(|line| !line.is_empty()) {
        errors.push("a blank line must follow the subject".to_owned());
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
            let path = format!("{}/{work}.rs", layout::WORK_DIR);
            if !exists(&path) {
                errors.push(format!("work {work} is not in the commit tree ({path})"));
            }
        }
        for slice in slices {
            let path = format!("{}/{slice}.rs", layout::SLICE_DIR);
            if !exists(&path) {
                errors.push(format!("slice {slice} is not in the commit tree ({path})"));
            }
        }
    }
    errors
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
                problems(message, &scopes(), Some(&planned)),
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
            let found = problems(message, &scopes(), Some(&planned));
            assert!(
                found.iter().any(|problem| problem.contains(expected)),
                "«{message}»: ожидалось «{expected}», получено {found:?}"
            );
        }
    }

    #[test]
    fn form_only_does_not_look_for_work() {
        let message = "[FEAT](cli): суть\n\nSlipway-Work: w0099";
        assert!(problems(message, &scopes(), None).is_empty());
    }

    #[test]
    fn comment_lines_are_not_the_subject() {
        let message = "# комментарий git\n[FEAT](cli): суть\n\nSlipway-Work: w0001";
        assert!(problems(message, &scopes(), Some(&planned)).is_empty());
    }
}
