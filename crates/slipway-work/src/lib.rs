//! Слой работы: направления, срезы, единицы работы. Спецификация — RFC-0002 в документах Slipway, `doc/rfc/r0002.rs`.
//!
//! Минимальный срез для самообкатки: только поля, без которых ломается
//! инвариант «работа имеет основание». Статуса у записей нет — по инварианту 3
//! состояние работы есть свёртка журнала. Журнала пока нет, и до него
//! состояние восстанавливается из истории git; это известное ограничение,
//! а не норма.
//!
//! Чего нет и почему: `severity`, `service_class`, `size`, `depends_on`,
//! `touches`, ссылок на гейты. Они появятся вместе с механизмами, которые их
//! потребляют: каталогом гейтов, потоком, границами задачи. Поле без
//! потребителя — налог на заведение работы без выигрыша.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

pub mod attacks;
mod schema;

pub use schema::{radius_within_slice, InquiryOutcome, Slice, Thrust, WorkItem, WorkOrigin};

/// Регистрация направления. Файл обязан называться по идентификатору (`t0001.rs`).
#[macro_export]
macro_rules! thrust {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        pub static THRUST: $crate::Thrust = $crate::Thrust { id: $id, $($field: $value),* };
    };
}

/// Регистрация среза. Файл обязан называться по идентификатору (`s0001.rs`).
#[macro_export]
macro_rules! slice {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        pub static SLICE: $crate::Slice = $crate::Slice { id: $id, $($field: $value),* };
    };
}

/// Регистрация единицы работы. Файл обязан называться по идентификатору (`w0001.rs`).
#[macro_export]
macro_rules! work {
    ($id:literal, $($field:ident : $value:expr),* $(,)?) => {
        pub static WORK: $crate::WorkItem = $crate::WorkItem { id: $id, $($field: $value),* };
    };
}
