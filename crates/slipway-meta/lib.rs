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

    /// Непустота авторов и инвариантов обеспечена типом; здесь проверяется
    /// только то, чего тип не выражает: реестры не пусты, у записей есть
    /// заголовки.
    #[test]
    fn registries_are_not_empty() {
        assert!(!ALL.is_empty(), "реестр решений пуст");
        assert!(!ALL_SPECS.is_empty(), "реестр спецификаций пуст");
        for d in ALL {
            assert!(!d.title.trim().is_empty(), "решение {} без заголовка", d.id);
        }
        for s in ALL_SPECS {
            assert!(
                !s.title.trim().is_empty(),
                "спецификация {} без заголовка",
                s.id
            );
        }
    }
}
