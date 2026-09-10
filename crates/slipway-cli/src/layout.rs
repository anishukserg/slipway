//! Соглашения о раскладке проекта, на которые опирается инструмент (решение 14).
//!
//! Все пути собраны здесь: инструмент не читает настроек, и проект с другой
//! раскладкой пока не поддерживается.

/// Таксономия реестра: значения оси подсистем — области темы коммита.
pub const TAXONOMY: &str = "doc/taxonomy.rs";

/// Каталог единиц работы: основание коммита обязано лежать здесь.
pub const WORK_DIR: &str = "doc/work";

/// Каталог срезов: основание коммита закрытия среза (решение 15).
pub const SLICE_DIR: &str = "doc/slice";

/// Каталог журнала работы (решение 15).
pub const JOURNAL_DIR: &str = "doc/journal";

/// Манифест крейта документов: его сборка сворачивает журнал.
pub const DOC_MANIFEST: &str = "doc/Cargo.toml";

/// Решение с правилами коммитов — ссылка в сообщениях об отказе.
pub const COMMIT_RULES: &str = "правила коммитов — решение 8, doc/adr/a0008.rs";

/// Блокировка коммита в каталоге git.
pub const COMMIT_LOCK: &str = "slipway-commit.lock";

/// Сообщение коммита события журнала, в каталоге git.
pub const JOURNAL_MESSAGE: &str = "slipway-journal-message";

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
