use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use crate::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(8,
    title: NonEmptyStr::new("Правила коммитов: проверка сообщения, калитка на дереве коммита, обёртка коммита"),
    slice: crate::slice::s0003,
    origin: WorkOrigin::Decision(crate::adr::a0008),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Самотест tools/test-commit-rules.sh проходит: неверная форма, отсутствующее или несуществующее основание, пустой набор путей и внешнее имя отвергаются, корректный коммит принимается ровно с перечисленными путями."
    ),
);
