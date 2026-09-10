use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(16,
    title: NonEmptyStr::new("Проверка зависимостей cargo-deny"),
    slice: crate::slice::s0005,
    origin: WorkOrigin::Decision(crate::adr::a0013),
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "deny.toml записывает политику зависимостей; калитка коммита проверяет её по сохранённой базе без сети, pre-push — по свежей базе на выгруженной публикуемой вершине; крейты документов и демо не публикуются, зависимости между библиотеками несут версию."
    ),
);
