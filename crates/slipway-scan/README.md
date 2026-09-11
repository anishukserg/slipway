# slipway-scan

Скан реестра [Slipway](https://github.com/anishukserg/slipway) для `build.rs` крейта документов: читает файлы документов и пишет в `OUT_DIR` код, который крейт включает через `include!`.

- **Решения и спецификации** — `scan_decisions`, `scan_specs`, `emit_refs`, `emit_spec_refs`: модули констант-ссылок, списки документов и константные сверки скана с компилятором.
- **Разметка кода** — `scan_anchors` и `anchors::emit_anchor_refs`: константы для каждого `#[doc_anchor]` в исходниках продукта; удалили разметку — ссылки из решений не собираются.
- **План** — `plan::scan_plan`, `plan::emit_plan`, `plan::emit_work_checks`: направления, срезы, единицы работы и проверка радиуса.
- **Журнал** — `journal::scan_journal`: свёртка событий; нарушение автомата — ошибка компиляции с именем файла события.

Имя файла документа обязано совпадать с его идентификатором, а отсутствующий каталог — ошибка сборки, а не пустой реестр. Порождённый код документирован и проходит строгий профиль lints продукта (решение 16).

Пример `build.rs` — [examples/demo-doc/build.rs](https://github.com/anishukserg/slipway/blob/master/examples/demo-doc/build.rs).

Лицензия — [Apache-2.0](https://github.com/anishukserg/slipway/blob/master/LICENSE).
