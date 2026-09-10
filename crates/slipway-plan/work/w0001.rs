use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_meta::taxonomy::Subsystem;
use slipway_work::WorkOrigin;

slipway_work::work!(1,
    title: NonEmptyStr::new(
        "Закрыть подделку ссылок, дубли идентификаторов, молчаливые ошибки разметки, смешение осей и пустые строки"
    ),
    slice: crate::slice::s0001,
    origin: WorkOrigin::Divergence {
        specification: slipway_meta::rfc::r0001,
        violated: NonEmptyStr::new("Ссылка на живое решение из позиции «замещённое» не компилируется."),
    },
    taxon: taxon!(Subsystem, Knowledge),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Атакующие тесты падали на 7c29847 и проходят после починки: 23 doctest-атаки с контролями, 20 тестов скана."
    ),
);
