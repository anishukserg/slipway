use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(7,
    title: NonEmptyStr::new("Журнал работы: события, доказательство готовности, команды"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "Состояние каждой работы и среза Slipway вычисляется свёрткой doc/journal при сборке; приземление без доказательства на том же дереве не собирается; события пишут команды cargo slipway work и slice; прошлые работы восстановлены из истории."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
