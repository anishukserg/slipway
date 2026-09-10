//! Доказательство готовности (решение 15): калитка прошла на дереве, хэш
//! которого считается без каталога журнала.
//!
//! Событие журнала само меняет дерево коммита, поэтому доказательство
//! привязано не к хэшу коммита и не к хэшу дерева git, а к git-хэшу списка
//! `git ls-tree -r -z` дерева без каталога журнала. Разделитель — нулевой байт:
//! список не зависит от настройки core.quotePath, и хэш одного дерева
//! одинаков у всех.

use crate::{git, layout};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Хэш дерева без каталога журнала; `tree` — любой указатель на дерево: sha
/// дерева, коммит, `HEAD`.
pub fn content_hash(root: &Path, tree: &str) -> Option<String> {
    let listing = git::read(root, &["ls-tree", "-r", "-z", "--full-tree", tree])?;
    let journal = format!("{}/", layout::JOURNAL_DIR);
    let mut kept = String::new();
    for entry in listing.split('\0').filter(|entry| !entry.is_empty()) {
        let path = entry.split_once('\t').map_or("", |(_, path)| path);
        if !path.starts_with(&journal) {
            kept.push_str(entry);
            kept.push('\0');
        }
    }
    hash_stdin(root, &kept)
}

/// Хэш дерева коммита, который сейчас собирается: индекса, на который
/// указывает GIT_INDEX_FILE хука.
pub fn index_hash(root: &Path) -> Option<String> {
    let tree = git::read(root, &["write-tree"])?;
    content_hash(root, &tree)
}

/// git-хэши файлов по содержимому, без фильтров git, в порядке путей; `None` —
/// хэш файла не посчитан.
pub fn file_hashes(root: &Path, files: &[PathBuf]) -> Vec<Option<String>> {
    if files.is_empty() {
        return Vec::new();
    }
    let mut input = String::new();
    for file in files {
        input.push_str(&file.to_string_lossy());
        input.push('\n');
    }
    let output = run_with_input(
        root,
        &["hash-object", "--no-filters", "--stdin-paths"],
        &input,
    );
    let mut hashes: Vec<Option<String>> = output
        .unwrap_or_default()
        .lines()
        .map(|line| Some(line.trim().to_owned()))
        .collect();
    hashes.resize(files.len(), None);
    hashes
}

/// Путь доказательства для хэша.
pub fn path(git_dir: &Path, hash: &str) -> PathBuf {
    git_dir.join(layout::PROOFS_DIR).join(hash)
}

/// Сохраняет доказательство: время и вердикт калитки.
pub fn record(git_dir: &Path, hash: &str, verdict: &str) -> io::Result<()> {
    let file = path(git_dir, hash);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(
        file,
        format!("{}\n{verdict}\n", slipway_journal::time::now()),
    )
}

/// Вердикт сохранённого доказательства для хэша.
pub fn verdict(git_dir: &Path, hash: &str) -> Option<String> {
    let text = fs::read_to_string(path(git_dir, hash)).ok()?;
    text.lines().nth(1).map(str::to_owned)
}

fn hash_stdin(root: &Path, text: &str) -> Option<String> {
    run_with_input(root, &["hash-object", "--stdin"], text).map(|out| out.trim().to_owned())
}

/// Вывод git с данным стандартным вводом; `None` — git отказал.
fn run_with_input(root: &Path, args: &[&str], input: &str) -> Option<String> {
    let mut child = git::command(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(input.as_bytes()).ok()?;
    let output = child.wait_with_output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}
