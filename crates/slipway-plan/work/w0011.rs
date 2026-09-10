use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(11,
    title: NonEmptyStr::new("Калитка: форматирование, clippy, документация, скрипты, ссылки, защита публикации"),
    slice: crate::slice::s0003,
    origin: WorkOrigin::Decision(slipway_meta::adr::a0008),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Калитка на дереве коммита проходит восемь шагов с тулчейном из rust-toolchain.toml; самотест подтверждает, что pre-push отвергает историю со следом внешнего имени и ветки архива и пропускает чистую историю."
    ),
);
