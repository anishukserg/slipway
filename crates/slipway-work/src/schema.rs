//! Схема слоя работы. Ссылки — пути к константам, порождённым сканом
//! реестров; правило, выразимое типом, валидатором не проверяется.

use slipway_core::{
    axis::Subsystem, AdrRef, BlastRadius, NonEmptyStr, RfcRef, SliceRef, SupersededRef, Taxon,
    ThrustRef,
};
use std::num::NonZeroU16;

/// Направление: исход на месяцы, ради которого режутся срезы.
#[derive(Debug)]
pub struct Thrust {
    pub id: u32,
    pub title: NonEmptyStr,
    /// Наблюдаемый исход. Формулировка обязана быть проверяемой, а не оценочной.
    pub outcome: NonEmptyStr,
}

/// Срез: вертикальное изменение на недели, закрываемое по исходу, а не по календарю.
#[derive(Debug)]
pub struct Slice {
    pub id: u32,
    pub title: NonEmptyStr,
    pub thrust: ThrustRef,
    pub outcome: NonEmptyStr,
    /// Спецификация, из которой срез выведен.
    pub specification: RfcRef,
    /// Потолок радиуса для единиц работы среза.
    pub max_radius: BlastRadius,
}

/// Единица работы: от половины дня до недели.
#[derive(Debug)]
pub struct WorkItem {
    pub id: u32,
    pub title: NonEmptyStr,
    pub slice: SliceRef,
    pub origin: WorkOrigin,
    /// Ровно одна подсистема: работа на две обязана быть разделена.
    pub taxon: Taxon<Subsystem>,
    pub radius: BlastRadius,
    /// Чем подтверждается готовность. Пока словами: каталог гейтов заменит
    /// это поле ссылками на гейты.
    pub outcome: NonEmptyStr,
}

/// Происхождение работы: тип задачи и её обоснование — одно поле.
///
/// Перечисление неполное относительно части III: `Mandate`, `Migration` и
/// `DebtService` появятся вместе с реестрами, на которые они ссылаются.
/// `#[non_exhaustive]` не ставится намеренно: новый вариант обязан сломать
/// сборку валидатора, а не молча остаться без правила.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkOrigin {
    /// Реализация принятого решения.
    Decision(AdrRef),
    /// Реализация доменной спецификации.
    Specification(RfcRef),
    /// Расхождение фактического с заявленным. Реестра инвариантов с путями
    /// пока нет, поэтому нарушенное утверждение названо текстом рядом со
    /// спецификацией, где оно объявлено.
    Divergence { specification: RfcRef, violated: NonEmptyStr },
    /// Снятие неопределённости. Поля, объявляющего приземление кода, нет:
    /// исследование, уезжающее в main, невыразимо.
    Inquiry { question: NonEmptyStr, produces: InquiryOutcome, timebox_days: NonZeroU16 },
    /// Удаление кода, замещённого решением. Принимает только ссылку
    /// из модуля `superseded`.
    Retirement(SupersededRef),
    /// Рутина без архитектурного следа.
    Toil { justification: NonEmptyStr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InquiryOutcome {
    Adr,
    Rfc,
    Measurement,
}

impl WorkOrigin {
    /// Приземляет ли работа код. Выводится из варианта, а не хранится.
    pub const fn lands_code(&self) -> bool {
        !matches!(self, Self::Inquiry { .. })
    }
}

/// Радиус работы не выше потолка её среза.
///
/// Функция `const`: скан порождает утверждение на каждую единицу работы,
/// и нарушение становится ошибкой вычисления константы, а не находкой
/// валидатора. Правило связывает две записи, но вычисляется компилятором.
pub const fn radius_within_slice(work: &WorkItem, slices: &[&Slice]) -> bool {
    let mut i = 0;
    while i < slices.len() {
        if slices[i].id == work.slice.index() {
            return work.radius as u8 <= slices[i].max_radius as u8;
        }
        i += 1;
    }
    false
}
