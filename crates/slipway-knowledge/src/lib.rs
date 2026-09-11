//! Слой знания: что решено и почему. Спецификация — RFC-0001 в документах Slipway, `doc/rfc/r0001.rs`.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

pub mod attacks;
pub mod schema;
pub use schema::{ArchitectureDecision, Breaking, DocStatus, DomainSpecification};

/// Регистрация решения. Файл обязан называться по идентификатору
/// (`a0007.rs` для 7): скан отвергает любое другое имя, поэтому два решения
/// с одним идентификатором в одном каталоге невыразимы.
#[macro_export]
macro_rules! adr {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        $crate::paste_adr!($id, $($field: $value),*);
    };
}

/// Регистрация доменной спецификации.
#[macro_export]
macro_rules! rfc {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        /// Запись спецификации этого файла.
        pub static SPEC: $crate::DomainSpecification = $crate::DomainSpecification {
            id: $id,
            $($field: $value),*
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! paste_adr {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        /// Запись решения этого файла.
        pub static DECISION: $crate::ArchitectureDecision = $crate::ArchitectureDecision {
            id: $id,
            $($field: $value),*
        };
    };
}
