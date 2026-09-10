//! Демо-реестр решений. Показывает механизм ссылок-путей.

pub mod taxonomy;

/// Порождается build.rs: объявления модулей, константы ссылок, список.
include!(concat!(env!("OUT_DIR"), "/registry.rs"));

/// Порождается build.rs: константы разметки кода продукта.
include!(concat!(env!("OUT_DIR"), "/anchors.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use slipway_knowledge::DocStatus;

    #[test]
    fn registry_is_complete() {
        assert_eq!(ALL.len(), 3);
    }

    #[test]
    fn reference_resolves_to_existing_decision() {
        // adr::a0002 — путь к константе, порождённой сканом.
        // Опечатка здесь = unresolved path, а не молчаливо неверная ссылка.
        assert_eq!(adr::a0002.index(), 2);
        assert_eq!(adr::a0003.index(), 3);
    }

    #[test]
    fn superseded_module_holds_only_retired_decisions() {
        // Решение 1 замещено — константа есть.
        assert_eq!(superseded::a0001.index(), 1);
        // Для действующего решения 2 константы в этом модуле НЕТ,
        // поэтому уборка живого кода невыразима:
        //     WorkOrigin::Retirement(superseded::a0002)  // unresolved path
    }

    #[test]
    fn work_origin_cannot_point_at_live_decision() {
        // Тип принимает только SupersededRef, а такие константы порождаются
        // лишь для замещённых решений.
        fn retire(_: slipway_core::SupersededRef) {}
        retire(superseded::a0001);
    }

    #[test]
    fn decision_references_real_code() {
        // Решение 2 ссылается на разметку в коде продукта.
        // Удалить #[doc_anchor(id = "plan-ir")] из demo-product — эта строка
        // перестанет резолвиться, потому что константа исчезнет при скане.
        assert_eq!(a0002::DECISION.code_refs, &[anchor::plan_ir]);
    }

    #[test]
    fn breaking_decision_carries_migration() {
        let d = &a0003::DECISION;
        match d.breaking {
            slipway_knowledge::Breaking::Yes { migration } => {
                assert_eq!(migration.len(), 2);
            }
            slipway_knowledge::Breaking::No => panic!("ожидалось ломающее решение"),
        }
        assert!(matches!(d.status, DocStatus::Active));
    }
}
