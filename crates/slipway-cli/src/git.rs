//! Запуск git.

use crate::config::Config;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Команда git в каталоге `dir`.
pub fn command(dir: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir);
    cmd
}

/// Стандартный вывод git без завершающих переводов строки; `None`, если git не
/// запустился или завершился с ошибкой.
pub fn read(dir: &Path, args: &[&str]) -> Option<String> {
    let out = command(dir)
        .args(args)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
}

/// Завершилась ли команда git успехом; вывод отбрасывается.
pub fn succeeds(dir: &Path, args: &[&str]) -> bool {
    command(dir)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Репозиторий: корень рабочего дерева, каталог git и настройка рабочего
/// дерева (решение 20).
pub struct Repo {
    pub root: PathBuf,
    pub git_dir: PathBuf,
    pub config: Config,
}

impl Repo {
    /// Репозиторий, которому принадлежит каталог `dir`, с настройкой из его
    /// рабочего дерева. `Err` — не репозиторий или негодная настройка: по
    /// испорченной настройке инструмент не работает.
    pub fn discover(dir: &Path) -> Result<Repo, String> {
        let root = read(dir, &["rev-parse", "--show-toplevel"])
            .ok_or_else(|| "not a git repository".to_owned())?;
        let git_dir = read(dir, &["rev-parse", "--absolute-git-dir"])
            .ok_or_else(|| "not a git repository".to_owned())?;
        let root = PathBuf::from(root);
        let config = Config::read_dir(&root)?;
        Ok(Repo {
            root,
            git_dir: PathBuf::from(git_dir),
            config,
        })
    }
}
