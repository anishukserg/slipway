use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(15,
    title: NonEmptyStr::new("Сборка и атаки на минимальной версии Rust"),
    slice: crate::slice::s0005,
    origin: WorkOrigin::Decision(crate::adr::a0012),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Калитка берёт минимальную версию из rust-version рабочего пространства, собирает на ней все цели, проверяет пробой сверку кодов и выполняет атаки с тем же полом по числу прошедших; отсутствующий тулчейн этой версии — отказ калитки."
    ),
);
