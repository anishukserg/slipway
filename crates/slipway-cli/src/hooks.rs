//! Хуки git по решениям 8, 9, 13 и 14.
//!
//! ```text
//! cargo slipway hook pre-commit
//! cargo slipway hook commit-msg <файл сообщения>
//! cargo slipway hook pre-push <удалённый> <адрес>
//! cargo slipway hooks install
//! ```
//!
//! Файлы хуков — однострочники в `.githooks/`, вызывающие эти команды; вся
//! логика хуков — здесь.

use crate::{gate, git, layout, message};
use std::ffi::OsString;
use std::fs;
use std::io::{self, BufRead};
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
            eprintln!("hook: pre-commit | commit-msg <файл> | pre-push <удалённый> <адрес>");
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
    let Some(repo) = git::Repo::discover(Path::new(".")) else {
        eprintln!("hooks install: не git-репозиторий");
        return 2;
    };
    let dir = repo.root.join(layout::HOOKS_DIR);
    for (name, call) in HOOKS {
        let path = dir.join(name);
        if path.exists() {
            println!("хук {name}: уже есть, не тронут");
            continue;
        }
        let text = format!(
            "#!/bin/sh\n# Правила Slipway (решение 14): хук вызывает установленный cargo slipway.\nexport RUSTUP_AUTO_INSTALL=0\nexec cargo slipway {call}\n"
        );
        let written = fs::create_dir_all(&dir)
            .and_then(|()| fs::write(&path, text))
            .and_then(|()| fs::set_permissions(&path, fs::Permissions::from_mode(0o755)));
        if let Err(error) = written {
            eprintln!("хук {name} не записан: {error}");
            return 2;
        }
        println!("хук {name}: записан");
    }
    if !git::succeeds(&repo.root, &["config", "core.hooksPath", layout::HOOKS_DIR]) {
        eprintln!("core.hooksPath не выставлен");
        return 2;
    }
    println!("core.hooksPath = {}", layout::HOOKS_DIR);
    0
}

/// pre-commit: калитка на дереве коммита, а не на рабочем дереве. При частичном
/// коммите это разные деревья, и проверка рабочего подтвердила бы не то, что
/// уходит в историю.
fn pre_commit() -> u8 {
    let Some(repo) = git::Repo::discover(Path::new(".")) else {
        eprintln!("pre-commit: не git-репозиторий");
        return 2;
    };
    let work = repo.git_dir.join(layout::COMMIT_TREE_DIR);
    let tree = work.join("tree");
    if let Err(problem) = export_index(&repo.root, &work.join("stage"), &tree) {
        eprintln!("pre-commit: {problem}");
        return 2;
    }
    let code = run_gate(&repo, &tree);
    if code != 0 {
        return code;
    }
    // Постусловие: тесты калитки не повредили индекс коммита. Породивший отказ:
    // переменные хука направили запись временного репозитория в этот индекс, и
    // коммит упал на построении дерева уже после зелёных проверок.
    if !git::succeeds(&repo.root, &["write-tree"]) {
        eprintln!("pre-commit: индекс коммита повреждён после калитки — коммит не создаётся");
        return 1;
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
        .map_err(|error| format!("каталоги выгрузки не созданы: {error}"))?;
    let prefix = format!("--prefix={}/", stage.display());
    if !git::succeeds(root, &["checkout-index", "--all", &prefix]) {
        return Err("git checkout-index не выгрузил индекс коммита".to_owned());
    }
    sync(stage, tree).map_err(|error| format!("дерево коммита не выгружено: {error}"))?;
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
fn run_gate(repo: &git::Repo, tree: &Path) -> u8 {
    if !tree.join(layout::TOOL_MANIFEST).is_file() {
        let args = [
            OsString::from("--repo"),
            repo.root.clone().into_os_string(),
            tree.as_os_str().to_owned(),
        ];
        return gate::run(&args);
    }
    let mut command = gate::cargo_command(tree, &repo.root.join(layout::GATE_TOOL_TARGET));
    command
        .args(["run", "--quiet", "--locked", "--manifest-path"])
        .arg(tree.join(layout::MANIFEST))
        .args(["-p", layout::TOOL_PACKAGE, "--", "gate", "--repo"])
        .arg(&repo.root)
        .arg(tree);
    match command.status() {
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
        Err(error) => {
            eprintln!("pre-commit: cargo не запустился: {error}");
            2
        }
    }
}

/// pre-push: в удалённый репозиторий не уходят ветки архива (решение 9),
/// вершина, зависимости которой не проходят политику по свежей базе
/// (решение 13), и история с именами внешних проектов (решение 9).
///
/// git передаёт на стандартный ввод строки
/// `<локальная ссылка> <локальный sha> <удалённая ссылка> <удалённый sha>`.
fn pre_push(input: impl BufRead) -> u8 {
    let Some(repo) = git::Repo::discover(Path::new(".")) else {
        eprintln!("pre-push: не git-репозиторий");
        return 2;
    };
    let list = repo.git_dir.join("info").join(layout::EXTERNAL_NAMES);
    let names = gate::read_external_names(&list);
    if names.is_empty() {
        eprintln!(
            "pre-push: список внешних имён {} пуст или не задан — история на внешние имена не проверялась",
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
            eprintln!("pre-push: ветка архива {local_ref} не публикуется (решение 9)");
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
        eprintln!("pre-push: в {short} нет Cargo.toml — зависимости не проверялись");
        return true;
    }
    if !in_commit(layout::DENY_POLICY) {
        eprintln!(
            "pre-push: в {short} нет deny.toml — политика зависимостей не задана (решение 13)"
        );
        return false;
    }
    if !gate::installed("cargo-deny") {
        eprintln!("pre-push: cargo-deny не установлен — cargo install cargo-deny --locked");
        return false;
    }
    let tree = repo.git_dir.join(layout::PUSH_TREE);
    if let Err(problem) = export_commit(repo, sha, &tree) {
        eprintln!("pre-push: не выгружено дерево {short}: {problem}");
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
        "pre-push: зависимости {short} не прошли cargo-deny, полный вывод: {}",
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
        Err("git read-tree или checkout-index не выполнились".to_owned())
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
        eprintln!("pre-push: история {local} не читается — внешние имена не проверены");
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
                eprintln!("pre-push: поиск внешних имён в {commit} не выполнился");
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
        eprintln!("pre-push: коммит {short} содержит внешнее имя");
        for file in files {
            eprintln!("  файл: {file}");
        }
        if in_message {
            eprintln!("  в сообщении коммита");
        }
    }
    clean
}
