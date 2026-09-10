use crate::taxonomy::Subsystem;
use slipway_core::{taxon, BlastRadius, NonEmptyStr};
use slipway_work::WorkOrigin;

slipway_work::work!(5,
    title: NonEmptyStr::new("Журнал событий: формат, автомат переходов, свёртка при сборке реестра"),
    slice: crate::slice::s0007,
    origin: WorkOrigin::Specification(crate::rfc::r0002),
    taxon: taxon!(Subsystem, Work),
    radius: BlastRadius::Crate,
    outcome: NonEmptyStr::new(
        "Состояние каждой единицы работы — свёртка каталога doc/journal при сборке крейта документов; статусного поля нет; недопустимый переход не собирается и называет файл события; событие о работе вне плана не разрешается; незавершённая работа в закрытом срезе не собирается."
    ),
);
