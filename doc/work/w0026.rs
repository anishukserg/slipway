use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::{InquiryOutcome, WorkOrigin};
use std::num::NonZeroU16;

slipway_work::work!(26,
    title: NonEmptyStr::new("Доставка крейтов Slipway в сторонний проект"),
    slice: crate::slice::s0008,
    origin: WorkOrigin::Inquiry {
        question: NonEmptyStr::new(
            "Как крейты Slipway попадают в сторонний проект, чья политика зависимостей допускает только crates.io, так, чтобы CI проекта собирал реестр с закреплённой версией?"
        ),
        produces: InquiryOutcome::Adr,
        timebox_days: NonZeroU16::new(2).unwrap(),
    },
    taxon: taxon!(Subsystem, Methodology),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "ADR: способ доставки — публикация, git-зависимость с исключением в политике проекта или копия в дереве проекта — с ценой выпуска новой версии, требованиями к политике и CI проекта и перечнем необратимого."
    ),
);
