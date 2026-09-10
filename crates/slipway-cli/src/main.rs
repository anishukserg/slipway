//! `cargo slipway` — правила коммитов, калитка и хуки Slipway (решения 8 и 14).
//!
//! ```text
//! cargo slipway commit -F <сообщение> [--log <файл>] [--timeout <сек>] -- <пути…>
//! cargo slipway msg-check [--form-only] <сообщение>
//! ```
//!
//! Инструмент опирается на соглашения, а не на настройку: пути собраны в
//! модуле `layout`. Зависимостей нет: хук собирает инструмент, и сборка не
//! тянет граф крейтов.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

mod commit;
mod git;
mod layout;
mod message;

use std::ffi::OsString;
use std::process::ExitCode;

const USAGE: &str = "cargo slipway — правила коммитов Slipway (решения 8 и 14)

  cargo slipway commit -F <сообщение> [--log <файл>] [--timeout <сек>] -- <пути…>
  cargo slipway msg-check [--form-only] <сообщение>";

fn main() -> ExitCode {
    let mut args: Vec<OsString> = std::env::args_os().skip(1).collect();
    // `cargo slipway …` запускает `cargo-slipway slipway …`.
    if args.first().and_then(|a| a.to_str()) == Some("slipway") {
        args.remove(0);
    }
    let Some(command) = args.first().map(|a| a.to_string_lossy().into_owned()) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let rest = &args[1..];
    let code = match command.as_str() {
        "commit" => commit::run(rest),
        "msg-check" => message::run(rest),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            0
        }
        other => {
            eprintln!("неизвестная команда {other}\n\n{USAGE}");
            2
        }
    };
    ExitCode::from(code)
}
