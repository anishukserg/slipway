//! Замещённое решение: ссылка на замещающее — путь к константе, а не число.
use crate::taxonomy::Subsystem;
use slipway_core::{nonempty_str, taxon};
use slipway_knowledge::{Breaking, DocStatus};

slipway_knowledge::adr!(1,
    title: "Текстовый SQL как единственный путь исполнения",
    status: DocStatus::SupersededBy(crate::adr::a0002),
    subsystems: &[taxon!(Subsystem, Executor)],
    context: r#"
        Исходное устройство: любой запрос проходит через разбор текста SQL.
    "#,
    decision: r#"
        Единственный путь исполнения — текстовый SQL.
    "#,
    trade_offs: &["Плюс: простота", "Минус: разбор текста на горячем пути"],
    constraints: &[],
    authors: nonempty_str!["Анищук Сергей"],
    decided_at: slipway_core::date!(2026, 3, 1),
    breaking: Breaking::No,
    code_refs: &[],
    related_rfcs: &[],
);
