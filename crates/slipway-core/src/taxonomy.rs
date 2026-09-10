//! Таксономия: продукт объявляет допустимые значения, `taxon!` резолвится
//! путём к константе — опечатка становится ошибкой компилятора.
//! Спецификация — RFC-0001.
//!
//! Ось — часть типа значения: `Taxon<axis::Subsystem>` и `Taxon<axis::Team>`
//! различны, поэтому значение одной оси не встаёт в поле другой (атака E5,
//! `slipway_knowledge::attacks`).

use std::{fmt, hash, marker::PhantomData};

/// Оси, на которые опирается сама схема методологии.
///
/// Уровень соответствия, владение и радиус объявляются на подсистему,
/// команда — ось владения. Оси продукта сверх этих двух не поддерживаются:
/// схема на них не ссылается, а объявлять их впрок — лишнее обязательство.
pub mod axis {
    /// Подсистема: единица владения, уровня соответствия и радиуса.
    pub enum Subsystem {}
    /// Команда, которой принадлежит подсистема.
    pub enum Team {}
}

/// Значение оси таксономии `A`.
pub struct Taxon<A> {
    name: &'static str,
    axis: PhantomData<fn() -> A>,
}

impl<A> Taxon<A> {
    /// Только для `declare_taxonomy!`. В тексте реестра вызов отвергается сканом.
    #[doc(hidden)]
    pub const fn __new_unchecked(name: &'static str) -> Self {
        Self {
            name,
            axis: PhantomData,
        }
    }

    pub const fn as_str(&self) -> &'static str {
        self.name
    }
}

// Реализации без ограничений на `A`: ось — маркер без значений, и границы
// `A: Clone` и т. п., которые вывел бы `derive`, были бы бессмысленны.
impl<A> Clone for Taxon<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Taxon<A> {}

impl<A> PartialEq for Taxon<A> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl<A> Eq for Taxon<A> {}

impl<A> hash::Hash for Taxon<A> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl<A> fmt::Debug for Taxon<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Taxon").field(&self.name).finish()
    }
}

/// Объявление значений осей таксономии продукта.
///
/// Порождает модуль констант на каждую ось, поэтому `taxon!(Subsystem, Foo)`
/// при опечатке даёт `unresolved path`, а не проходит молча. Имя оси обязано
/// быть одним из [`axis`], иначе ошибка компиляции.
#[macro_export]
macro_rules! declare_taxonomy {
    ($($axis:ident => [$($value:ident),* $(,)?]),* $(,)?) => {
        $(
            #[allow(non_upper_case_globals, non_snake_case)]
            pub mod $axis {
                /// Ось значений этого модуля.
                pub type Axis = $crate::axis::$axis;
                $(pub const $value: $crate::Taxon<Axis> = $crate::Taxon::__new_unchecked(stringify!($value));)*
                pub const ALL: &[$crate::Taxon<Axis>] = &[$($value),*];
            }
        )*
    };
}

/// Значение таксономии. Резолвится как путь: `taxon!(Subsystem, Storage)`.
#[macro_export]
macro_rules! taxon {
    ($axis:ident, $value:ident) => {
        $axis::$value
    };
}

#[cfg(test)]
mod tests {
    crate::declare_taxonomy! {
        Subsystem => [Executor, Storage],
        Team => [CoreDb],
    }

    #[test]
    fn taxon_resolves_by_path() {
        assert_eq!(taxon!(Subsystem, Storage).as_str(), "Storage");
        assert_eq!(Subsystem::ALL.len(), 2);
        assert_eq!(Team::ALL, &[Team::CoreDb]);
    }
}
