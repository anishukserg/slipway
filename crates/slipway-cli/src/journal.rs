//! Служебные команды журнала (решение 15).
//!
//! ```text
//! cargo slipway journal hash [<ревизия>]
//! ```
//!
//! `hash` печатает хэш дерева ревизии без каталога журнала — тот, к которому
//! привязано доказательство готовности.

use crate::{git, proof};
use std::ffi::OsString;
use std::path::Path;

/// `cargo slipway journal …`.
pub fn run(args: &[OsString]) -> u8 {
    match args.first().and_then(|name| name.to_str()) {
        Some("hash") if args.len() <= 2 => hash(args.get(1).and_then(|rev| rev.to_str())),
        _ => {
            eprintln!("journal: hash [<ревизия>]");
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
