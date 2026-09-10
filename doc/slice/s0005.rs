use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(5,
    title: NonEmptyStr::new("Сборка на stable, атаки на минимальной версии, проверка зависимостей"),
    thrust: crate::thrust::t0001,
    outcome: NonEmptyStr::new(
        "Калитка собирает Slipway закреплённым stable, сверяет коды ошибок атак на нём и на минимальной версии и отказывает, если сверка не работает; лицензии, источники и уязвимости зависимостей проверяются cargo-deny."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
