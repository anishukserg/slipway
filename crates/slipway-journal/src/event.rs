//! События журнала: предмет, время, вид и поля вида.

use crate::format::{self, Record};
use crate::time;

/// Предмет события: единица работы или срез.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Subject {
    Work(u32),
    Slice(u32),
}

impl Subject {
    /// Идентификатор в плане: `w0022` или `s0007`.
    pub fn id(self) -> String {
        match self {
            Self::Work(number) => format!("w{number:04}"),
            Self::Slice(number) => format!("s{number:04}"),
        }
    }

    /// Разбирает `w0022` или `s0007`.
    pub fn parse(text: &str) -> Option<Subject> {
        let digits = text.get(1..)?;
        if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let number = digits.parse().ok()?;
        match text.as_bytes().first() {
            Some(b'w') => Some(Self::Work(number)),
            Some(b's') => Some(Self::Slice(number)),
            _ => None,
        }
    }
}

/// Чем подтверждено приземление.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// Событием gate на том же дереве.
    Gate,
    /// Восстановлено из трейлеров истории, когда журнала ещё не было.
    History,
}

/// Вид события и его поля.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Started,
    Gate {
        gate: String,
        tree: String,
        verdict: String,
    },
    Landed {
        commit: String,
        tree: String,
        evidence: Evidence,
    },
    Abandoned {
        reason: String,
    },
    Closed,
}

impl Kind {
    /// Имя вида в файле и в имени файла.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Gate { .. } => "gate",
            Self::Landed { .. } => "landed",
            Self::Abandoned { .. } => "abandoned",
            Self::Closed => "closed",
        }
    }
}

/// Событие журнала.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Путь файла от каталога журнала через `/` — имя события в сообщениях.
    pub file: String,
    pub subject: Subject,
    /// Время в UTC, `ГГГГ-ММ-ДДTЧЧ:ММ:ССZ`.
    pub at: String,
    pub kind: Kind,
}

impl Event {
    /// Новое событие с каноническим путём файла.
    pub fn new(subject: Subject, at: String, kind: Kind) -> Event {
        Event {
            file: relative_path(subject, &at, &kind, 0),
            subject,
            at,
            kind,
        }
    }

    /// Событие из разобранного файла. Поля проверяются строго: лишнее,
    /// недостающее или неверно записанное поле — ошибка с названием поля.
    pub fn from_record(file: &str, record: &Record) -> Result<Event, String> {
        let name = record.get("event").ok_or("нет поля event")?;
        let (subject_key, own): (&str, &[&str]) = match name {
            "started" => ("work", &[]),
            "gate" => ("work", &["gate", "tree", "verdict"]),
            "landed" => ("work", &["commit", "tree", "evidence"]),
            "abandoned" => ("work", &["reason"]),
            "closed" => ("slice", &[]),
            other => {
                return Err(format!(
                    "неизвестное событие `{other}`: started, gate, landed, abandoned, closed"
                ))
            }
        };
        for (key, _) in &record.fields {
            let known = key == "event" || key == "at" || key == subject_key;
            if !known && !own.contains(&key.as_str()) {
                return Err(format!("лишнее поле `{key}` у события {name}"));
            }
        }

        let subject_text = record
            .get(subject_key)
            .ok_or_else(|| format!("нет поля {subject_key} у события {name}"))?;
        let subject = Subject::parse(subject_text)
            .filter(|subject| {
                matches!(
                    (subject, subject_key),
                    (Subject::Work(_), "work") | (Subject::Slice(_), "slice")
                )
            })
            .ok_or_else(|| {
                let example = if subject_key == "work" {
                    "w0001"
                } else {
                    "s0001"
                };
                format!("поле {subject_key} — идентификатор вида {example}, а не «{subject_text}»")
            })?;

        let at = record.get("at").ok_or("нет поля at")?;
        if !time::is_timestamp(at) {
            return Err(format!(
                "поле at — время UTC вида 2026-09-11T03:15:00Z, а не «{at}»"
            ));
        }

        let required = |key: &str| {
            record
                .get(key)
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("нет непустого поля {key} у события {name}"))
        };
        let hash = |key: &str| {
            let value = required(key)?;
            if is_hash(&value) {
                Ok(value)
            } else {
                Err(format!(
                    "поле {key} — хэш git из 40 или 64 строчных шестнадцатеричных символов"
                ))
            }
        };
        let kind = match name {
            "started" => Kind::Started,
            "gate" => Kind::Gate {
                gate: required("gate")?,
                tree: hash("tree")?,
                verdict: required("verdict")?,
            },
            "landed" => Kind::Landed {
                commit: hash("commit")?,
                tree: hash("tree")?,
                evidence: match record.get("evidence") {
                    None => Evidence::Gate,
                    Some("history") => Evidence::History,
                    Some(other) => {
                        return Err(format!("поле evidence — только history, а не «{other}»"))
                    }
                },
            },
            "abandoned" => Kind::Abandoned {
                reason: required("reason")?,
            },
            _ => Kind::Closed,
        };

