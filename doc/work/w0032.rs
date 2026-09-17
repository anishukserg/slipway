use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(32,
    title: NonEmptyStr::new("Текст инструмента на английском"),
    slice: crate::slice::s0012,
    origin: WorkOrigin::Decision(crate::adr::a0019),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Вывод инструмента, темы и тела создаваемых им коммитов и комментарии порождённых хуков — английские; отказ не называет номер решения чужого реестра; сценарии и документы Slipway обновлены под новые строки."
    ),
);
