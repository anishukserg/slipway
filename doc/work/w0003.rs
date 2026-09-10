use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use crate::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(3,
    title: NonEmptyStr::new("Схема направлений, срезов и единиц работы; скан реестра работы"),
    slice: crate::slice::s0002,
    origin: WorkOrigin::Specification(crate::rfc::r0002),
    taxon: taxon!(Subsystem, Work),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Этот план компилируется; атаки slipway_work::attacks зелёные; радиус работы выше потолка среза — ошибка вычисления константы."
    ),
);
