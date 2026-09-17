use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(33,
    title: NonEmptyStr::new("slipway.toml: пути реестра, правила темы, шаблоны служебных коммитов"),
    slice: crate::slice::s0012,
    origin: WorkOrigin::Decision(crate::adr::a0020),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Инструмент читает slipway.toml тем же плоским разбором, что и события журнала: пути реестра, набор типов и предел темы, шаблоны служебных тем, ссылку на правила проекта; без файла поведение прежнее; сценарий на чужой раскладке проходит."
    ),
);