        let event = Event {
            file: file.to_owned(),
            subject,
            at: at.to_owned(),
            kind,
        };
        if !event.file_matches() {
            return Err(format!(
                "имя файла не совпадает с событием: ожидалось {}",
                relative_path(event.subject, &event.at, &event.kind, 0)
            ));
        }
        Ok(event)
    }

    /// Текст файла события.
    pub fn to_text(&self) -> String {
        let subject_key = match self.subject {
            Subject::Work(_) => "work",
            Subject::Slice(_) => "slice",
        };
        let id = self.subject.id();
        let mut fields: Vec<(&str, &str)> = vec![
            ("event", self.kind.name()),
            (subject_key, &id),
            ("at", &self.at),
        ];
        match &self.kind {
            Kind::Started | Kind::Closed => {}
            Kind::Gate {
                gate,
                tree,
                verdict,
            } => fields.extend([
                ("gate", gate.as_str()),
                ("tree", tree),
                ("verdict", verdict),
            ]),
            Kind::Landed {
                commit,
                tree,
                evidence,
            } => {
                fields.extend([("commit", commit.as_str()), ("tree", tree)]);
                if *evidence == Evidence::History {
                    fields.push(("evidence", "history"));
                }
            }
            Kind::Abandoned { reason } => fields.push(("reason", reason)),
        }
        format::render(&fields)
    }

    /// Путь файла соответствует предмету, времени и виду события. При
    /// совпадении времени допускается числовой суффикс `-N`.
    fn file_matches(&self) -> bool {
        let canonical = relative_path(self.subject, &self.at, &self.kind, 0);
        if self.file == canonical {
            return true;
        }
        let stem = canonical.strip_suffix(".toml").unwrap_or(&canonical);
        self.file
            .strip_prefix(stem)
            .and_then(|rest| rest.strip_prefix('-'))
            .and_then(|rest| rest.strip_suffix(".toml"))
            .is_some_and(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
    }
}

/// Путь файла события от каталога журнала; `attempt` больше нуля добавляет
/// суффикс для событий с одинаковым временем.
pub fn relative_path(subject: Subject, at: &str, kind: &Kind, attempt: u32) -> String {
    let compact: String = at.chars().filter(|c| *c != '-' && *c != ':').collect();
    let name = kind.name();
    let id = subject.id();
    if attempt == 0 {
        format!("{id}/{compact}-{name}.toml")
    } else {
        format!("{id}/{compact}-{name}-{attempt}.toml")
    }
}

fn is_hash(text: &str) -> bool {
    matches!(text.len(), 40 | 64)
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format;

    const TREE: &str = "0123456789abcdef0123456789abcdef01234567";

    fn event(file: &str, text: &str) -> Result<Event, String> {
        Event::from_record(file, &format::parse(text).unwrap())
    }

    #[test]
    fn every_kind_round_trips_through_its_file() {
        let at = "2026-09-11T03:15:00Z".to_owned();
        for (subject, kind) in [
            (Subject::Work(22), Kind::Started),
            (
                Subject::Work(22),
                Kind::Gate {
                    gate: "commit".into(),
                    tree: TREE.into(),
                    verdict: "GATE OK (11 из 11)".into(),
                },
            ),
            (
                Subject::Work(22),
                Kind::Landed {
                    commit: TREE.into(),
                    tree: TREE.into(),
                    evidence: Evidence::History,
                },
            ),
            (
                Subject::Work(22),
                Kind::Abandoned {
                    reason: "замещена работой 23".into(),
                },
            ),
            (Subject::Slice(7), Kind::Closed),
        ] {
            let written = Event::new(subject, at.clone(), kind);
            let read = event(&written.file, &written.to_text()).unwrap();
            assert_eq!(read, written);
        }
        assert_eq!(
            Event::new(Subject::Work(22), at, Kind::Started).file,
            "w0022/20260911T031500Z-started.toml"
        );
    }

    #[test]
    fn fields_are_checked_strictly() {
        let file = "w0022/20260911T031500Z-started.toml";
        let base = "work = \"w0022\"\nat = \"2026-09-11T03:15:00Z\"\n";
        for (text, reason) in [
            (base.to_owned(), "нет поля event"),
            (format!("event = \"begun\"\n{base}"), "неизвестное событие"),
            (
                format!("event = \"started\"\n{base}owner = \"x\"\n"),
                "лишнее поле `owner`",
            ),
            (
                "event = \"started\"\nwork = \"s0022\"\nat = \"2026-09-11T03:15:00Z\"\n".to_owned(),
                "идентификатор вида w0001",
            ),
            (
                "event = \"started\"\nwork = \"w0022\"\nat = \"вчера\"\n".to_owned(),
                "время UTC",
            ),
        ] {
            let error = event(file, &text).expect_err(&text);
            assert!(error.contains(reason), "{text}: {error}");
        }
        let gate = "event = \"gate\"\nwork = \"w0022\"\nat = \"2026-09-11T03:15:00Z\"\ngate = \"commit\"\ntree = \"XYZ\"\nverdict = \"ok\"\n";
        let error = event("w0022/20260911T031500Z-gate.toml", gate).unwrap_err();
        assert!(error.contains("поле tree — хэш git"), "{error}");
    }

    #[test]
    fn file_name_must_match_the_event() {
        let text = "event = \"started\"\nwork = \"w0022\"\nat = \"2026-09-11T03:15:00Z\"\n";
        assert!(event("w0022/20260911T031500Z-started-2.toml", text).is_ok());
        let error = event("w0023/20260911T031500Z-started.toml", text).unwrap_err();
        assert!(
            error.contains("ожидалось w0022/20260911T031500Z-started.toml"),
            "{error}"
        );
    }
}
