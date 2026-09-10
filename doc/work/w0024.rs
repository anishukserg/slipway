use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(24,
    title: NonEmptyStr::new("Журнал Slipway восстановлен из истории"),
    slice: crate::slice::s0007,
    origin: WorkOrigin::Decision(crate::adr::a0015),
    taxon: taxon!(Subsystem, Work),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Работы Slipway с коммитами по трейлеру записаны в журнал как приземлённые из истории; срезы без незавершённых работ закрыты; состояние плана выводится свёрткой, а не догадкой по git log."
    ),
);
