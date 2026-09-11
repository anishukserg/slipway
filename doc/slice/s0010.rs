use slipway_core::{BlastRadius, NonEmptyStr};

slipway_work::slice!(10,
    title: NonEmptyStr::new("CI и порядок участия в публичном репозитории"),
    thrust: crate::thrust::t0002,
    outcome: NonEmptyStr::new(
        "Калитка и проверка оснований коммитов выполняются в GitHub Actions на push и pull request в master; SECURITY.md и CONTRIBUTING.md описывают приватные сообщения об уязвимостях и перенос внешних изменений сопровождающим."
    ),
    specification: crate::rfc::r0002,
    max_radius: BlastRadius::Local,
);
