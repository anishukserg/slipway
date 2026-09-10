use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::{InquiryOutcome, WorkOrigin};
use std::num::NonZeroU16;

slipway_work::work!(2,
    title: NonEmptyStr::new("Проза, пережившая переименование символа (атака E3)"),
    slice: crate::slice::s0001,
    origin: WorkOrigin::Inquiry {
        question: NonEmptyStr::new(
            "Как проверять имена символов в прозе решений так, чтобы переименование размеченного символа ломало документ, а проза не превращалась в код?"
        ),
        produces: InquiryOutcome::Adr,
        timebox_days: NonZeroU16::new(3).unwrap(),
    },
    taxon: taxon!(Subsystem, Scan),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "ADR с выбранной проверкой, её точкой принуждения и границей: какие упоминания символов она не видит."
    ),
);
