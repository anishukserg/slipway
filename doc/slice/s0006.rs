use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(6,
    title: NonEmptyStr::new("Правила коммитов, калитка и хуки — инструмент cargo slipway"),
    thrust: crate::thrust::t0004,
    outcome: NonEmptyStr::new(
        "Коммиты Slipway проходят через cargo slipway commit; хуки — однострочники, вызывающие инструмент, а калитку и pre-push выполняет он; каталога tools/ со скриптами bash нет; сценарии прежнего самотеста — интеграционные тесты крейта slipway-cli."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
