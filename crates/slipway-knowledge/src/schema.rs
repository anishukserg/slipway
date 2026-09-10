//! Схема решения. Правила, которые в обычной методологии проверялись бы
//! валидатором, здесь выражены типами и потому не проверяются вовсе.
//! Атаки на эти правила и их контроли — [`crate::attacks`].

use chrono::NaiveDate;
use slipway_core::{axis::Subsystem, AdrRef, AnchorId, NonEmpty, NonEmptyStr, RfcRef, Taxon};

/// Статус документа.
///
/// `SupersededBy` несёт ссылку на замещающее решение: замещение без указания,
/// чем именно замещено, невыразимо.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocStatus {
    Draft,
    Active,
    Deprecated,
    SupersededBy(AdrRef),
}

/// Ломающее изменение и инструкции миграции — одна конструкция.
///
/// Приём из решения 4: прежде здесь была пара полей
/// `breaking: bool` + `migration_notes: &[&str]`, согласованность которых
/// приходилось сверять валидатором. Теперь инструкции живут **внутри**
/// варианта `Yes` и непусты по типу, поэтому отметить изменение ломающим
/// и не приложить инструкции нельзя синтаксически. Каждый шаг тоже непуст:
/// список пустых строк формально непуст, но ничего не предписывает.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breaking {
    No,
    Yes { migration: NonEmpty<NonEmptyStr> },
}

impl Breaking {
    pub const fn is_breaking(&self) -> bool {
        matches!(self, Self::Yes { .. })
    }
}

#[derive(Debug)]
pub struct ArchitectureDecision {
    pub id: u32,
    pub title: &'static str,
    pub status: DocStatus,
    /// Только значения оси подсистем: значение другой оси — ошибка типа.
    pub subsystems: &'static [Taxon<Subsystem>],
    pub context: &'static str,
    pub decision: &'static str,
    pub trade_offs: &'static [&'static str],
    pub constraints: &'static [&'static str],
    /// Непусто по типу, и каждое имя непусто: запись без авторов невыразима.
    pub authors: NonEmpty<NonEmptyStr>,
    pub decided_at: NaiveDate,
    pub breaking: Breaking,
    /// Ссылки на разметку кода. Каждая — путь к константе, порождённой
    /// сканом исходников: удалили разметку, ссылка не собирается.
    pub code_refs: &'static [AnchorId],
    pub related_rfcs: &'static [RfcRef],
}

#[cfg(test)]
mod tests {
    use super::*;
    use slipway_core::nonempty_str;

    #[test]
    fn breaking_carries_its_migration() {
        let b = Breaking::Yes {
            migration: nonempty_str!["перекодировать страницы"],
        };
        assert!(b.is_breaking());
        assert_eq!(
            b,
            Breaking::Yes {
                migration: nonempty_str!["перекодировать страницы"]
            }
        );
    }

    #[test]
    fn superseded_names_its_successor() {
        let s = DocStatus::SupersededBy(AdrRef::__from_scan(7));
        assert!(matches!(s, DocStatus::SupersededBy(r) if r.index() == 7));
    }
}

/// Доменная спецификация: что требуется, в отличие от решения — как сделано.
///
/// Отвечает на вопрос «чего мы хотим», решение — на вопрос «как решили».
/// Работа выводится либо из спецификации, либо из решения.
#[derive(Debug)]
pub struct DomainSpecification {
    pub id: u32,
    pub title: &'static str,
    pub status: DocStatus,
    pub target: &'static [Taxon<Subsystem>],
    /// Наблюдаемый результат. Формулировка обязана быть проверяемой
    /// инструментом, а не оценочной.
    pub goal: &'static str,
    pub input_contract: &'static str,
    pub output_contract: &'static str,
    /// Утверждения, которые обязаны держаться. Непусто по типу: спецификация
    /// без единого инварианта ничего не требует.
    pub invariants: NonEmpty<NonEmptyStr>,
    pub authors: NonEmpty<NonEmptyStr>,
    pub decided_at: NaiveDate,
    /// Решения, реализующие эту спецификацию.
    pub decided_by: &'static [AdrRef],
}
