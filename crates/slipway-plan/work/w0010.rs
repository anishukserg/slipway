use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(10,
    title: NonEmptyStr::new("Документы реестров без каталога src"),
    slice: crate::slice::s0003,
    origin: WorkOrigin::Decision(slipway_meta::adr::a0010),
    taxon: taxon!(Subsystem, Knowledge),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Решения, спецификации, план и демо-реестр открываются из корня своих крейтов; сборка, тесты и проверка сообщения коммита работают по новым путям."
    ),
);
