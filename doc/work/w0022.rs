use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(22,
    title: NonEmptyStr::new("Доказательство готовности по хэшу дерева без журнала"),
    slice: crate::slice::s0007,
    origin: WorkOrigin::Decision(crate::adr::a0015),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "pre-commit после прохождения калитки сохраняет доказательство для хэша дерева коммита без каталога журнала; калитка отвергает изменение или удаление файла журнала и приземление, чей коммит отсутствует или чьё дерево расходится с событием; коммит, меняющий только журнал, переиспользует доказательство."
    ),
);
