//! Атаки на гарантии схемы слоя знания.
//!
//! Каждая атака записана `compile_fail`-тестом с кодом ошибки, и рядом с ней —
//! позитивный контроль той же формы. Без контроля отрицательный тест проходит
//! вакуумно: запись могла не собраться из-за опечатки, а не из-за атакуемого
//! правила. Коды ошибок rustdoc сверяет только на nightly; на stable остаётся
//! факт отказа, и от вакуума защищает один контроль.
//!
//! ## E5. Значение чужой оси таксономии там, где ждут подсистему
//!
//! ```compile_fail,E0308
//! slipway_core::declare_taxonomy! { Subsystem => [Storage], Team => [Core] }
//! fn attack(d: &slipway_knowledge::ArchitectureDecision) -> bool {
//!     d.subsystems.contains(&Team::Core)
//! }
//! ```
//!
//! Контроль:
//!
//! ```
//! slipway_core::declare_taxonomy! { Subsystem => [Storage], Team => [Core] }
//! fn control(d: &slipway_knowledge::ArchitectureDecision) -> bool {
//!     d.subsystems.contains(&Subsystem::Storage)
//! }
//! ```
//!
//! ## E6. Пустая строка в непустом списке авторов
//!
//! ```compile_fail,E0308
//! fn attack(d: &mut slipway_knowledge::ArchitectureDecision) {
//!     d.authors = slipway_core::nonempty![""];
//! }
//! ```
//!
//! Контроль:
//!
//! ```
//! fn control(d: &mut slipway_knowledge::ArchitectureDecision) {
//!     d.authors = slipway_core::nonempty_str!["Анищук Сергей"];
//! }
//! ```
//!
//! Пустое значение отвергается при вычислении статика, а не в рантайме:
//!
//! ```compile_fail,E0080
//! static AUTHORS: slipway_core::NonEmpty<slipway_core::NonEmptyStr> =
//!     slipway_core::nonempty_str!["  "];
//! ```
