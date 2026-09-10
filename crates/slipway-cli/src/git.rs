//! Запуск git.

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

/// Репозиторий: корень рабочего дерева и каталог git.
pub struct Repo {
    pub root: PathBuf,
    pub git_dir: PathBuf,
}

impl Repo {
    /// Репозиторий, которому принадлежит каталог `dir`.
    pub fn discover(dir: &Path) -> Option<Repo> {
        let root = read(dir, &["rev-parse", "--show-toplevel"])?;
        let git_dir = read(dir, &["rev-parse", "--absolute-git-dir"])?;
        Some(Repo {
            root: PathBuf::from(root),
            git_dir: PathBuf::from(git_dir),
        })
    }
}
