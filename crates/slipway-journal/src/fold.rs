//! Свёртка журнала: стадия каждой работы, закрытые срезы и нарушения автомата.
//!
//! События упорядочиваются по времени, при равенстве — по имени файла.
//! Нарушающее событие не меняет стадию: следующее событие проверяется против
//! последнего законного.

use crate::event::{Event, Evidence, Kind, Subject};
use std::collections::BTreeMap;

/// Стадия работы после свёртки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Событий нет.
    Planned,
    /// Начата и не завершена.
    Started,
    /// Приземлена с доказательством на том же дереве.
    Landed,
    /// Приземлена до журнала: восстановлена из трейлеров истории.
    LandedFromHistory,
    /// Снята.
    Abandoned,
}

impl Stage {
    /// После этой стадии событий у работы нет.
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            Self::Landed | Self::LandedFromHistory | Self::Abandoned
        )
    }
}

/// Результат свёртки.
#[derive(Debug, Default)]
pub struct Journal {
    /// Стадия каждой работы, у которой есть законные события.
    pub works: BTreeMap<u32, Stage>,
    /// Закрытые срезы и файл закрывшего события.
    pub closed_slices: BTreeMap<u32, String>,
    /// Последнее законное событие каждой работы.
    pub last_event: BTreeMap<u32, String>,
    /// Деревья из событий gate каждой работы — доказательства.
    pub proofs: BTreeMap<u32, Vec<String>>,
}

impl Journal {
    /// Стадия работы; работа без событий запланирована.
    pub fn stage(&self, work: u32) -> Stage {
        self.works.get(&work).copied().unwrap_or(Stage::Planned)
    }
}

/// Нарушение: файл события и причина.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub reason: String,
}

/// Сворачивает события в состояние и перечисляет нарушения автомата.
pub fn fold(events: &[Event]) -> (Journal, Vec<Violation>) {
    let mut ordered: Vec<&Event> = events.iter().collect();
    ordered.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.file.cmp(&b.file)));
    let mut journal = Journal::default();
    let mut violations = Vec::new();
    for event in ordered {
        let outcome = match event.subject {
            Subject::Slice(slice) => close_slice(&mut journal, slice, event),
            Subject::Work(work) => advance_work(&mut journal, work, event),
        };
        if let Err(reason) = outcome {
            violations.push(Violation {
                file: event.file.clone(),
                reason,
            });
        }
    }
    (journal, violations)
}

fn close_slice(journal: &mut Journal, slice: u32, event: &Event) -> Result<(), String> {
    if let Some(previous) = journal.closed_slices.get(&slice) {
        return Err(format!(
            "срез {} уже закрыт событием {previous}",
            event.subject.id()
        ));
    }
    journal.closed_slices.insert(slice, event.file.clone());
    Ok(())
}

