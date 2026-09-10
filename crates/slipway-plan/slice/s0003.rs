use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(3,
    title: NonEmptyStr::new("Правила коммитов и репозиторий без внешних артефактов"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "Коммит проходит только с формой сообщения и основанием из плана, калитка гоняется на дереве коммита, строка с именем внешнего проекта отвергается; каталога spec/ в дереве нет."
    ),
    specification: slipway_meta::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
