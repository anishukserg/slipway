use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(34,
    title: NonEmptyStr::new("Делегирование проверок проекту и настраиваемый пол атак"),
    slice: crate::slice::s0012,
    origin: WorkOrigin::Decision(crate::adr::a0020),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Команда проекта выполняется шагом калитки, и её отказ — отказ калитки; при заданной команде разбор сообщения делегируется ей, а трейлер основания по-прежнему проверяет Slipway; пол атакующих doctest берётся из настройки."
    ),
);
