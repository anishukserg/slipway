//! Рабочий код продукта. Разметка `#[doc_anchor]` связывает фрагменты
//! с решениями: удаление размеченного элемента ломает ссылку из решения.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

// Разметка — атрибут-маркер: он проверяет свои аргументы и возвращает элемент
// без изменений. Константы ссылок порождает скан (RFC-0001), поэтому ставить
// разметку можно где угодно, включая методы внутри `impl`.
use slipway_derive::doc_anchor;

#[doc_anchor(id = "plan-ir", mode = "snippet")]
pub struct PlanIr {
    pub steps: Vec<PlanStep>,
    pub source_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanStep {
    Scan { table: u32 },
    Filter { column: u32 },
}

impl PlanIr {
    #[doc_anchor(id = "plan-ir-nonempty", mode = "embed")]
    pub fn is_executable(&self) -> bool {
        !self.steps.is_empty()
    }
}

pub mod page {
    use slipway_derive::doc_anchor;

    /// Заголовок страницы данных: формат версии 4.
    #[doc_anchor(id = "page-header", mode = "ref")]
    pub struct PageHeader {
        pub version: u8,
        pub flags: u16,
        pub checksum: u32,
    }
}
