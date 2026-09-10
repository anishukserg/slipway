use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(7,
    title: NonEmptyStr::new("Перенести открытые пункты ревью в план и удалить статусный файл"),
    slice: crate::slice::s0002,
    origin: WorkOrigin::Toil {
        justification: NonEmptyStr::new(
            "Раздел «Состояние» в REVIEW-2026-09-10.md — изменяемый статусный файл, первый контрпример гипотезе «журнал как единственное состояние»."
        ),
    },
    taxon: taxon!(Subsystem, Methodology),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Каждый открытый пункт ревью — единица работы, решение или запись с названной границей; REVIEW-2026-09-10.md удалён, история остаётся в git."
    ),
);
