//! Журнал слоя работы (решение 15): формат событий, автомат переходов,
//! свёртка в состояние.
//!
//! Событие — файл плоского подмножества TOML в каталоге журнала. Крейт без
//! зависимостей: его подключают скан при сборке реестра и инструмент в хуке, и
//! автомат переходов у них один.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

pub mod event;
pub mod fold;
pub mod format;
pub mod time;

pub use event::{Event, Evidence, Kind, Subject};
pub use fold::{fold, Journal, Stage, Violation};

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Разбирает файл события. `file` — путь от каталога журнала через `/`.
pub fn parse_event(file: &str, text: &str) -> Result<Event, Violation> {
    let record = format::parse(text).map_err(|error| Violation {
        file: file.to_owned(),
        reason: format!("строка {}: {}", error.line, error.reason),
    })?;
    Event::from_record(file, &record).map_err(|reason| Violation {
        file: file.to_owned(),
        reason,
    })
}

/// Читает события каталога журнала: файлы `*.toml` на любой глубине, по порядку
/// путей; прочие файлы пропускаются. Нечитаемый каталог — ошибка ввода-вывода,
/// неразобранный файл — нарушение с его именем.
pub fn read_dir(dir: &Path) -> io::Result<(Vec<Event>, Vec<Violation>)> {
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    let mut events = Vec::new();
    let mut violations = Vec::new();
    for (relative, path) in files {
        let bytes = fs::read(&path)?;
        let parsed = String::from_utf8(bytes)
            .map_err(|_| Violation {
                file: relative.clone(),
                reason: "файл не в UTF-8".to_owned(),
            })
            .and_then(|text| parse_event(&relative, &text));
        match parsed {
            Ok(event) => events.push(event),
            Err(violation) => violations.push(violation),
        }
    }
    Ok((events, violations))
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(root, &path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            files.push((relative, path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    /// События одной секунды упорядочиваются автоматом, а не именем файла:
    /// по алфавиту `gate` раньше `started`, и проверка шла бы до начала.
    #[test]
    fn same_second_events_follow_the_automaton() {
        let at = "2026-09-11T10:00:00Z".to_owned();
        let work = Subject::Work(22);
        let events = [
            Event::new(
                work,
                at.clone(),
                Kind::Landed {
                    commit: TREE.into(),
                    tree: TREE.into(),
                    evidence: Evidence::Gate,
                },
            ),
            Event::new(
                work,
                at.clone(),
                Kind::Gate {
                    gate: "commit".into(),
                    tree: TREE.into(),
                    verdict: "GATE OK".into(),
                },
            ),
            Event::new(work, at, Kind::Started),
        ];
        let (journal, violations) = fold(&events);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(journal.stage(22), Stage::Landed);
    }
}
