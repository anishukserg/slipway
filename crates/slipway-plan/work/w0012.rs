use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(12,
    title: NonEmptyStr::new("Шаг ссылок в markdown не выполнялся на дереве коммита"),
    slice: crate::slice::s0003,
    origin: WorkOrigin::Divergence {
        specification: slipway_meta::rfc::r0002,
        violated: NonEmptyStr::new(
            "Калитка на дереве коммита проверяет относительные ссылки в markdown (решение 8)."
        ),
    },
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Самотест, падавший до починки, проходит: битая ссылка в markdown отвергается и в дереве внутри каталога git; калитка на дереве коммита выполняет все восемь шагов."
    ),
);
