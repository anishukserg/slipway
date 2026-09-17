# slipway-cli

Команда `cargo slipway` [Slipway](https://github.com/anishukserg/slipway): правила коммитов, калитка, хуки git и журнал работы (решения 8, 14 и 15).

```text
cargo slipway commit -F <message> [--log <file>] [--timeout <seconds>] -- <paths…>
cargo slipway msg-check [--form-only] <message> | --range <range>
cargo slipway gate [--repo <directory>] [--journal-only] [<tree>]
cargo slipway hook pre-commit | commit-msg <message> | pre-push <remote> <url>
cargo slipway hooks install
cargo slipway work start | land | drop | state …
cargo slipway slice close <sNNNN>
cargo slipway journal hash [<revision>] | import --work <wNNNN> [--close-finished-slices]
```

- **Калитка** проверяет дерево коммита, а не рабочую копию: форматирование, clippy, тесты, атакующие doctest со сверкой кодов ошибок, документацию, сборку на минимальной версии Rust и политику зависимостей.
- **Журнал**: команды `work` и `slice` пишут события и коммитят их; коммит, меняющий только журнал, переиспользует доказательство калитки для того же дерева.
- **Без внешних зависимостей**: хук собирает инструмент, и сборка не тянет граф крейтов.

Раскладку реестра, набор типов темы, её предел, ссылку на правила коммитов проекта и темы служебных коммитов задаёт `slipway.toml` в корне репозитория (решение 20). Без файла действуют соглашения Slipway: `doc/` и `.githooks/`.

```bash
cargo install --locked --git https://github.com/anishukserg/slipway slipway-cli
```

Лицензия — [Apache-2.0](https://github.com/anishukserg/slipway/blob/master/LICENSE).
