use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(2,
    title: NonEmptyStr::new("Минимальный слой работы и план Slipway в нём"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "План Slipway записан направлениями, срезами и единицами работы; ссылка на несуществующее решение, спецификацию или срез не компилируется; радиус работы выше потолка среза не компилируется."
    ),
    specification: slipway_meta::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
