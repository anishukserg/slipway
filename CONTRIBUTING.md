# Участие

Slipway ведёт собственную работу по своей методологии: у каждого коммита есть основание в плане, и каждый коммит проходит калитку. Отсюда порядок участия.

## Вопросы и проблемы

Открывайте [issue](https://github.com/anishukserg/slipway/issues): ошибка, неясное место в [SLIPWAY.md](SLIPWAY.md) или [GUARANTEES.md](GUARANTEES.md), механизм, который у вас не работает, — последнее ценнее всего. Об уязвимостях — не в issues, а приватно, как описано в [SECURITY.md](SECURITY.md).

## Pull request

Pull request — предложение, и кнопкой слияния он не сливается (решение 18):

- у коммита в `master` должен быть трейлер основания — работа из плана в [doc/work](doc/work), а план ведёт сопровождающий;
- squash и rebase на GitHub дают коммитам новые хэши, а журнал работы ссылается на коммиты по хэшу.

Сопровождающий переносит изменение коммитом по правилам, указывает вас в трейлере `Co-authored-by` и закрывает pull request ссылкой на этот коммит. CI проверяет pull request той же калиткой — её замечания лучше исправить до переноса.

## Сборка и проверка

Разработка идёт на тулчейне из [rust-toolchain.toml](rust-toolchain.toml). Калитке нужны ещё тулчейн минимальной версии из `rust-version` в [Cargo.toml](Cargo.toml) и [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) с загруженной базой уязвимостей.

```bash
cargo test --workspace
cargo run -p slipway-cli -- gate
```

## Коммиты

Правила — решение 8, [doc/adr/a0008.rs](doc/adr/a0008.rs):

- тема — тип в квадратных скобках, сразу за ним область в круглых, затем двоеточие и суть; тип из набора `FEAT FIX REFACTOR TEST DOCS ADR PLAN CHORE`, область — подсистема из [doc/taxonomy.rs](doc/taxonomy.rs) в нижнем регистре; не длиннее 72 символов и без точки в конце. Примеры — в `git log` этого репозитория;
- трейлер `Slipway-Work: wNNNN` — работа, которая есть в дереве коммита; коммит закрытия среза несёт `Slipway-Slice: sNNNN`;
- коммит — через `cargo slipway commit -F <message> -- <paths>`: хук pre-commit выполняет калитку на дереве коммита, commit-msg проверяет сообщение;
- хуки ставит `cargo run -p slipway-cli -- hooks install`.

Состояние работы — не поле записи, а журнал: `cargo slipway work start` и `land`, `cargo slipway slice close` (решение 15).

## Лицензия

Присылая изменение, вы соглашаетесь распространять его на условиях [Apache License 2.0](LICENSE), как и весь проект.
