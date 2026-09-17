use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(11,
    title: NonEmptyStr::new("Дефекты инструмента, найденные при письме документов и внедрении"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "Шаги калитки отвергают документ или коммит за настоящее нарушение, а не за форму записи; каждый дефект закрыт сценарием, падавшим до починки."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Local,
);
