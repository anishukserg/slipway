use slipway_core::NonEmptyStr;

slipway_work::thrust!(3,
    title: NonEmptyStr::new("Спецификация методологии — компилируемый реестр"),
    outcome: NonEmptyStr::new(
        "Нормативный текст методологии живёт в реестре slipway-meta; markdown спецификации порождается рендером, и ручная правка порождённого текста отвергается проверкой свежести."
    ),
);
