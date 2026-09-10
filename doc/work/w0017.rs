use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(17,
    title: NonEmptyStr::new("cargo slipway commit и msg-check"),
    slice: crate::slice::s0006,
    origin: WorkOrigin::Decision(crate::adr::a0014),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Крейт slipway-cli без зависимостей выполняет commit и msg-check с теми же кодами возврата и вердиктами, что прежний скрипт коммита; сценарии формы сообщения, основания, путей, журнала и блокировки — интеграционные тесты на временных репозиториях."
    ),
);
