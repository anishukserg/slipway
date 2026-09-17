//! Сценарии блокировки коммита (решения 8 и 14): брошенная блокировка без
//! номера процесса снимается, свежая уважается, отказ по тайм-ауту называет
//! держателя.

mod common;

use common::{Run, TempRepo};
use std::fs::{self, File};
use std::time::{Duration, SystemTime};

const OK: &str = "[FEAT](cli): новый файл\n\nSlipway-Work: w0001\n";

/// Репозиторий с таксономией, работой w0001, базовым коммитом и новым файлом.
fn planned_repo(name: &str) -> TempRepo {
    let repo = TempRepo::new(name);
    repo.write(
        "doc/taxonomy.rs",
        "slipway_core::declare_taxonomy! {\n    Subsystem => [Cli],\n}\n",
    );
    repo.write("doc/work/w0001.rs", "work\n");
    repo.git(&["add", "-A"]);
    repo.git(&[
        "commit",
        "-q",
        "-m",
        "[CHORE](cli): база",
        "-m",
        "Slipway-Work: w0001",
    ]);
    repo.write("new.txt", "new\n");
    repo
}

fn commit(repo: &TempRepo, timeout: &str) -> Run {
    let message = repo.outside("message.txt", OK);
    let message = message.to_str().expect("путь в UTF-8");
    repo.tool(&[
        "commit",
        "-F",
        message,
        "--timeout",
        timeout,
        "--",
        "new.txt",
    ])
}

#[test]
fn leftover_lock_without_pid_is_taken_over() {
    // Пустой файл блокировки оставлял flock прежнего скрипта коммита; такой же
    // остаётся, если процесс умер между созданием файла и записью номера.
    let repo = planned_repo("leftover-empty-lock");
    let lock = repo.path(".git/slipway-commit.lock");
    File::create(&lock)
        .and_then(|file| file.set_modified(SystemTime::now() - Duration::from_secs(60)))
        .expect("брошенная блокировка создана");
    let run = commit(&repo, "5");
    assert_eq!(run.code, 0, "{}", run.output());
    assert!(!lock.exists(), "брошенная блокировка осталась");
}

#[test]
fn fresh_lock_without_pid_is_respected() {
    // Контроль: живой процесс пишет номер сразу после создания файла, поэтому
    // свежий файл без номера — чужая блокировка в момент захвата.
    let repo = planned_repo("fresh-empty-lock");
    let lock = repo.path(".git/slipway-commit.lock");
    File::create(&lock).expect("свежая блокировка создана");
    let run = commit(&repo, "1");
    assert_eq!(run.code, 3, "{}", run.output());
    assert!(lock.exists(), "ожидающий снял свежую блокировку");
}

#[test]
fn timeout_names_the_holder() {
    let repo = planned_repo("lock-holder-named");
    let lock = repo.path(".git/slipway-commit.lock");
    fs::write(&lock, format!("{}\n", std::process::id())).expect("блокировка записана");
    let run = commit(&repo, "1");
    assert_eq!(run.code, 3, "{}", run.output());
    let holder = format!("held by process {}", std::process::id());
    assert!(run.verdict().contains(&holder), "{}", run.output());
}
