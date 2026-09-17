//! Хуки git по решениям 8, 9, 13, 14 и 15.
//!
//! ```text
//! cargo slipway hook pre-commit
//! cargo slipway hook commit-msg <message>
//! cargo slipway hook pre-push <remote> <url>
//! cargo slipway hooks install
//! ```
//!
//! Файлы хуков — однострочники в `.githooks/`, вызывающие эти команды; вся
//! логика хуков — здесь.

use crate::{gate, git, layout, message, proof};
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Stdio;

/// Нулевой sha: удаление ссылки или её отсутствие на удалённой стороне.
const ZERO: &str = "0000000000000000000000000000000000000000";

/// Хуки, которые пишет `hooks install`, и их вызов установленного инструмента.
const HOOKS: [(&str, &str); 3] = [
    ("pre-commit", "hook pre-commit"),
    ("commit-msg", "hook commit-msg \"$1\""),
    ("pre-push", "hook pre-push \"$@\""),
];

/// Строк cargo-deny, которые pre-push показывает при отказе.
const DENY_LINES: usize = 20;

/// `cargo slipway hook <имя> …`.
pub fn run(args: &[OsString]) -> u8 {
    match args.first().and_then(|name| name.to_str()) {
        Some("pre-commit") if args.len() == 1 => pre_commit(),
        Some("commit-msg") if args.len() == 2 => message::run(&args[1..]),
        Some("pre-push") => pre_push(io::stdin().lock()),
        _ => {
            eprintln!("hook: pre-commit | commit-msg <file> | pre-push <remote> <url>");
            2
        }
    }
}

/// `cargo slipway hooks install`: недостающие хуки — однострочники, вызывающие
/// установленный инструмент; существующие не трогаются. Git направляется в
/// каталог хуков репозитория.
pub fn install(args: &[OsString]) -> u8 {
    if args.len() != 1 || args[0].to_str() != Some("install") {
        eprintln!("hooks: install");
        return 2;
    }
    let repo = match git::Repo::discover(Path::new(".")) {
        Ok(repo) => repo,
        Err(problem) => {
            eprintln!("hooks install: {problem}");
            return 2;
        }
    };
    let dir = repo.root.join(layout::HOOKS_DIR);
    for (name, call) in HOOKS {
        let path = dir.join(name);
        if path.exists() {
            println!("hook {name}: already present, left alone");
            continue;
        }
        let text = format!(
            "#!/bin/sh\n# Slipway rules (decision 14): the hook calls the installed cargo slipway.\nexport RUSTUP_AUTO_INSTALL=0\nexec cargo slipway {call}\n"
        );
        let written = fs::create_dir_all(&dir)
            .and_then(|()| fs::write(&path, text))
            .and_then(|()| fs::set_permissions(&path, fs::Permissions::from_mode(0o755)));
        if let Err(error) = written {
            eprintln!("hook {name} not written: {error}");
            return 2;
        }
        println!("hook {name}: written");
    }
    if !git::succeeds(&repo.root, &["config", "core.hooksPath", layout::HOOKS_DIR]) {
        eprintln!("core.hooksPath is not set");
        return 2;
    }
    println!("core.hooksPath = {}", layout::HOOKS_DIR);
    0
}

