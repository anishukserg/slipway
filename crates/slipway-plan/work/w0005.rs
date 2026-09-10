use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(5,
    title: NonEmptyStr::new("Журнал событий: файл на событие, свёртка, трейлер коммита"),
    slice: crate::slice::s0002,
    origin: WorkOrigin::Specification(slipway_meta::rfc::r0002),
    taxon: taxon!(Subsystem, Work),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Состояние каждой единицы плана вычисляется свёрткой каталога journal/; статусного файла нет; недопустимый переход отвергается с названием события."
    ),
);
