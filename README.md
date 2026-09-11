# Slipway

**Docs as Compiled Code** — методология и крейты Rust для проектов, где определяющее ограничение — не скорость написания кода, а удержание архитектурной целостности системы, которую одновременно меняют десятки людей и ИИ-агентов.

*A methodology and Rust crates in which architecture decisions, specifications and the work plan are Rust code: a reference to a document is a path to a constant, and a dangling reference does not compile.*

Решения, спецификации и план работ записываются кодом на Rust. Ссылка на документ — путь к константе, которую порождает скан реестра при сборке. Удалили решение или размеченный фрагмент кода — ссылки на него перестают собираться. Правила, которые обычно держатся на ревью, выражены типами, а обещания, объявленные компиляторными, закрыты атакующими тестами `compile_fail`.

## С чего читать

- [GUARANTEES.md](GUARANTEES.md) — что гарантируется и чем; для людей, читать первым.
- [SLIPWAY.md](SLIPWAY.md) — карта методологии: слои, инварианты, уровни внедрения.
- [doc](doc) — реестр самого Slipway: решения, спецификации, план и журнал работы, записанные по его же правилам.

## Крейты

| Крейт | Назначение |
|---|---|
| [slipway-core](crates/slipway-core) | общие типы и макросы: ссылки на документы, непустые строки и списки, обратимость, таксономия |
| [slipway-knowledge](crates/slipway-knowledge) | слой знания: схема решений и спецификаций, макросы `adr!` и `rfc!` |
| [slipway-work](crates/slipway-work) | слой работы: направления, срезы, единицы работы и их происхождение |
| [slipway-scan](crates/slipway-scan) | скан реестра из `build.rs`: константы-ссылки, сверки с компилятором, свёртка журнала |
| [slipway-derive](crates/slipway-derive) | атрибут `#[doc_anchor]` — разметка кода, на которую ссылаются решения |
| [slipway-journal](crates/slipway-journal) | журнал работы: формат событий, автомат переходов, свёртка |
| [slipway-cli](crates/slipway-cli) | `cargo slipway`: правила коммитов, калитка, хуки git и команды журнала |

Реестр продукта в миниатюре — [examples/demo-doc](examples/demo-doc) с разметкой кода в [examples/demo-product](examples/demo-product).

## Статус

Рабочий черновик. Работают слой знания, скан, атрибут разметки, план в реестре, правила коммитов и журнал работы; слой доступа описан спецификацией, но не реализован. Крейты не опубликованы в crates.io. Вне самого Slipway методология не внедрялась — что это значит для её обещаний, сказано в [GUARANTEES.md](GUARANTEES.md).

## Сборка и проверка

Разработка идёт на stable с точной версией из [rust-toolchain.toml](rust-toolchain.toml); потребителям крейтов достаточно Rust 1.83. Полной калитке нужны установленный тулчейн 1.83.0 и [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) с загруженной базой уязвимостей (`cargo deny fetch`).

```bash
cargo test --workspace                     # тесты и doctest
cargo run -p slipway-cli -- gate           # калитка коммита
cargo run -p slipway-cli -- hooks install  # хуки git: калитка на каждом коммите
```

Коммит в Slipway проходит калитку и несёт трейлер основания из плана — правила в решении 8, [doc/adr/a0008.rs](doc/adr/a0008.rs).

## Участие

Как предложить изменение — [CONTRIBUTING.md](CONTRIBUTING.md); об уязвимостях сообщают приватно — [SECURITY.md](SECURITY.md).

## Лицензия

[Apache License 2.0](LICENSE).
