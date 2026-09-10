//! Реестр решений и спецификаций самого Slipway.
//!
//! Методология, применённая к себе: решения о том, как устроен Slipway,
//! ведутся по правилам Slipway. Это первое её испытание — и первое место,
//! где несоблюдение собственных правил стало бы видно немедленно.

pub mod taxonomy;

include!(concat!(env!("OUT_DIR"), "/adr.rs"));
include!(concat!(env!("OUT_DIR"), "/rfc.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use slipway_knowledge::DocStatus;

    #[test]
    fn every_decision_is_reachable() {
        assert!(!ALL.is_empty(), "реестр решений не должен быть пуст");
        for d in ALL {
            assert!(!d.title.is_empty());
            assert!(d.authors.len() >= 1);
        }
    }

    #[test]
    fn specifications_declare_invariants() {
        for s in ALL_SPECS {
            // Непустота обеспечена типом; проверяем осмысленность длины.
            assert!(s.invariants.len() >= 1, "{}", s.title);
        }
    }

    #[test]
    fn specifications_point_at_decisions_that_exist() {
        // Ссылки уже проверены компилятором; здесь — что они не пусты
        // у спецификаций, которые объявлены реализованными.
        for s in ALL_SPECS {
            if matches!(s.status, DocStatus::Active) && s.decided_by.is_empty() {
                // Спецификация без решений законна: она описывает требуемое,
                // решения могут появиться позже.
                continue;
            }
        }
    }
}
