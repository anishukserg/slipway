//! Общие типы Slipway: ссылочные newtype, непустые списки, обратимость.
//!
//! Крейт не содержит ни схемы документов, ни схемы работы — только то, что
//! нужно обоим слоям. Зависимостей нет:
//! крейт подключают и атрибут разметки, и скан, работающий в `build.rs`.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

pub mod anchor_rules;
pub mod nonempty;
pub mod radius;
pub mod refs;
pub mod taxonomy;

pub use nonempty::{NonEmpty, NonEmptyStr};
pub use radius::{BlastRadius, Severity};
pub use refs::{
    AdrRef, AnchorId, BreakingRef, GateRef, RfcRef, SliceRef, SupersededRef, ThrustRef, WorkRef,
};
pub use taxonomy::{axis, Taxon};

/// Дата в const-контексте. Несуществующая дата — ошибка компиляции.
///
/// Раскрывается в `::chrono`, поэтому `chrono` обязан быть зависимостью
/// вызывающего крейта; сам `slipway-core` от него не зависит.
#[macro_export]
macro_rules! date {
    ($y:literal, $m:literal, $d:literal) => {
        match ::chrono::NaiveDate::from_ymd_opt($y, $m, $d) {
            Some(d) => d,
            None => panic!("slipway: некорректная дата"),
        }
    };
}
