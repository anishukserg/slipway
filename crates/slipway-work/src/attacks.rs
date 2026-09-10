//! Атаки на гарантии схемы слоя работы. Каждая — `compile_fail`-тест
//! с кодом ошибки и позитивным контролем той же формы.
//!
//! ## Исследование, объявляющее приземление кода
//!
//! ```compile_fail,E0559
//! use slipway_core::nonempty_str;
//! fn attack() -> slipway_work::WorkOrigin {
//!     slipway_work::WorkOrigin::Inquiry {
//!         question: *nonempty_str!["Нужен ли CI?"].first(),
//!         produces: slipway_work::InquiryOutcome::Adr,
//!         timebox_days: std::num::NonZeroU16::new(5).unwrap(),
//!         lands_code: true,
//!     }
//! }
//! ```
//!
//! Контроль:
//!
//! ```
//! use slipway_core::nonempty_str;
//! fn control() -> slipway_work::WorkOrigin {
//!     slipway_work::WorkOrigin::Inquiry {
//!         question: *nonempty_str!["Нужен ли CI?"].first(),
//!         produces: slipway_work::InquiryOutcome::Adr,
//!         timebox_days: std::num::NonZeroU16::new(5).unwrap(),
//!     }
//! }
//! assert!(!control().lands_code());
//! ```
//!
//! ## Исследование без срока
//!
//! ```compile_fail,E0080
//! static TIMEBOX: std::num::NonZeroU16 = std::num::NonZeroU16::new(0).unwrap();
//! ```
//!
//! ## Радиус работы выше потолка её среза
//!
//! ```compile_fail,E0080
//! use slipway_core::{BlastRadius, NonEmptyStr, RfcRef, SliceRef, ThrustRef};
//! use slipway_work::{Slice, WorkItem, WorkOrigin};
//! slipway_core::declare_taxonomy! { Subsystem => [Core] }
//! static SLICE: Slice = Slice {
//!     id: 1, title: NonEmptyStr::new("срез"), thrust: ThrustRef::__from_scan(1),
//!     outcome: NonEmptyStr::new("исход"), specification: RfcRef::__from_scan(1),
//!     max_radius: BlastRadius::Crate,
//! };
//! static WORK: WorkItem = WorkItem {
//!     id: 1, title: NonEmptyStr::new("работа"), slice: SliceRef::__from_scan(1),
//!     origin: WorkOrigin::Toil { justification: NonEmptyStr::new("рутина") },
//!     taxon: Subsystem::Core, radius: BlastRadius::Persistent,
//!     outcome: NonEmptyStr::new("готово"),
//! };
//! const _: () = assert!(slipway_work::radius_within_slice(&WORK, &[&SLICE]));
//! ```
//!
//! Контроль — тот же срез, радиус в пределах потолка:
//!
//! ```
//! use slipway_core::{BlastRadius, NonEmptyStr, RfcRef, SliceRef, ThrustRef};
//! use slipway_work::{Slice, WorkItem, WorkOrigin};
//! slipway_core::declare_taxonomy! { Subsystem => [Core] }
//! static SLICE: Slice = Slice {
//!     id: 1, title: NonEmptyStr::new("срез"), thrust: ThrustRef::__from_scan(1),
//!     outcome: NonEmptyStr::new("исход"), specification: RfcRef::__from_scan(1),
//!     max_radius: BlastRadius::Crate,
//! };
//! static WORK: WorkItem = WorkItem {
//!     id: 1, title: NonEmptyStr::new("работа"), slice: SliceRef::__from_scan(1),
//!     origin: WorkOrigin::Toil { justification: NonEmptyStr::new("рутина") },
//!     taxon: Subsystem::Core, radius: BlastRadius::Local,
//!     outcome: NonEmptyStr::new("готово"),
//! };
//! const _: () = assert!(slipway_work::radius_within_slice(&WORK, &[&SLICE]));
//! ```
//!
//! ## Уборка по ссылке на живое решение
//!
//! ```compile_fail,E0308
//! fn attack(live: slipway_core::AdrRef) -> slipway_work::WorkOrigin {
//!     slipway_work::WorkOrigin::Retirement(live)
//! }
//! ```
//!
//! Контроль:
//!
//! ```
//! fn control(retired: slipway_core::SupersededRef) -> slipway_work::WorkOrigin {
//!     slipway_work::WorkOrigin::Retirement(retired)
//! }
//! ```
