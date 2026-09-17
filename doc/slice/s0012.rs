use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(12,
    title: NonEmptyStr::new("Инструмент под чужой проект: язык и настройка"),
    thrust: crate::thrust::t0004,
    outcome: NonEmptyStr::new(
        "Инструмент говорит и пишет по-английски, а раскладку реестра, правила сообщения, шаблоны служебных тем, пол атак и собственную проверку проект задаёт настройкой; сценарий на чужой раскладке проходит целиком."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Crate,
);
