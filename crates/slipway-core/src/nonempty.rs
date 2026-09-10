//! Непустой список: нарушение выражается типом, а не проверяется.
//!
//! Приём из решения 4. Значение `NonEmpty` невозможно
//! получить пустым, поэтому правило «список обязан быть непуст» не нужно
//! проверять — оно невыразимо.
//!
//! Для строк этого мало: `nonempty![""]` непуст как список и пуст по смыслу
//! (атака E6). Поэтому списки имён и шагов строятся из [`NonEmptyStr`] через
//! [`nonempty_str!`](crate::nonempty_str), где пустое значение — ошибка вычисления константы.

/// Список, содержащий хотя бы один элемент, конструируемый в `const`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonEmpty<T: 'static> {
    head: T,
    tail: &'static [T],
}

impl<T: 'static> NonEmpty<T> {
    pub const fn new(head: T, tail: &'static [T]) -> Self {
        Self { head, tail }
    }

    pub const fn first(&self) -> &T {
        &self.head
    }

    pub const fn len(&self) -> usize {
        1 + self.tail.len()
    }

    /// Всегда `false`. Метод существует ради читаемости вызывающего кода
    /// и как исполняемое доказательство инварианта.
    pub const fn is_empty(&self) -> bool {
        false
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }
}

/// Строка, в которой есть хотя бы один непробельный символ.
///
/// Конструктор — `const fn`: в статике и в [`nonempty_str!`](crate::nonempty_str) пустое значение
/// отвергается при компиляции (E0080), в рантайме — паникой.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmptyStr(&'static str);

impl NonEmptyStr {
    pub const fn new(text: &'static str) -> Self {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if !bytes[i].is_ascii_whitespace() {
                return Self(text);
            }
            i += 1;
        }
        panic!("slipway: пустая строка там, где требуется содержание");
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Конструктор непустого списка. Пустой список — ошибка раскрытия макроса.
#[macro_export]
macro_rules! nonempty {
    [$head:expr $(, $tail:expr)* $(,)?] => {
        $crate::NonEmpty::new($head, &[$($tail),*])
    };
}

/// Непустой список непустых строк. Пустой список — ошибка раскрытия макроса,
/// пустая строка — ошибка вычисления константы в любом контексте: значения
/// вычисляются в `const`, даже когда макрос стоит в теле функции.
#[macro_export]
macro_rules! nonempty_str {
    [$head:literal $(, $tail:literal)* $(,)?] => {{
        const HEAD: $crate::NonEmptyStr = $crate::NonEmptyStr::new($head);
        const TAIL: &[$crate::NonEmptyStr] = &[$($crate::NonEmptyStr::new($tail)),*];
        $crate::NonEmpty::new(HEAD, TAIL)
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    static ONE: NonEmpty<&str> = nonempty!["первый"];
    static THREE: NonEmpty<&str> = nonempty!["a", "b", "c"];
    static AUTHORS: NonEmpty<NonEmptyStr> = nonempty_str!["Анищук Сергей", "Второй автор"];

    #[test]
    fn const_construction_works() {
        assert_eq!(ONE.len(), 1);
        assert_eq!(THREE.len(), 3);
        assert!(!THREE.is_empty());
    }

    #[test]
    fn iterates_head_then_tail() {
        assert_eq!(THREE.iter().copied().collect::<Vec<_>>(), ["a", "b", "c"]);
    }

    #[test]
    fn nonempty_str_list_keeps_order() {
        let names: Vec<_> = AUTHORS.iter().map(NonEmptyStr::as_str).collect();
        assert_eq!(names, ["Анищук Сергей", "Второй автор"]);
    }

    #[test]
    #[should_panic(expected = "пустая строка")]
    fn blank_string_is_rejected_at_runtime_too() {
        let blank: &'static str = String::from(" \t").leak();
        let _ = NonEmptyStr::new(blank);
    }
}
