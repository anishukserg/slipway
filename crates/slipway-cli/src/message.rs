//! Сообщение коммита по решению 8: тема `[ТИП](область): суть`, пустая строка
//! после темы, трейлер `Slipway-Work: wNNNN` с единицей работы из дерева
//! коммита.
//!
//! ```text
//! cargo slipway msg-check [--form-only] <файл сообщения>
//! ```
//!
//! Без `--form-only` проверяется ещё и то, что каждая работа из трейлера есть в
//! индексе — в дереве будущего коммита. Хук commit-msg вызывает полный вариант,
//! команда commit — только форму и до блокировки, чтобы ошибка в теме не стоила
//! прогона калитки.
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

/// Ключ трейлера основания.
const TRAILER: &str = "Slipway-Work:";

/// `cargo slipway msg-check`.
pub fn run(args: &[OsString]) -> u8 {
    let (form_only, file) = match args {
        [flag, file] if flag.to_str() == Some("--form-only") => (true, file),
        [file] => (false, file),
        _ => {
            eprintln!("msg-check: нужен файл сообщения — msg-check [--form-only] <файл>");
            return 2;
        }
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        eprintln!("msg-check: нужен читаемый файл сообщения");
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

/// Проверяет сообщение по индексу репозитория в `dir`: области — из таксономии
/// в индексе, работы — из индекса. `Err` — проверку не из чего выполнить.
pub fn check_in_index(dir: &Path, text: &str, form_only: bool) -> Result<Vec<String>, String> {
    let taxonomy = format!(":{}", layout::TAXONOMY);
    let scopes = git::read(dir, &["show", &taxonomy])
        .map(|text| subsystem_scopes(&text))
        .unwrap_or_default();
    if scopes.is_empty() {
        return Err(format!(
            "в индексе нет значений оси Subsystem ({})",
            layout::TAXONOMY
        ));
    }
    let work_in_index = |work: &str| {
        let spec = format!(":{}/{work}.rs", layout::WORK_DIR);
        git::succeeds(dir, &["cat-file", "-e", &spec])
    };
    let work_exists: Option<&dyn Fn(&str) -> bool> = if form_only {
        None
    } else {
        Some(&work_in_index)
    };
    Ok(problems(text, &scopes, work_exists))
}

/// Текст отказа: ссылка на правило и причины по одной в строке.
pub fn report(errors: &[String]) -> String {
    let mut text = format!(
        "msg-check: сообщение отвергнуто ({}):\n",
        layout::COMMIT_RULES
    );
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
/// `work_exists` отвечает, есть ли единица работы в дереве коммита; без неё
/// проверяется только форма.
pub fn problems(
    text: &str,
    scopes: &[String],
    work_exists: Option<&dyn Fn(&str) -> bool>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with('#')).collect();
    let subject = lines.first().copied().unwrap_or("");

    match parse_subject(subject) {
        Some((kind, subject_scopes, summary)) => {
            if !TYPES.contains(&kind) {
                errors.push(format!("тип [{kind}] не из набора: {}", TYPES.join(" ")));
            }
            for scope in subject_scopes.split(',') {
                if !scopes.iter().any(|known| known == scope) {
                    errors.push(format!(
                        "область ({scope}) не значение оси подсистем: {}",
                        scopes.join(" ")
                    ));
                }
            }
            if summary.ends_with('.') {
                errors.push("точка в конце темы".to_owned());
            }
            let length = subject.chars().count();
            if length > SUBJECT_LIMIT {
                errors.push(format!("тема длиннее {SUBJECT_LIMIT} символов ({length})"));
            }
        }
        None => errors.push(format!(
            "тема не по форме [ТИП](область): суть — «{subject}»"
        )),
    }

    if lines.get(1).is_some_and(|line| !line.is_empty()) {
        errors.push("после темы нужна пустая строка".to_owned());
    }

    let trailers: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with(TRAILER))
        .collect();
    if trailers.iter().any(|line| work_id(line).is_none()) {
        errors.push("трейлер Slipway-Work не по форме wNNNN".to_owned());
    }
    let works: Vec<&str> = trailers.iter().filter_map(|line| work_id(line)).collect();
    if works.is_empty() {
        errors
            .push("нет трейлера Slipway-Work: wNNNN — у коммита нет основания в плане".to_owned());
    } else if let Some(exists) = work_exists {
        for work in works {
            if !exists(work) {
                errors.push(format!(
                    "единицы работы {work} нет в дереве коммита ({}/{work}.rs)",
                    layout::WORK_DIR
                ));
            }
        }
    }
    errors
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

/// Идентификатор работы из строки трейлера — ровно `Slipway-Work: wNNNN`.
fn work_id(line: &str) -> Option<&str> {
    let id = line.strip_prefix(TRAILER)?.strip_prefix(' ')?;
    let digits = id.strip_prefix('w')?;
    (digits.len() == 4 && digits.bytes().all(|b| b.is_ascii_digit())).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scopes() -> Vec<String> {
        vec!["knowledge".to_owned(), "cli".to_owned()]
    }

    fn planned(work: &str) -> bool {
        work == "w0001"
    }

    #[test]
    fn taxonomy_values_become_scopes() {
        let text = "// Subsystem — ось подсистем\nslipway_core::declare_taxonomy! {\n    Subsystem => [Knowledge, Cli],\n    Team => [Core],\n}\n";
        assert_eq!(subsystem_scopes(text), ["knowledge", "cli"]);
        assert!(subsystem_scopes("Team => [Core]").is_empty());
    }

    #[test]
    fn well_formed_message_passes() {
        let message = "[FEAT](cli,knowledge): суть\n\nтело\n\nSlipway-Work: w0001\n";
        assert_eq!(
            problems(message, &scopes(), Some(&planned)),
            Vec::<String>::new()
        );
    }

    #[test]
    fn each_rule_names_its_violation() {
        let long = format!("[FEAT](cli): {}\n\nSlipway-Work: w0001", "я".repeat(70));
        let cases = [
            ("суть без типа\n\nSlipway-Work: w0001", "тема не по форме"),
            (
                "[FEATURE](cli): суть\n\nSlipway-Work: w0001",
                "тип [FEATURE]",
            ),
            ("[FEAT](wal): суть\n\nSlipway-Work: w0001", "область (wal)"),
            (
                "[FEAT](cli,,knowledge): суть\n\nSlipway-Work: w0001",
                "область ()",
            ),
            (
                "[FEAT](cli): суть.\n\nSlipway-Work: w0001",
                "точка в конце темы",
            ),
            // Тринадцать символов «[FEAT](cli): » и семьдесят «я».
            (long.as_str(), "тема длиннее 72 символов (83)"),
            (
                "[FEAT](cli): суть\nтело\n\nSlipway-Work: w0001",
                "пустая строка",
            ),
            ("[FEAT](cli): суть\n\nSlipway-Work: 1", "не по форме wNNNN"),
            (
                "[FEAT](cli): суть\n\nSlipway-Work: w00012",
                "не по форме wNNNN",
            ),
            ("[FEAT](cli): суть", "нет трейлера"),
            (
                "[FEAT](cli): суть\n\nSlipway-Work: w0099",
                "единицы работы w0099",
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
