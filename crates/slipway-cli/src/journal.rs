//! Служебные команды журнала (решение 15).
//!
//! ```text
//! cargo slipway journal hash [<ревизия>]
//! cargo slipway journal import --work <wNNNN> [--close-finished-slices] [--trailer <трейлер>]…
//! ```
//!
//! `hash` печатает хэш дерева ревизии без каталога журнала — тот, к которому
//! привязано доказательство готовности. `import` восстанавливает прошлое по
//! трейлерам истории: работа с коммитом по трейлеру приземляется из истории.

use crate::{git, proof, work};
use std::ffi::OsString;
use std::path::Path;

/// `cargo slipway journal …`.
pub fn run(args: &[OsString]) -> u8 {
    match args.first().and_then(|name| name.to_str()) {
        Some("hash") if args.len() <= 2 => hash(args.get(1).and_then(|rev| rev.to_str())),
        Some("import") => work::run_import(&args[1..]),
        _ => {
            eprintln!(
                "journal: hash [<ревизия>] | import --work <wNNNN> [--close-finished-slices]"
            );
            2
        }
    }
}

fn hash(revision: Option<&str>) -> u8 {
    let Some(repo) = git::Repo::discover(Path::new(".")) else {
        eprintln!("journal hash: не git-репозиторий");
        return 2;
    };
    let revision = revision.unwrap_or("HEAD");
    match proof::content_hash(&repo.root, revision) {
        Some(hash) => {
            println!("{hash}");
            0
        }
        None => {
            eprintln!("journal hash: ревизия {revision} не читается");
            2
        }
    }
}
