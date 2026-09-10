use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(14,
    title: NonEmptyStr::new("Stable-тулчейн и сверка кодов ошибок атак"),
    slice: crate::slice::s0005,
    origin: WorkOrigin::Decision(crate::adr::a0012),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "rust-toolchain.toml закрепляет stable 1.98.1; калитка выполняет атаки под RUSTC_BOOTSTRAP в отдельном каталоге сборки после пробной атаки, которая падает с неверным кодом и проходит с верным; #![feature] запрещён в крейтах и в их doctest."
    ),
);
