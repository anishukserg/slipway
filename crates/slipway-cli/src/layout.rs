//! Соглашения о раскладке проекта, на которые опирается инструмент
//! (решения 14 и 20).
//!
//! Здесь остаётся то, что настройке не подлежит: каталог хуков, каталоги
//! сборки, имена служебных файлов в каталоге git, манифест, политика
//! зависимостей и список внешних имён. Пути реестра — корень документов и всё,
//! что из него выводится, — задаёт настройка `slipway.toml` (модуль `config`),
//! а прежние значения стали в ней умолчаниями.

/// Настройка продукта в корне репозитория (решение 20).
pub const CONFIG: &str = "slipway.toml";

/// Блокировка коммита в каталоге git.
pub const COMMIT_LOCK: &str = "slipway-commit.lock";

/// Сообщение коммита события журнала, в каталоге git.
pub const JOURNAL_MESSAGE: &str = "slipway-journal-message";

/// Сообщение проверяемого коммита для делегированной команды, в каталоге git:
/// в режиме `--range` сообщение берётся из git, а команде нужен файл
/// (решение 20). Убирается сразу после запуска команды.
pub const CHECKED_MESSAGE: &str = "slipway-checked-message";

/// Корневой манифест: рабочее пространство и `rust-version`.
pub const MANIFEST: &str = "Cargo.toml";

/// Политика зависимостей (решение 13).
pub const DENY_POLICY: &str = "deny.toml";

/// Локальный список имён внешних проектов в `<каталог git>/info/` (решение 9).
/// В репозитории его нет: иначе имена оказались бы в нём.
pub const EXTERNAL_NAMES: &str = "slipway-external-names";

/// Каталог сборки калитки от корня репозитория; остальные каталоги сборки —
/// рядом с ним, `target/gate-<шаг>`.
pub const GATE_TARGET: &str = "target/gate";

/// Каталог хуков в репозитории; git направляется в него через core.hooksPath.
pub const HOOKS_DIR: &str = ".githooks";

/// Каталог в каталоге git, куда pre-commit выгружает дерево коммита.
pub const COMMIT_TREE_DIR: &str = "slipway-gate";

/// Доказательства калитки в каталоге git: файл на хэш дерева без журнала
/// (решение 15).
pub const PROOFS_DIR: &str = "slipway-proofs";

/// Манифест самого инструмента в дереве Slipway. Если он есть в дереве коммита,
/// калитку собирает и запускает инструмент из этого дерева.
pub const TOOL_MANIFEST: &str = "crates/slipway-cli/Cargo.toml";

/// Пакет инструмента.
pub const TOOL_PACKAGE: &str = "slipway-cli";

/// Каталог сборки инструмента из дерева коммита.
pub const GATE_TOOL_TARGET: &str = "target/gate-tool";

/// Дерево публикуемой вершины для pre-push, в каталоге git.
pub const PUSH_TREE: &str = "slipway-push-tree";

/// Временный индекс для выгрузки публикуемой вершины, в каталоге git.
pub const PUSH_INDEX: &str = "slipway-push-index";

/// Журнал cargo-deny в pre-push, в каталоге git.
pub const PUSH_DENY_LOG: &str = "slipway-push-deny.log";
