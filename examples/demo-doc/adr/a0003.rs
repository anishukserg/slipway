//! Ломающее решение: инструкции миграции лежат внутри варианта.
use crate::taxonomy::Subsystem;
use slipway_core::{nonempty_str, taxon};
use slipway_knowledge::{Breaking, DocStatus};

slipway_knowledge::adr!(3,
    title: "Версия 4 формата страницы данных",
    status: DocStatus::Active,
    subsystems: &[taxon!(Subsystem, Storage)],
    context: r"
        Заголовок страницы исчерпал резерв флагов.
    ",
    decision: r"
        Расширить заголовок до 72 байт, подняв версию формата до 4.
    ",
    trade_offs: &["Минус: файлы версии 3 требуют перекодирования"],
    constraints: &["Страницы версии 3 обязаны читаться до конца срока поддержки."],
    authors: nonempty_str!["Анищук Сергей"],
    decided_at: slipway_core::date!(2026, 9, 10),
    // Отметить ломающим и не приложить инструкции — невыразимо.
    breaking: Breaking::Yes {
        migration: nonempty_str![
            "Остановить запись, дождаться контрольной точки",
            "Прогнать перекодировщик страниц, проверить контрольные суммы",
        ],
    },
    code_refs: &[crate::anchor::page_header],
    related_rfcs: &[],
);
