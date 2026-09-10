use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(21,
    title: NonEmptyStr::new("Путь, удалённый через git rm, не коммитился"),
    slice: crate::slice::s0006,
    origin: WorkOrigin::Divergence {
        specification: crate::rfc::r0002,
        violated: NonEmptyStr::new(
            "Пути перечисляются явно — новые, изменённые и удалённые; коммитятся ровно они (решение 8)."
        ),
    },
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Сценарий, падавший до починки, проходит: путь, удалённый через git rm, коммитится командой commit без предварительного git reset; путь, которого нет ни в рабочем дереве, ни в индексе, ни в HEAD, по-прежнему отвергается как опечатка."
    ),
);
