use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(4,
    title: NonEmptyStr::new("Документы Slipway в одном крейте doc/"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "Решения, спецификации и план лежат в doc/ одним крейтом slipway-doc, в crates/ только библиотеки; сборка, тесты, калитка и правила коммитов работают по новым путям."
    ),
    specification: crate::rfc::r0001,
    max_radius: BlastRadius::Crate,
);
