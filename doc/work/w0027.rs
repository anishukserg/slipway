use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::{InquiryOutcome, WorkOrigin};
use std::num::NonZeroU16;

slipway_work::work!(27,
    title: NonEmptyStr::new("Гипотезы и критерии выхода пилота L1"),
    slice: crate::slice::s0008,
    origin: WorkOrigin::Inquiry {
        question: NonEmptyStr::new(
            "Какие гипотезы проверяет пилот L1 на стороннем проекте, чем измеряется каждая, какой результат прекращает пилот и кто выносит вердикт?"
        ),
        produces: InquiryOutcome::Adr,
        timebox_days: NonZeroU16::new(2).unwrap(),
    },
    taxon: taxon!(Subsystem, Methodology),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "ADR до начала пилота: гипотезы с метрикой и порогом каждая, срок, критерии отказа и владелец вердикта."
    ),
);
