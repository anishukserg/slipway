use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(13,
    title: NonEmptyStr::new("Документы Slipway в одном крейте doc/"),
    slice: crate::slice::s0004,
    origin: WorkOrigin::Decision(crate::adr::a0011),
    taxon: taxon!(Subsystem, Knowledge),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Крейты slipway-meta и slipway-plan заменены крейтом slipway-doc в doc/, демо-реестр переименован в demo-doc; документы ссылаются друг на друга через crate::; калитка и самотест правил коммитов проходят по новым путям."
    ),
);
