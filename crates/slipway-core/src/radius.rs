//! Обратимость как единственная ось риска. Спецификация — RFC-0002.

/// Мощность множества субъектов, которые должны согласованно измениться,
/// чтобы отменить изменение.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlastRadius {
    /// Приватный код внутри крейта. Всех пострадавших находит компилятор.
    Local,
    /// Публичный API крейта внутри workspace. Всех находит компилятор.
    Crate,
    /// Потребители за границей компиляции: плагины, другие команды.
    Contract,
    /// Данные, уже записанные в новом формате. `git revert` не возвращает.
    Persistent,
    /// Отмена физически невозможна.
    Irreversible,
}

/// Влияние на пользователя сейчас. Ось, независимая от обратимости:
/// критичный сбой может чиниться локально, а косметический дефект —
/// требовать изменения формата.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Cosmetic,
    Minor,
    Major,
    Critical,
    DataLoss,
}

impl BlastRadius {
    /// Требуемая независимость проверяющего растёт с обратимостью —
    /// это главное, что различает уровни (DO-178C: 5 → 33 цели
    /// с независимостью).
    pub const fn required_independence(self) -> Independence {
        match self {
            Self::Local => Independence::NotAuthor,
            Self::Crate => Independence::OutsideTeam,
            Self::Contract => Independence::BothContractSides,
            Self::Persistent => Independence::OutsideTaxon,
            Self::Irreversible => Independence::TwoHumanSignatures,
        }
    }

    /// Требует ли радиус принятого решения.
    pub const fn requires_decision(self) -> bool {
        matches!(self, Self::Contract | Self::Persistent | Self::Irreversible)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Independence {
    NotAuthor,
    OutsideTeam,
    BothContractSides,
    OutsideTaxon,
    /// Для агентов: проверяющий обязан быть другой моделью. Две сессии
    /// одной модели коррелированы в ошибках.
    DistinctModel,
    DistinctVendor,
    TwoHumanSignatures,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_is_ordered_by_reversibility() {
        assert!(BlastRadius::Local < BlastRadius::Persistent);
        assert!(BlastRadius::Persistent < BlastRadius::Irreversible);
    }

    #[test]
    fn decision_required_from_contract_up() {
        assert!(!BlastRadius::Crate.requires_decision());
        assert!(BlastRadius::Contract.requires_decision());
        assert!(BlastRadius::Persistent.requires_decision());
    }
}
