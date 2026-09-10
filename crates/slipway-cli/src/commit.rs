//! Коммит по решению 8.
//!
//! ```text
//! cargo slipway commit -F <файл сообщения> [--log <файл>] [--timeout <сек>] -- <пути…>
//! ```
//!
//! Сообщение проверяется по форме до блокировки и хуков. Пути перечисляются
//! явно — новые, изменённые и удалённые, от корня репозитория; коммитятся ровно
//! они. Параллельный коммит ждёт блокировку, а не отказывает. С `--log` вывод
//! git и хуков уходит в файл, на терминал — строки отказа и вердикт.
//!
//! Код возврата: 0 — коммит создан; 1 — по путям нечего коммитить или git add
//! упал; 2 — неверные аргументы или окружение; 3 — блокировка не получена;
//! 4 — коммит не создан: отказ проверки сообщения, хука или самого git.
//! Последняя строка вывода — `COMMIT OK <sha>` или `COMMIT REFUSED: <причина>`.

use crate::{git, layout, message};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Сколько ждать блокировку по умолчанию.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// Сколько строк отказа из журнала показывать на терминале.
const REFUSAL_LINES: usize = 40;

/// `cargo slipway commit`.
pub fn run(args: &[OsString]) -> u8 {
    let args = match Args::parse(args) {
        Ok(args) => args,
        Err(refusal) => return refusal.print(None),
    };
    match commit(&args) {
        Ok(sha) => {
            println!("COMMIT OK {sha}");
            0
        }
        Err(refusal) => refusal.print(args.log.as_deref()),
    }
}

struct Args {
    message: PathBuf,
    log: Option<PathBuf>,
    timeout: Duration,
    paths: Vec<OsString>,
}

impl Args {
    fn parse(args: &[OsString]) -> Result<Args, Refusal> {
        let mut message = None;
        let mut log = None;
        let mut timeout = DEFAULT_TIMEOUT;
        let mut rest = args;
        loop {
            match rest {
                [flag, value, tail @ ..] if flag.to_str() == Some("-F") => {
                    message = Some(PathBuf::from(value));
                    rest = tail;
                }
                [flag, value, tail @ ..] if flag.to_str() == Some("--log") => {
                    log = Some(absolute(Path::new(value)));
                    rest = tail;
                }
                [flag, value, tail @ ..] if flag.to_str() == Some("--timeout") => {
                    let secs = value.to_str().and_then(|v| v.parse::<u64>().ok());
                    let secs = secs.ok_or_else(|| refuse(2, "после --timeout нужны секунды"))?;
                    timeout = Duration::from_secs(secs);
                    rest = tail;
                }
                [separator, tail @ ..] if separator.to_str() == Some("--") => {
                    rest = tail;
                    break;
                }
                [] => break,
                [flag] if matches!(flag.to_str(), Some("-F" | "--log" | "--timeout")) => {
                    return Err(refuse(
                        2,
                        format!("после {} нужно значение", flag.to_string_lossy()),
                    ))
                }
                [other, ..] => {
                    return Err(refuse(
                        2,
                        format!(
                            "неизвестный аргумент {}; пути перечисляются после --",
                            other.to_string_lossy()
                        ),
                    ))
                }
            }
        }
        let message = message
            .map(|path| absolute(&path))
            .filter(|path| File::open(path).is_ok())
            .ok_or_else(|| refuse(2, "нужен -F <читаемый файл сообщения>"))?;
        if rest.is_empty() {
            return Err(refuse(
                2,
                "пути не перечислены: коммит всего изменённого запрещён",
            ));
        }
        Ok(Args {
            message,
            log,
            timeout,
            paths: rest.to_vec(),
        })
    }
}

/// Путь от текущего каталога: команда переходит в корень репозитория.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Отказ: код возврата и причина для вердикта.
struct Refusal {
    code: u8,
    reason: String,
}

fn refuse(code: u8, reason: impl Into<String>) -> Refusal {
    Refusal {
        code,
        reason: reason.into(),
    }
}

