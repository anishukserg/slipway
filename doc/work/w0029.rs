use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(29,
    title: NonEmptyStr::new("Калитка и основания коммитов в GitHub Actions"),
    slice: crate::slice::s0010,
    origin: WorkOrigin::Decision(crate::adr::a0018),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Workflow gate выполняет калитку и msg-check --range на push и pull request в master; msg-check --range отвергает коммит диапазона без основания в его дереве и называет его хэш — сценарий в тестах; калитка проходит."
    ),
);
