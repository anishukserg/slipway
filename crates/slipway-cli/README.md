# slipway-cli

Команда `cargo slipway` [Slipway](https://github.com/anishukserg/slipway): правила коммитов, калитка, хуки git и журнал работы (решения 8, 14 и 15).

```text
cargo slipway commit -F <сообщение> [--log <файл>] [--timeout <сек>] -- <пути…>
cargo slipway msg-check [--form-only] <сообщение> | --range <диапазон>
cargo slipway gate [--repo <каталог>] [--journal-only] [<дерево>]
cargo slipway hook pre-commit | commit-msg <сообщение> | pre-push <удалённый> <адрес>
cargo slipway hooks install
cargo slipway work start | land | drop | state …
cargo slipway slice close <sNNNN>
cargo slipway journal hash [<ревизия>] | import --work <wNNNN> [--close-finished-slices]
```

- **Калитка** проверяет дерево коммита, а не рабочую копию: форматирование, clippy, тесты, атакующие doctest со сверкой кодов ошибок, документацию, сборку на минимальной версии Rust и политику зависимостей.
- **Журнал**: команды `work` и `slice` пишут события и коммитят их; коммит, меняющий только журнал, переиспользует доказательство калитки для того же дерева.
- **Без внешних зависимостей**: хук собирает инструмент, и сборка не тянет граф крейтов.

Инструмент опирается на соглашения раскладки самого Slipway (`doc/`, `.githooks/`) и пока не настраивается.

```bash
cargo install --locked --git https://github.com/anishukserg/slipway slipway-cli
```

Лицензия — [Apache-2.0](https://github.com/anishukserg/slipway/blob/master/LICENSE).