/// pre-commit: калитка на дереве коммита, а не на рабочем дереве. При частичном
/// коммите это разные деревья, и проверка рабочего подтвердила бы не то, что
/// уходит в историю.
///
/// Доказательство (решение 15): после полной калитки для хэша дерева коммита
/// без журнала сохраняется доказательство. Если оно уже есть, дерево без
/// журнала не изменилось, и калитка проверяет только журнал.
fn pre_commit() -> u8 {
    let repo = match git::Repo::discover(Path::new(".")) {
        Ok(repo) => repo,
        Err(problem) => {
            eprintln!("pre-commit: {problem}");
            return 2;
        }
    };
    let work = repo.git_dir.join(layout::COMMIT_TREE_DIR);
    let tree = work.join("tree");
    if let Err(problem) = export_index(&repo.root, &work.join("stage"), &tree) {
        eprintln!("pre-commit: {problem}");
        return 2;
    }
    let hash = proof::index_hash(&repo.root, &repo.config.journal_dir());
    let proven = hash
        .as_deref()
        .and_then(|hash| proof::verdict(&repo.git_dir, hash));
    if let (Some(hash), Some(verdict)) = (&hash, &proven) {
        println!(
            "pre-commit: the tree without the journal is already checked ({}: {verdict}) — checking the journal",
            hash.get(..12).unwrap_or(hash)
        );
    }
    let (code, verdict) = run_gate(&repo, &tree, proven.is_some());
    if code != 0 {
        return code;
    }
    // Постусловие: тесты калитки не повредили индекс коммита. Породивший отказ:
    // переменные хука направили запись временного репозитория в этот индекс, и
    // коммит упал на построении дерева уже после зелёных проверок.
    if !git::succeeds(&repo.root, &["write-tree"]) {
        eprintln!("pre-commit: the commit index is damaged after the gate — no commit is created");
        return 1;
    }
    if proven.is_none() {
        match (hash, verdict) {
            (Some(hash), Some(verdict)) => {
                if let Err(error) = proof::record(&repo.git_dir, &hash, &verdict) {
                    eprintln!("pre-commit: proof not written: {error}");
                }
            }
            _ => eprintln!("pre-commit: no tree hash or no verdict — proof not written"),
        }
    }
    0
}

/// Выгружает индекс коммита в `tree` через промежуточный `stage`.
/// GIT_INDEX_FILE, выставленный git для хука, указывает на индекс коммита (при
/// `commit --only` — временный), поэтому выгружается ровно дерево коммита.
fn export_index(root: &Path, stage: &Path, tree: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(stage);
    fs::create_dir_all(stage)
        .and_then(|()| fs::create_dir_all(tree))
        .map_err(|error| format!("export directories not created: {error}"))?;
    let prefix = format!("--prefix={}/", stage.display());
    if !git::succeeds(root, &["checkout-index", "--all", &prefix]) {
        return Err("git checkout-index did not export the commit index".to_owned());
    }
    sync(stage, tree).map_err(|error| format!("commit tree not exported: {error}"))?;
    let _ = fs::remove_dir_all(stage);
    Ok(())
}

/// Приводит `dst` к содержимому `src`. Совпадающие файлы не перезаписываются:
/// время изменения сохраняется, и сборка дерева коммита остаётся
/// инкрементальной.
fn sync(src: &Path, dst: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dst)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let same_kind = fs::symlink_metadata(src.join(entry.file_name()))
            .map(|theirs| {
                let theirs = theirs.file_type();
                theirs.is_dir() == kind.is_dir() && theirs.is_symlink() == kind.is_symlink()
            })
            .unwrap_or(false);
        if !same_kind {
            if kind.is_dir() {
                fs::remove_dir_all(entry.path())?;
            } else {
                fs::remove_file(entry.path())?;
            }
        }
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir_all(&target)?;
            sync(&entry.path(), &target)?;
        } else if kind.is_symlink() {
            let link = fs::read_link(entry.path())?;
            if fs::read_link(&target).ok().as_deref() != Some(link.as_path()) {
                let _ = fs::remove_file(&target);
                std::os::unix::fs::symlink(&link, &target)?;
            }
        } else {
            copy_if_changed(&entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Копирует файл, если содержимое или права отличаются.
fn copy_if_changed(src: &Path, dst: &Path) -> io::Result<()> {
    let ours = fs::metadata(src)?;
    if let Ok(theirs) = fs::metadata(dst) {
        if theirs.len() == ours.len()
            && theirs.permissions() == ours.permissions()
            && fs::read(dst)? == fs::read(src)?
        {
            return Ok(());
        }
    }
    fs::copy(src, dst).map(|_| ())
}

/// Калитка из дерева коммита, если в нём есть сам инструмент: изменение правил
/// проверяется изменёнными правилами. Иначе — калитка этого инструмента.
/// Возвращает код и строку вердикта.
fn run_gate(repo: &git::Repo, tree: &Path, journal_only: bool) -> (u8, Option<String>) {
    let mut args = vec![OsString::from("--repo"), repo.root.clone().into_os_string()];
    if journal_only {
        args.push(OsString::from("--journal-only"));
    }
    args.push(tree.as_os_str().to_owned());
    if !tree.join(layout::TOOL_MANIFEST).is_file() {
        let (code, verdict) = gate::run_with_verdict(&args);
        return (code, Some(verdict));
    }
    let mut command = gate::cargo_command(tree, &repo.root.join(layout::GATE_TOOL_TARGET));
    command
        .args(["run", "--quiet", "--locked", "--manifest-path"])
        .arg(tree.join(layout::MANIFEST))
        .args(["-p", layout::TOOL_PACKAGE, "--", "gate"])
        .args(&args)
        .stdout(Stdio::piped());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            eprintln!("pre-commit: cargo did not start: {error}");
            return (2, None);
        }
    };
    // Вывод калитки передаётся дальше как есть; последняя строка GATE —
    // вердикт для доказательства.
    let mut verdict = None;
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            println!("{line}");
            if line.starts_with("GATE ") {
                verdict = Some(line);
            }
        }
    }
    let code = child
        .wait()
        .ok()
        .and_then(|status| status.code())
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);
    (code, verdict)
}

