use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(19,
    title: NonEmptyStr::new("Хуки через cargo slipway, без скриптов bash"),
    slice: crate::slice::s0006,
    origin: WorkOrigin::Decision(crate::adr::a0014),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Хуки в .githooks — однострочники, вызывающие cargo slipway hook; pre-commit выгружает дерево коммита и запускает калитку инструментом из этого дерева; pre-push проверяет ветки архива, зависимости вершины и внешние имена в истории; hooks install ставит хуки в другом проекте; каталога tools/ нет; сценарии pre-commit и pre-push — интеграционные тесты."
    ),
);
