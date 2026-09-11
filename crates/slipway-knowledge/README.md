# slipway-knowledge

Слой знания [Slipway](https://github.com/anishukserg/slipway): схема архитектурных решений и доменных спецификаций, макросы регистрации `adr!` и `rfc!`.

Документ — файл реестра на Rust, по файлу на документ. Правила, которые обычно проверяет валидатор, выражены типами:

- ломающее решение без инструкций миграции невыразимо — инструкции живут внутри варианта `Breaking::Yes`;
- замещение без указания замещающего решения невыразимо — `DocStatus::SupersededBy` несёт ссылку;
- решение без авторов и спецификация без инвариантов не компилируются.

Модуль `attacks` — атакующие doctest `compile_fail` с контролями: каждое обещание «не компилируется» проверено попыткой его нарушить.

Константы-ссылки на документы порождает [slipway-scan](https://github.com/anishukserg/slipway/tree/master/crates/slipway-scan) из `build.rs` крейта документов. Пример реестра — [examples/demo-doc](https://github.com/anishukserg/slipway/tree/master/examples/demo-doc).

Лицензия — [Apache-2.0](https://github.com/anishukserg/slipway/blob/master/LICENSE).