impl Refusal {
    /// Печатает строки отказа из журнала и вердикт; возвращает код.
    fn print(self, log: Option<&Path>) -> u8 {
        if let Some(text) = log.and_then(|log| fs::read(log).ok()) {
            let text = String::from_utf8_lossy(&text);
            let lines: Vec<&str> = text.lines().filter(|line| is_refusal_line(line)).collect();
            for line in &lines[lines.len().saturating_sub(REFUSAL_LINES)..] {
                println!("{line}");
            }
            if let Some(log) = log {
                println!("полный вывод: {}", log.display());
            }
        }
        println!("COMMIT REFUSED: {}", self.reason);
        self.code
    }
}

fn commit(args: &Args) -> Result<String, Refusal> {
    let repo =
        git::Repo::discover(Path::new(".")).ok_or_else(|| refuse(2, "не git-репозиторий"))?;
    let root = repo.root.as_path();
    let output = Output::open(args.log.as_deref())?;

    let text = fs::read_to_string(&args.message)
        .map_err(|_| refuse(2, "нужен -F <читаемый файл сообщения>"))?;
    match message::check_in_index(root, &text, true) {
        Ok(errors) if errors.is_empty() => {}
        Ok(errors) => {
            output.note(&message::report(&errors));
            return Err(refuse(4, "сообщение не по форме (решение 8)"));
        }
        Err(problem) => return Err(refuse(2, problem)),
    }

    let _lock = Lock::acquire(&repo.git_dir.join(layout::COMMIT_LOCK), args.timeout)
        .map_err(|problem| refuse(3, problem))?;

    let mut add = git::command(root);
    add.args(["add", "-A", "--"]).args(&args.paths);
    if !output.run(&mut add) {
        return Err(refuse(1, "git add упал на перечисленных путях"));
    }
    let staged = git::command(root)
        .args(["diff", "--cached", "--quiet", "--"])
        .args(&args.paths)
        .status();
    if staged.is_ok_and(|status| status.code() == Some(0)) {
        return Err(refuse(1, "по перечисленным путям нечего коммитить"));
    }

    let mut commit = git::command(root);
    commit
        .args(["commit", "--only", "-F"])
        .arg(&args.message)
        .arg("--")
        .args(&args.paths);
    if !output.run(&mut commit) {
        let _ = git::command(root)
            .args(["reset", "-q", "--"])
            .args(&args.paths)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        return Err(refuse(
            4,
            "git commit не создал коммит: отказ хука или ошибка самого git (см. вывод)",
        ));
    }
    git::read(root, &["rev-parse", "--short", "HEAD"])
        .ok_or_else(|| refuse(4, "коммит создан, но HEAD не читается"))
}

/// Куда идёт вывод git и хуков: в журнал или на терминал.
struct Output {
    log: Option<File>,
}

impl Output {
    fn open(path: Option<&Path>) -> Result<Output, Refusal> {
        let Some(path) = path else {
            return Ok(Output { log: None });
        };
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .map_err(|_| refuse(2, format!("не создать каталог для {}", path.display())))?;
        }
        let log = File::create(path)
            .map_err(|_| refuse(2, format!("не открыть журнал {}", path.display())))?;
        Ok(Output { log: Some(log) })
    }

    /// Собственное сообщение инструмента.
    fn note(&self, text: &str) {
        match self.log.as_ref() {
            Some(mut log) => {
                let _ = log.write_all(text.as_bytes());
            }
            None => eprint!("{text}"),
        }
    }

    /// Запускает команду с выводом в журнал или на терминал; успех — код 0.
    fn run(&self, command: &mut Command) -> bool {
        if let Some(log) = &self.log {
            let (Ok(out), Ok(err)) = (log.try_clone(), log.try_clone()) else {
                return false;
            };
            command.stdout(out).stderr(err);
        }
        command.status().is_ok_and(|status| status.success())
    }
}

/// Строка журнала, которую стоит показать при отказе.
fn is_refusal_line(line: &str) -> bool {
    ["GATE FAIL", "SELFTEST FAIL", "отвергнуто", "FAILED"]
        .iter()
        .any(|marker| line.contains(marker))
        || line.starts_with("  - ")
        || has_error_marker(line)
}

