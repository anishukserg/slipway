use slipway_core::NonEmptyStr;

slipway_work::thrust!(1,
    title: NonEmptyStr::new("Гарантии Slipway честны и проверены атакой"),
    outcome: NonEmptyStr::new(
        "Каждое обещание GUARANTEES.md с силой «невозможно» или «компилятор» имеет атакующий compile_fail-тест с позитивным контролем, и набор атак зелёный на основной ветке."
    ),
);