/// pre-push: в удалённый репозиторий не уходят ветки архива (решение 9),
/// вершина, зависимости которой не проходят политику по свежей базе
/// (решение 13), и история с именами внешних проектов (решение 9).
///
/// git передаёт на стандартный ввод строки
/// `<локальная ссылка> <локальный sha> <удалённая ссылка> <удалённый sha>`.
fn pre_push(input: impl BufRead) -> u8 {
    let repo = match git::Repo::discover(Path::new(".")) {
        Ok(repo) => repo,
        Err(problem) => {
            eprintln!("pre-push: {problem}");
            return 2;
        }
    };
    let list = repo.git_dir.join("info").join(layout::EXTERNAL_NAMES);
    let names = gate::read_external_names(&list);
    if names.is_empty() {
        eprintln!(
            "pre-push: external name list {} is empty or not set — history was not checked for external names",
            list.display()
        );
    }
    let mut refused = false;
    for line in input.lines() {
        let Ok(line) = line else {
            break;
        };
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [local_ref, local_sha, remote_ref, remote_sha] = fields.as_slice() else {
            continue;
        };
        if *local_sha == ZERO {
            continue;
        }
        if is_archive(local_ref) || is_archive(remote_ref) {
            eprintln!("pre-push: archive branch {local_ref} is not published (decision 9)");
            refused = true;
            continue;
        }
        if !dependencies_pass(&repo, local_sha) {
            refused = true;
        }
        if !names.is_empty() && !history_is_clean(&repo, local_sha, remote_sha, &names) {
            refused = true;
        }
    }
    if refused {
        eprintln!("PUSH REFUSED");
        return 1;
    }
    0
}

fn is_archive(reference: &str) -> bool {
    reference.starts_with("refs/heads/archive/")
}

/// Политика deny.toml со свежей базой уязвимостей на выгруженном дереве
/// вершины (решение 13): проверяется то, что публикуется, а не рабочее дерево.
fn dependencies_pass(repo: &git::Repo, sha: &str) -> bool {
    let short =
        git::read(&repo.root, &["rev-parse", "--short", sha]).unwrap_or_else(|| sha.to_owned());
    let in_commit =
        |path: &str| git::succeeds(&repo.root, &["cat-file", "-e", &format!("{sha}:{path}")]);
    if !in_commit(layout::MANIFEST) {
        eprintln!("pre-push: no Cargo.toml in {short} — dependencies were not checked");
        return true;
    }
    if !in_commit(layout::DENY_POLICY) {
        eprintln!("pre-push: no deny.toml in {short} — no dependency policy is set (decision 13)");
        return false;
    }
    if !gate::installed("cargo-deny") {
        eprintln!("pre-push: cargo-deny is not installed — cargo install cargo-deny --locked");
        return false;
    }
    let tree = repo.git_dir.join(layout::PUSH_TREE);
    if let Err(problem) = export_commit(repo, sha, &tree) {
        eprintln!("pre-push: tree {short} not exported: {problem}");
        return false;
    }
    let log = repo.git_dir.join(layout::PUSH_DENY_LOG);
    let mut deny = gate::cargo_command(&tree, &repo.root.join(layout::GATE_TARGET));
    deny.args([
        "deny",
        "--manifest-path",
        layout::MANIFEST,
        "--config",
        layout::DENY_POLICY,
        "--locked",
        "check",
    ]);
    let passed = fs::File::create(&log)
        .and_then(|out| Ok((out.try_clone()?, out)))
        .and_then(|(err, out)| deny.stdin(Stdio::null()).stdout(out).stderr(err).status())
        .is_ok_and(|status| status.success());
    if passed {
        return true;
    }
    let text = gate::read_lossy(&log);
    for line in text
        .lines()
        .filter(|line| line.starts_with("error") || line.starts_with("bug"))
        .take(DENY_LINES)
    {
        eprintln!("{line}");
    }
    if let Some(summary) = text.lines().last() {
        eprintln!("{summary}");
    }
    eprintln!(
        "pre-push: dependencies of {short} did not pass cargo-deny, full output: {}",
        log.display()
    );
    false
}