/// `error:` или `error[E0000]:` в строке.
fn has_error_marker(line: &str) -> bool {
    line.match_indices("error").any(|(at, _)| {
        let rest = &line[at + "error".len()..];
        rest.starts_with(':')
            || rest
                .strip_prefix("[E")
                .and_then(|rest| rest.split_once("]:"))
                .is_some_and(|(digits, _)| {
                    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                })
    })
}

/// Блокировка коммита: файл, созданный атомарно, с номером процесса внутри.
/// Снимается при завершении команды; блокировку умершего процесса снимает
/// ожидающий.
struct Lock {
    path: PathBuf,
}

impl Lock {
    fn acquire(path: &Path, timeout: Duration) -> Result<Lock, String> {
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Lock {
                        path: path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if holder_is_dead(path) {
                        let _ = fs::remove_file(path);
                        continue;
                    }
                }
                Err(error) => {
                    return Err(format!(
                        "блокировка {} не создаётся: {error}",
                        path.display()
                    ))
                }
            }
            if started.elapsed() >= timeout {
                return Err(format!(
                    "блокировка коммита не получена за {} с: {} ({})",
                    timeout.as_secs(),
                    describe_holder(path),
                    path.display()
                ));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Сколько файл блокировки без номера процесса считается захватываемым прямо
/// сейчас: живой процесс пишет номер сразу после атомарного создания файла.
const PIDLESS_LOCK_GRACE: Duration = Duration::from_secs(10);

/// Номер процесса из файла блокировки.
fn holder_pid(path: &Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| text.trim().parse().ok())
}

/// Блокировка брошена. С номером процесса — процесс завершился; без /proc это
/// не определить, и блокировка ждёт тайм-аута. Без номера — файл старше
/// `PIDLESS_LOCK_GRACE`: так выглядит след flock прежнего скрипта коммита или
/// процесса, умершего между созданием файла и записью номера.
fn holder_is_dead(path: &Path) -> bool {
    match holder_pid(path) {
        Some(pid) => {
            Path::new("/proc/self").exists() && !Path::new("/proc").join(pid.to_string()).exists()
        }
        None => fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > PIDLESS_LOCK_GRACE),
    }
}

/// Кто держит блокировку — для отказа по тайм-ауту.
fn describe_holder(path: &Path) -> String {
    match holder_pid(path) {
        Some(pid) => format!("держит процесс {pid}"),
        None => format!(
            "файл без номера процесса моложе {} с",
            PIDLESS_LOCK_GRACE.as_secs()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn arguments_require_message_and_paths() {
        let refusal = |list: &[&str]| Args::parse(&args(list)).err().map(|r| (r.code, r.reason));
        let (code, reason) = refusal(&["--", "a"]).unwrap();
        assert_eq!(code, 2);
        assert!(reason.contains("-F"), "{reason}");
        let (code, reason) = refusal(&["-F", "Cargo.toml", "a"]).unwrap();
        assert_eq!(code, 2);
        assert!(reason.contains("неизвестный аргумент a"), "{reason}");
        let (_, reason) = refusal(&["-F", "Cargo.toml", "--timeout", "x", "--", "a"]).unwrap();
        assert!(reason.contains("секунды"), "{reason}");
        let (_, reason) = refusal(&["-F", "Cargo.toml", "--"]).unwrap();
        assert!(reason.contains("пути не перечислены"), "{reason}");
    }

    #[test]
    fn refusal_lines_are_recognised() {
        for line in [
            "GATE FAIL: cargo clippy -D warnings (код 101)",
            "msg-check: сообщение отвергнуто (правила коммитов):",
            "  - точка в конце темы",
            "error[E0308]: mismatched types",
            "error: could not compile",
            "test attacks::e1 ... FAILED",
        ] {
            assert!(is_refusal_line(line), "{line}");
        }
        for line in [
            "Compiling slipway-core",
            "warning: unused",
            "errors are fine",
        ] {
            assert!(!is_refusal_line(line), "{line}");
        }
    }
}
