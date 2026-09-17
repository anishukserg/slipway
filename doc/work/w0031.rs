use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(31,
    title: NonEmptyStr::new("Шаг ссылок в markdown принимал код за ссылку"),
    slice: crate::slice::s0011,
    origin: WorkOrigin::Divergence {
        specification: crate::rfc::r0002,
        violated: NonEmptyStr::new(
            "Проверка содержательна: шаг калитки отвергает документ за настоящую битую ссылку, а не за форму записи (решения 8 и 14)."
        ),
    },
    taxon: taxon!(Subsystem, Cli),
    radius: BlastRadius::Local,
    outcome: NonEmptyStr::new(
        "Сценарий, падавший до починки, проходит: `](цель)` внутри кода в обратных кавычках и в огороженном блоке кода ссылкой не считается, а битая ссылка вне кода по-прежнему отвергается."
    ),
);
