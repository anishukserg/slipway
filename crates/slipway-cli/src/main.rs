//! `cargo slipway` — правила коммитов, калитка, хуки и журнал Slipway
//! (решения 8, 14 и 15).
//!
//! ```text
//! cargo slipway commit -F <message> [--log <file>] [--timeout <seconds>] -- <paths…>
//! cargo slipway msg-check [--form-only] <message> | --range <range>
//! cargo slipway gate [--repo <directory>] [--journal-only] [<tree>]
//! cargo slipway hook pre-commit | commit-msg <message> | pre-push <remote> <url>
//! cargo slipway hooks install
//! cargo slipway work start | land | drop | state …
//! cargo slipway slice close <sNNNN>
//! cargo slipway journal hash [<revision>] | import --work <wNNNN> [--close-finished-slices]
//! ```
//!
//! Инструмент опирается на соглашения, а не на настройку: пути собраны в
//! модуле `layout`. Внешних зависимостей нет: хук собирает инструмент, и
//! сборка не тянет граф крейтов.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

mod commit;
mod gate;
mod git;
mod hooks;
mod journal;
mod layout;
mod message;
mod proof;
mod work;

use std::ffi::OsString;
use std::process::ExitCode;

const USAGE: &str = "cargo slipway — commit rules and the Slipway journal (decisions 8, 14 and 15)

  cargo slipway commit -F <message> [--log <file>] [--timeout <seconds>] -- <paths…>
  cargo slipway msg-check [--form-only] <message> | --range <range>
  cargo slipway gate [--repo <directory>] [--journal-only] [<tree>]
  cargo slipway hook pre-commit | commit-msg <message> | pre-push <remote> <url>
  cargo slipway hooks install
  cargo slipway work start <wNNNN> | land <wNNNN> [--commit <revision>] | drop <wNNNN> --reason <reason> | state [<wNNNN>]
  cargo slipway slice close <sNNNN>
  cargo slipway journal hash [<revision>]
  cargo slipway journal import --work <wNNNN> [--close-finished-slices]

  The work, slice and journal import commands accept --trailer <trailer> for
  extra trailer lines of the commit message.";

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
        "gate" => gate::run(rest),
        "hook" => hooks::run(rest),
        "hooks" => hooks::install(rest),
        "work" => work::run_work(rest),
        "slice" => work::run_slice(rest),
        "journal" => journal::run(rest),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            0
        }
        other => {
            eprintln!("unknown command {other}\n\n{USAGE}");
            2
        }
    };
    ExitCode::from(code)
}
