use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(30,
    title: NonEmptyStr::new("SECURITY.md и CONTRIBUTING.md"),
    slice: crate::slice::s0010,
    origin: WorkOrigin::Decision(crate::adr::a0018),
    taxon: taxon!(Subsystem, Methodology),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "SECURITY.md описывает приватное сообщение об уязвимости и что в него включить; CONTRIBUTING.md — issues, перенос внешних pull request сопровождающим, правила коммитов, калитку и хуки; ссылки в обоих разрешаются."
    ),
);
