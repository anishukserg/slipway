use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(18,
    title: NonEmptyStr::new("cargo slipway gate"),
    slice: crate::slice::s0006,
    origin: WorkOrigin::Decision(crate::adr::a0014),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "cargo slipway gate выполняет шаги прежней калитки, кроме проверки синтаксиса скриптов, с тем же видом вердикта и кодами возврата; cargo запускается без переменных GIT_* и без унаследованной RUSTUP_TOOLCHAIN; на рабочем дереве Slipway вердикт совпадает со скриптом калитки; сценарии внешних имён, markdown, отсутствующего манифеста и явного дерева — интеграционные тесты."
    ),
);