/// Выгружает дерево коммита `sha` через временный индекс, не трогая индекс
/// репозитория.
fn export_commit(repo: &git::Repo, sha: &str, tree: &Path) -> Result<(), String> {
    let index = repo.git_dir.join(layout::PUSH_INDEX);
    let _ = fs::remove_dir_all(tree);
    fs::create_dir_all(tree).map_err(|error| error.to_string())?;
    let prefix = format!("--prefix={}/", tree.display());
    let with_index = |args: &[&str]| {
        git::command(&repo.root)
            .env("GIT_INDEX_FILE", &index)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    let exported =
        with_index(&["read-tree", sha]) && with_index(&["checkout-index", "--all", &prefix]);
    let _ = fs::remove_file(&index);
    if exported {
        Ok(())
    } else {
        Err("git read-tree or checkout-index failed".to_owned())
    }
}

/// История от удалённой вершины до локальной без внешних имён в файлах и
/// сообщениях. Если удалённой вершины нет локально, проверяется вся история
/// локальной: неизвестный диапазон не должен означать «нечего проверять».
fn history_is_clean(repo: &git::Repo, local: &str, remote: &str, names: &[String]) -> bool {
    let range = format!("{remote}..{local}");
    let listed = if remote == ZERO {
        None
    } else {
        git::read(&repo.root, &["rev-list", &range])
    };
    let Some(commits) = listed.or_else(|| git::read(&repo.root, &["rev-list", local])) else {
        eprintln!("pre-push: history of {local} cannot be read — external names not checked");
        return false;
    };
    let lowered: Vec<String> = names.iter().map(|name| name.to_lowercase()).collect();
    let mut clean = true;
    for commit in commits.lines() {
        let mut grep = git::command(&repo.root);
        grep.args(["grep", "-I", "-i", "-l", "-F"]);
        for name in names {
            grep.arg("-e").arg(name);
        }
        grep.args([commit, "--", "."]).stderr(Stdio::null());
        let prefix = format!("{commit}:");
        let files: Vec<String> = match grep.output() {
            Ok(out) if out.status.code() == Some(0) => String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|line| line.strip_prefix(&prefix).unwrap_or(line).to_owned())
                .collect(),
            Ok(out) if out.status.code() == Some(1) => Vec::new(),
            _ => {
                eprintln!("pre-push: external name search in {commit} failed");
                clean = false;
                continue;
            }
        };
        let message = git::read(&repo.root, &["log", "-1", "--format=%B", commit])
            .unwrap_or_default()
            .to_lowercase();
        let in_message = lowered.iter().any(|name| message.contains(name.as_str()));
        if files.is_empty() && !in_message {
            continue;
        }
        clean = false;
        let short = git::read(&repo.root, &["rev-parse", "--short", commit])
            .unwrap_or_else(|| commit.to_owned());
        eprintln!("pre-push: commit {short} contains an external name");
        for file in files {
            eprintln!("  file: {file}");
        }
        if in_message {
            eprintln!("  in the commit message");
        }
    }
    clean
}
