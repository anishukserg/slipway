use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(1,
    title: NonEmptyStr::new("Закрыть атаки на слой знания"),
    thrust: crate::thrust::t0001,
    outcome: NonEmptyStr::new(
        "Атаки E1, E1b, E2, E4, E5, E6, E6b, E9 не собираются, их контроли собираются; для E3 заведено исследование."
    ),
    specification: crate::rfc::r0001,
    max_radius: BlastRadius::Crate,
);