fn advance_work(journal: &mut Journal, work: u32, event: &Event) -> Result<(), String> {
    let id = event.subject.id();
    let stage = journal.stage(work);
    if stage.is_finished() {
        let last = journal.last_event.get(&work).map_or("", String::as_str);
        return Err(format!("работа {id} уже завершена событием {last}"));
    }
    let next = match (&event.kind, stage) {
        (Kind::Started, Stage::Planned) => Stage::Started,
        (Kind::Started, _) => {
            let last = journal.last_event.get(&work).map_or("", String::as_str);
            return Err(format!("работа {id} уже начата событием {last}"));
        }
        (Kind::Gate { tree, .. }, Stage::Started) => {
            journal.proofs.entry(work).or_default().push(tree.clone());
            Stage::Started
        }
        (Kind::Gate { .. }, _) => {
            return Err(format!("проверка работы {id} до её начала"));
        }
        (
            Kind::Landed {
                evidence: Evidence::Gate,
                tree,
                ..
            },
            Stage::Started,
        ) => {
            let proven = journal
                .proofs
                .get(&work)
                .is_some_and(|trees| trees.contains(tree));
            if !proven {
                return Err(format!(
                    "приземление работы {id} без доказательства: нет события gate с деревом {tree}"
                ));
            }
            Stage::Landed
        }
        (
            Kind::Landed {
                evidence: Evidence::Gate,
                ..
            },
            _,
        ) => return Err(format!("приземление работы {id} без её начала")),
        (
            Kind::Landed {
                evidence: Evidence::History,
                ..
            },
            Stage::Planned,
        ) => Stage::LandedFromHistory,
        (
            Kind::Landed {
                evidence: Evidence::History,
                ..
            },
            _,
        ) => {
            return Err(format!(
                "работа {id} уже в журнале: история — только для работы без событий"
            ));
        }
        (Kind::Abandoned { .. }, _) => Stage::Abandoned,
        (Kind::Closed, _) => {
            return Err("событие closed относится к срезу, а не к работе".to_owned())
        }
    };
    journal.works.insert(work, next);
    journal.last_event.insert(work, event.file.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TREE_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TREE_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn at(second: u32) -> String {
        format!("2026-09-11T03:15:{second:02}Z")
    }

    fn work(second: u32, kind: Kind) -> Event {
        Event::new(Subject::Work(22), at(second), kind)
    }

    fn gate(tree: &str) -> Kind {
        Kind::Gate {
            gate: "commit".into(),
            tree: tree.into(),
            verdict: "GATE OK".into(),
        }
    }

    fn landed(tree: &str, evidence: Evidence) -> Kind {
        Kind::Landed {
            commit: TREE_A.into(),
            tree: tree.into(),
            evidence,
        }
    }

    fn reasons(events: &[Event]) -> Vec<String> {
        fold(events).1.into_iter().map(|v| v.reason).collect()
    }

    #[test]
    fn started_gate_landed_on_the_same_tree_is_legal() {
        let events = [
            work(1, Kind::Started),
            work(2, gate(TREE_A)),
            work(3, landed(TREE_A, Evidence::Gate)),
        ];
        let (journal, violations) = fold(&events);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(journal.stage(22), Stage::Landed);
        assert_eq!(journal.stage(23), Stage::Planned);
    }

    #[test]
    fn order_comes_from_time_not_from_the_input() {
        let events = [
            work(3, landed(TREE_A, Evidence::Gate)),
            work(1, Kind::Started),
            work(2, gate(TREE_A)),
        ];
        assert!(fold(&events).1.is_empty());
    }

    #[test]
    fn illegal_transitions_name_their_reason() {
        let cases: [(&[Event], &str); 6] = [
            (&[work(1, gate(TREE_A))], "до её начала"),
            (&[work(1, landed(TREE_A, Evidence::Gate))], "без её начала"),
            (
                &[
                    work(1, Kind::Started),
                    work(2, gate(TREE_B)),
                    work(3, landed(TREE_A, Evidence::Gate)),
                ],
                "без доказательства",
            ),
            (
                &[work(1, Kind::Started), work(2, Kind::Started)],
                "уже начата",
            ),
            (
                &[
                    work(1, Kind::Started),
                    work(2, landed(TREE_A, Evidence::History)),
                ],
                "только для работы без событий",
            ),
            (
                &[
                    work(1, Kind::Abandoned { reason: "x".into() }),
                    work(2, Kind::Started),
                ],
                "уже завершена",
            ),
        ];
        for (events, reason) in cases {
            let found = reasons(events);
            assert!(
                found.iter().any(|r| r.contains(reason)),
                "ожидалось «{reason}», получено {found:?}"
            );
        }
    }

    #[test]
    fn history_lands_a_planned_work_and_slices_close_once() {
        let close = |second| Event::new(Subject::Slice(7), at(second), Kind::Closed);
        let events = [
            work(1, landed(TREE_A, Evidence::History)),
            close(2),
            close(3),
        ];
        let (journal, violations) = fold(&events);
        assert_eq!(journal.stage(22), Stage::LandedFromHistory);
        assert_eq!(journal.closed_slices.len(), 1);
        assert_eq!(violations.len(), 1);
        assert!(
            violations[0].reason.contains("уже закрыт"),
            "{violations:?}"
        );
    }
}
