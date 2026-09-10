use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(23,
    title: NonEmptyStr::new("Команды cargo slipway work и slice"),
    slice: crate::slice::s0007,
    origin: WorkOrigin::Decision(crate::adr::a0015),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "work start, land, drop и state и slice close пишут события и коммитят их по правилам коммитов; land отказывает без доказательства для дерева коммита работы; коммит закрытия среза несёт трейлер Slipway-Slice."
    ),
);
