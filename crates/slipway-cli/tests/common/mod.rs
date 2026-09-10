//! Временные репозитории для сценариев правил (решение 14).
//!
//! Репозиторий создаётся в каталоге сборки тестов, а не в TMPDIR: TMPDIR может
//! указывать внутрь чужого проекта. Дочерние git и инструмент запускаются без
//! переменных GIT_*: хук, запустивший тесты через калитку, передаёт их, и они
//! направили бы запись во временном репозитории в индекс коммита, ради которого
//! запущен хук.

// Каждый файл сценариев пользуется своей частью помощников.
#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// Бинарь инструмента, собранный для этих тестов.
pub const BIN: &str = env!("CARGO_BIN_EXE_cargo-slipway");

/// Итог запуска: код возврата и вывод.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    fn from(output: Output) -> Run {
        Run {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Последняя строка стандартного вывода — вердикт.
    pub fn verdict(&self) -> &str {
        self.stdout.lines().last().unwrap_or("")
    }

    /// Весь вывод — для сообщения упавшего теста.
    pub fn output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Временный репозиторий сценария. Каталог сценария удаляется, если сценарий
/// не упал; упавший остаётся для разбора.
pub struct TempRepo {
    case: PathBuf,
    pub root: PathBuf,
}

impl TempRepo {
    /// Пустой репозиторий с настроенным автором.
    pub fn new(name: &str) -> TempRepo {
        TempRepo::inside(name, "")
    }

    /// Репозиторий в подкаталоге `parent` каталога сценария: так проверяются
    /// фильтры путей, срабатывающие на имя родительского каталога.
    pub fn inside(name: &str, parent: &str) -> TempRepo {
        let case = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("slipway-cli")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&case);
        let root = case.join(parent).join("repo");
        fs::create_dir_all(&root).expect("каталог сценария не создан");
        let repo = TempRepo { case, root };
        repo.git(&["init", "-q"]);
        repo.git(&["config", "user.name", "selftest"]);
        repo.git(&["config", "user.email", "selftest@localhost"]);
        repo.git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Файл в рабочем дереве; каталоги создаются.
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().expect("у файла есть каталог")).expect("каталог файла");
        fs::write(path, text).expect("файл не записан");
    }

    /// Исполняемый файл в рабочем дереве.
    pub fn executable(&self, relative: &str, text: &str) {
        self.write(relative, text);
        let path = self.path(relative);
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("права не выставлены");
    }

    /// Файл вне рабочего дерева — например, сообщение коммита.
    pub fn outside(&self, name: &str, text: &str) -> PathBuf {
        let path = self.case.join(name);
        fs::write(&path, text).expect("файл не записан");
        path
    }

    /// git в репозитории; тест падает, если git отказал.
    pub fn git(&self, args: &[&str]) -> String {
        let output = clean("git")
            .current_dir(&self.root)
            .args(args)
            .output()
            .expect("git не запустился");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// Код возврата git без проверки успеха.
    pub fn git_code(&self, args: &[&str]) -> i32 {
        clean("git")
            .current_dir(&self.root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git не запустился")
            .code()
            .unwrap_or(-1)
    }

    /// Инструмент в корне репозитория.
    pub fn tool(&self, args: &[&str]) -> Run {
        self.tool_with_input(args, "")
    }

    /// Инструмент в корне репозитория со стандартным вводом.
    pub fn tool_with_input(&self, args: &[&str], input: &str) -> Run {
        let mut child = clean(BIN)
            .current_dir(&self.root)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("инструмент не запустился");
        child
            .stdin
            .take()
            .expect("стандартный ввод")
            .write_all(input.as_bytes())
            .expect("ввод не передан");
        Run::from(child.wait_with_output().expect("инструмент не завершился"))
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.case);
        }
    }
}

/// Команда без переменных GIT_* родительского процесса.
pub fn clean(program: &str) -> Command {
    let mut command = Command::new(program);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
}
