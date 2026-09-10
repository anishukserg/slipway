use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(9,
    title: NonEmptyStr::new("Убрать внешние артефакты и каталог spec/"),
    slice: crate::slice::s0003,
    origin: WorkOrigin::Decision(slipway_meta::adr::a0009),
    taxon: taxon!(Subsystem, Methodology),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Калитка не находит в дереве имён из локального списка внешних проектов; каталога spec/ и файла ревью нет; ссылки из кода и документов указывают на реестр."
    ),
);
