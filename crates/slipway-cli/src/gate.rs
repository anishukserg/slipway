//! Калитка коммита по решениям 8, 12, 13 и 15: один набор шагов и машинный
//! вердикт.
//!
//! ```text
//! cargo slipway gate [--repo <каталог>] [--journal-only] [<дерево>]
//! ```
//!
//! Хук pre-commit передаёт выгруженное дерево коммита; без аргумента
//! проверяется рабочее дерево. Сборка идёт в `target/gate*` корня репозитория,
//! чтобы кэш переживал выгрузки; cargo запускается из самого дерева, чтобы
//! тулчейн брался из его `rust-toolchain.toml`.
//!
//! Шаги, от дешёвых к дорогим:
//!
//! 1. внешние имена — по локальному списку в каталоге git (решение 9); без
//!    списка шаг называется невыполненным, а не пройденным;
//! 2. относительные ссылки в markdown ведут в существующие файлы;
//! 3. журнал: файлы событий из HEAD не изменены и не удалены, приземления
//!    совпадают с историей git (решение 15);
//! 4. форматирование — `cargo fmt --check`;
//! 5. clippy без предупреждений на всех целях;
//! 6. сборка всех целей с константными проверками реестров и тесты;
//! 7. сверка кодов ошибок работает: пробная атака с неверным кодом падает, с
//!    верным — проходит (решение 12);
//! 8. атаки — doctest со сверкой кодов под `RUSTC_BOOTSTRAP=1`, в отдельном
//!    каталоге сборки, с полом по числу прошедших;
//! 9. документация без предупреждений и битых внутренних ссылок;
//! 10. сборка всех целей на минимальной версии из `rust-version`;
//! 11. проба сверки кодов и атаки на минимальной версии, с тем же полом;
//! 12. зависимости — политика `deny.toml` по сохранённой базе уязвимостей, без
//!     сети (решение 13).
//!
//! С `--journal-only` — для коммита, дерево которого без журнала уже прошло
//! калитку, — выполняются шаги 1–3 и сборка крейта документов, где журнал
//! сворачивается.
//!
//! Отсутствующий инструмент — отказ шага, а не пропуск. Шаг без предмета
//! проверки называется невыполненным, а не пройденным.
//!
//! Код возврата: 0 — пройдено; 1 — шаг упал; 2 — ошибка запуска. Последняя
//! строка — `GATE OK (<n> из <m>; …)` или `GATE FAIL: <шаг>`.

use crate::{git, layout, proof};
use slipway_journal::Kind;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

/// Пол числа прошедших doctest. Только растёт: добавил атаку — подними;
/// понижение — изменение правила.
const DOCTEST_FLOOR: usize = 36;

/// Число шагов полной калитки.
const TOTAL: usize = 12;

/// Число шагов калитки журнала.
const JOURNAL_ONLY_TOTAL: usize = 4;

/// Сколько строк ошибок из журнала шага показывать при отказе.
const ERROR_LINES: usize = 40;

/// Сколько последних строк журнала шага показывать при отказе.
const TAIL_LINES: usize = 12;

/// Манифест пробы сверки кодов: собственное рабочее пространство, чтобы cargo
/// не искал родительское.
const PROBE_MANIFEST: &str = r#"[package]
name = "slipway-gate-probe"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]
"#;

/// Проба сверки кодов: неверный код обязан упасть, верный — пройти.
const PROBE_LIB: &str = r#"//! Проба сверки кодов ошибок в атаках (решение 12).
//!
//! Неверный код: тело даёт E0308, объявлен E0080 — обязана упасть.
//!
//! ```compile_fail,E0080
//! let _: u32 = "не число";
//! ```
//!
//! Верный код: то же тело — обязана пройти.
//!
//! ```compile_fail,E0308
//! let _: u32 = "не число";
//! ```
"#;

/// `cargo slipway gate`.
pub fn run(args: &[OsString]) -> u8 {
    run_with_verdict(args).0
}

/// Калитка с вердиктом: код возврата и последняя строка.
pub fn run_with_verdict(args: &[OsString]) -> (u8, String) {
    let outcome = Args::parse(args)
        .and_then(|args| Gate::open(&args))
        .and_then(Gate::check);
    let (code, verdict) = match outcome {
        Ok(verdict) => (0, verdict),
        Err(fail) => (fail.code, format!("GATE FAIL: {}", fail.step)),
    };
    println!("{verdict}");
    (code, verdict)
}

/// cargo в каталоге `dir` с каталогом сборки `build`. Переменные GIT_* хука
/// снимаются. RUSTUP_TOOLCHAIN тоже: rustup передаёт её всем дочерним
/// процессам `cargo run` хука, и она перекрыла бы `rust-toolchain.toml` дерева —
/// коммит, меняющий тулчейн, проверялся бы старым. Отсутствующий тулчейн —
/// отказ, а не загрузка внутри хука.
pub fn cargo_command(dir: &Path, build: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", build)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTUP_TOOLCHAIN_SOURCE");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    command
}

/// Имена внешних проектов из локального списка: без пустых строк и
/// комментариев, в том виде, в каком записаны. Нет списка — пустой список.
pub fn read_external_names(list: &Path) -> Vec<String> {
    fs::read_to_string(list)
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Установлена ли программа: она запускается и отвечает на `--version`.
pub fn installed(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Содержимое файла как текст; нечитаемый файл — пустая строка.
pub fn read_lossy(path: &Path) -> String {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default()
}

/// Отказ калитки: шаг и код возврата.
struct Fail {
    step: String,
    code: u8,
}

/// Шаг упал.
fn fail(step: impl Into<String>) -> Fail {
    Fail {
        step: step.into(),
        code: 1,
    }
}

/// Калитку не из чего или нечем выполнить.
fn start_fail(step: impl Into<String>) -> Fail {
    Fail {
        step: step.into(),
        code: 2,
    }
}

struct Args {
    repo: Option<PathBuf>,
    tree: Option<PathBuf>,
    journal_only: bool,
}

impl Args {
    fn parse(args: &[OsString]) -> Result<Args, Fail> {
        let mut parsed = Args {
            repo: None,
            tree: None,
            journal_only: false,
        };
        let mut rest = args;
        while let Some((first, tail)) = rest.split_first() {
            match first.to_str() {
                Some("--repo") => {
                    let (dir, tail) = tail
                        .split_first()
                        .ok_or_else(|| start_fail("после --repo нужен каталог"))?;
                    parsed.repo = Some(PathBuf::from(dir));
                    rest = tail;
                }
                Some("--journal-only") => {
                    parsed.journal_only = true;
                    rest = tail;
                }
                _ if parsed.tree.is_none() => {
                    parsed.tree = Some(PathBuf::from(first));
                    rest = tail;
                }
                _ => {
                    return Err(start_fail(format!(
                        "лишний аргумент {}",
                        first.to_string_lossy()
                    )))
                }
            }
        }
        Ok(parsed)
    }
}

struct Gate {
    root: PathBuf,
    tree: PathBuf,
    git_dir: PathBuf,
    target: PathBuf,
    journal_only: bool,
    passed: usize,
    skipped: Vec<&'static str>,
}

impl Gate {
    fn open(args: &Args) -> Result<Gate, Fail> {
        let start = args.repo.as_deref().unwrap_or(Path::new("."));
        let repo = git::Repo::discover(start).ok_or_else(|| start_fail("не git-репозиторий"))?;
        let tree = args.tree.clone().unwrap_or_else(|| repo.root.clone());
        let tree = fs::canonicalize(&tree)
            .map_err(|_| start_fail(format!("нет каталога {}", tree.display())))?;
        let target = repo.root.join(layout::GATE_TARGET);
        fs::create_dir_all(&target)
            .map_err(|_| start_fail(format!("не создать {}", target.display())))?;
        Ok(Gate {
            root: repo.root,
            tree,
            git_dir: repo.git_dir,
            target,
            journal_only: args.journal_only,
            passed: 0,
            skipped: Vec::new(),
        })
    }

    fn check(mut self) -> Result<String, Fail> {
        self.external_names()?;
        self.markdown_links()?;
        self.journal()?;
        if self.journal_only {
            return self.check_journal_build();
        }

        // Дальше запускается cargo. Без манифеста в самом дереве cargo пошёл бы
        // искать рабочее пространство в родительских каталогах и собрал бы
        // чужой проект, поэтому манифест обязателен и передаётся явно.
        let manifest = self.tree.join(layout::MANIFEST);
        if !manifest.is_file() {
            return Err(start_fail(
                "в дереве нет Cargo.toml — сборка не запускается",
            ));
        }
        let target = self.target.clone();

        let mut fmt = self.cargo(&target);
        fmt.args(["fmt", "--manifest-path"])
            .arg(&manifest)
            .args(["--all", "--check"]);
        self.cargo_step("cargo fmt --check", "fmt", &mut fmt)?;

        let mut clippy = self.cargo(&target);
        clippy
            .args(["clippy", "--manifest-path"])
            .arg(&manifest)
            .args([
                "--workspace",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ]);
        self.cargo_step("cargo clippy -D warnings", "clippy", &mut clippy)?;

        // Doctest-атаки выполняются отдельным шагом под RUSTC_BOOTSTRAP.
        let mut test = self.cargo(&target);
        test.args(["test", "--manifest-path"]).arg(&manifest).args([
            "--workspace",
            "--all-targets",
            "--no-fail-fast",
            "--locked",
        ]);
        self.cargo_step("cargo test --all-targets", "test", &mut test)?;

        // rustdoc сверяет коды compile_fail только в nightly-режиме; на stable его
        // включает RUSTC_BOOTSTRAP=1 (решение 12). Без сверки атака прошла бы на
        // любой ошибке компиляции, поэтому механизм проверяется до атак.
        self.write_probe()?;
        self.probe_codes("probe", &target.join("probe-build"), None)?;
        self.passed += 1;

        // Отдельный каталог сборки: build.rs зависимостей следят за
        // RUSTC_BOOTSTRAP, и общий кэш пересобирался бы на каждом шаге.
        let mut attacks = self.cargo(&self.build_dir("attacks"));
        attacks
            .env("RUSTC_BOOTSTRAP", "1")
            .args(["test", "--manifest-path"])
            .arg(&manifest)
            .args(["--workspace", "--doc", "--no-fail-fast", "--locked"]);
        let log = self.cargo_step("атаки: cargo test --doc", "attacks", &mut attacks)?;
        let doctests = count_doctests(&log);
        if doctests < DOCTEST_FLOOR {
            return Err(fail(format!(
                "прошло doctest {doctests} при поле {DOCTEST_FLOOR} — атаки не исполнялись или удалены"
            )));
        }

        let mut doc = self.cargo(&target);
        doc.env("RUSTDOCFLAGS", "-D warnings")
            .args(["doc", "--manifest-path"])
            .arg(&manifest)
            .args(["--workspace", "--no-deps", "--locked"]);
        self.cargo_step("cargo doc -D warnings", "doc", &mut doc)?;

        // Обещанная потребителям невыразимость проверяется на обещанном им
        // компиляторе (решение 12).
        let msrv = fs::read_to_string(&manifest)
            .ok()
            .and_then(|text| minimum_rust(&text))
            .ok_or_else(|| {
                start_fail(
                    "в Cargo.toml дерева нет rust-version — минимальная версия не проверяется",
                )
            })?;
        let toolchain = format!("+{msrv}");

        let mut msrv_check = self.cargo(&self.build_dir("msrv"));
        msrv_check
            .arg(&toolchain)
            .args(["check", "--manifest-path"])
            .arg(&manifest)
            .args(["--workspace", "--all-targets", "--locked"]);
        let label = format!("cargo {toolchain} check --all-targets");
        self.cargo_step(&label, "msrv-check", &mut msrv_check)?;

        self.probe_codes(
            "msrv-probe",
            &self.build_dir("msrv-probe"),
            Some(&toolchain),
        )?;
        let mut msrv_attacks = self.cargo(&self.build_dir("msrv-attacks"));
        msrv_attacks
            .arg(&toolchain)
            .env("RUSTC_BOOTSTRAP", "1")
            .args(["test", "--manifest-path"])
            .arg(&manifest)
            .args(["--workspace", "--doc", "--no-fail-fast", "--locked"]);
        let label = format!("атаки на {msrv}: cargo test --doc");
        let log = self.cargo_step(&label, "msrv-attacks", &mut msrv_attacks)?;
        let msrv_doctests = count_doctests(&log);
        if msrv_doctests < DOCTEST_FLOOR {
            return Err(fail(format!(
                "на {msrv} прошло doctest {msrv_doctests} при поле {DOCTEST_FLOOR} — атаки не исполнялись или удалены"
            )));
        }

        // Политика по сохранённой базе, без сети: коммит от сети не зависит.
        // Базу обновляет pre-push; без базы шаг отказывает (решение 13).
        let policy = self.tree.join(layout::DENY_POLICY);
        if !policy.is_file() {
            return Err(fail(
                "в дереве нет deny.toml — политика зависимостей не задана (решение 13)",
            ));
        }
        if !installed("cargo-deny") {
            return Err(start_fail(
                "cargo-deny не установлен — cargo install cargo-deny --locked",
            ));
        }
        let mut deny = self.cargo(&target);
        deny.args(["deny", "--manifest-path"])
            .arg(&manifest)
            .arg("--config")
            .arg(&policy)
            .args(["--frozen", "check"]);
        self.cargo_step("cargo deny check", "deny", &mut deny)?;

        let mut verdict = format!(
            "GATE OK ({} из {TOTAL}; doctest {doctests}, на {msrv} — {msrv_doctests}",
            self.passed
        );
        self.append_skipped(&mut verdict);
        Ok(verdict)
    }

    /// Калитка журнала: дерево без журнала уже проверено, осталось собрать
    /// крейт документов — сборка сворачивает журнал (решение 15).
    fn check_journal_build(mut self) -> Result<String, Fail> {
        let manifest = self.tree.join(layout::DOC_MANIFEST);
        if !manifest.is_file() {
            return Err(start_fail(
                "в дереве нет doc/Cargo.toml — журнал не свернуть",
            ));
        }
        let target = self.target.clone();
        let mut build = self.cargo(&target);
        build
            .args(["check", "--manifest-path"])
            .arg(&manifest)
            .arg("--locked");
        self.cargo_step(
            "свёртка журнала: cargo check doc",
            "journal-check",
            &mut build,
        )?;
        let mut verdict = format!(
            "GATE OK ({} из {JOURNAL_ONLY_TOTAL}; только журнал — дерево без журнала уже проверено",
            self.passed
        );
        self.append_skipped(&mut verdict);
        Ok(verdict)
    }

    fn append_skipped(&self, verdict: &mut String) {
        if !self.skipped.is_empty() {
            verdict.push_str("; не выполнялось: ");
            verdict.push_str(&self.skipped.join(", "));
        }
        verdict.push(')');
    }

    /// Шаг 1: имена внешних проектов не встречаются в файлах дерева.
    fn external_names(&mut self) -> Result<(), Fail> {
        let list = self.git_dir.join("info").join(layout::EXTERNAL_NAMES);
        let patterns: Vec<String> = read_external_names(&list)
            .iter()
            .map(|name| name.to_lowercase())
            .collect();
        if patterns.is_empty() {
            println!(
                "внешние имена: список {} пуст или не задан — шаг не выполнялся",
                list.display()
            );
            self.skipped.push("внешние имена");
            return Ok(());
        }
        let broken = |error: std::io::Error| {
            start_fail(format!("поиск внешних имён не выполнился: {error}"))
        };
        let mut hits = Vec::new();
        for file in walk(&self.tree).map_err(broken)? {
            let bytes = fs::read(&file).map_err(broken)?;
            // Двоичный файл пропускается, как у grep -I.
            if bytes.iter().take(8000).any(|&b| b == 0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes).to_lowercase();
            if patterns.iter().any(|name| text.contains(name.as_str())) {
                hits.push(relative(&self.tree, &file));
            }
        }
        if !hits.is_empty() {
            for hit in &hits {
                println!("  внешнее имя в файле: {hit}");
            }
            return Err(fail("внешние имена в дереве (решение 9)"));
        }
        self.passed += 1;
        Ok(())
    }

    /// Шаг 2: относительные ссылки в markdown ведут в существующие файлы.
    fn markdown_links(&mut self) -> Result<(), Fail> {
        let broken_walk =
            |error: std::io::Error| start_fail(format!("обход дерева не выполнился: {error}"));
        let documents: Vec<PathBuf> = walk(&self.tree)
            .map_err(broken_walk)?
            .into_iter()
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        if documents.is_empty() {
            self.skipped.push("ссылки в markdown");
            return Ok(());
        }
        let mut broken = Vec::new();
        for document in &documents {
            let text = read_lossy(document);
            let dir = document.parent().unwrap_or(&self.tree);
            for link in relative_links(&text) {
                let path = link.split('#').next().unwrap_or_default();
                if !path.is_empty() && !dir.join(path).exists() {
                    broken.push(format!("{}: {link}", relative(&self.tree, document)));
                }
            }
        }
        if !broken.is_empty() {
            for link in &broken {
                println!("  битая ссылка: {link}");
            }
            return Err(fail("относительные ссылки в markdown"));
        }
        self.passed += 1;
        Ok(())
    }

    /// Шаг 3: файлы событий из HEAD не изменены и не удалены, а каждое
    /// приземление ссылается на существующий коммит, дерево которого без
    /// журнала совпадает с деревом события. Законность переходов проверяет
    /// сборка крейта документов.
    fn journal(&mut self) -> Result<(), Fail> {
        let dir = self.tree.join(layout::JOURNAL_DIR);
        if !dir.is_dir() {
            self.skipped.push("журнал");
            return Ok(());
        }
        let mut problems = self.changed_event_files();
        let (events, _) = slipway_journal::read_dir(&dir)
            .map_err(|error| start_fail(format!("журнал не прочитан: {error}")))?;
        for event in &events {
            let Kind::Landed { commit, tree, .. } = &event.kind else {
                continue;
            };
            let object = format!("{commit}^{{commit}}");
            if !git::succeeds(&self.root, &["cat-file", "-e", &object]) {
                problems.push(format!(
                    "{}: коммита {commit} нет в репозитории",
                    event.file
                ));
            } else if proof::content_hash(&self.root, commit).as_deref() != Some(tree.as_str()) {
                problems.push(format!(
                    "{}: дерево коммита {commit} без журнала расходится с деревом события",
                    event.file
                ));
            }
        }
        if !problems.is_empty() {
            for problem in &problems {
                println!("  журнал: {problem}");
            }
            return Err(fail("журнал расходится с историей (решение 15)"));
        }
        self.passed += 1;
        Ok(())
    }

    /// Файлы событий из HEAD, изменённые или удалённые в дереве. Без HEAD —
    /// первый коммит — сравнивать не с чем.
    fn changed_event_files(&self) -> Vec<String> {
        let Some(listing) = git::read(
            &self.root,
            &["ls-tree", "-r", "-z", "HEAD", "--", layout::JOURNAL_DIR],
        ) else {
            return Vec::new();
        };
        let mut problems = Vec::new();
        let mut present = Vec::new();
        for entry in listing.split('\0') {
            let Some((meta, path)) = entry.split_once('\t') else {
                continue;
            };
            let Some(sha) = meta.split_whitespace().nth(2) else {
                continue;
            };
            if !path.ends_with(".toml") {
                continue;
            }
            let file = self.tree.join(path);
            if file.is_file() {
                present.push((sha.to_owned(), path.to_owned(), file));
            } else {
                problems.push(format!("{path}: файл события удалён"));
            }
        }
        let files: Vec<PathBuf> = present.iter().map(|(_, _, file)| file.clone()).collect();
        let hashes = proof::file_hashes(&self.root, &files);
        for ((sha, path, _), hash) in present.iter().zip(hashes) {
            if hash.as_deref() != Some(sha.as_str()) {
                problems.push(format!("{path}: файл события изменён"));
            }
        }
        problems
    }

    /// cargo из дерева с каталогом сборки `build`.
    fn cargo(&self, build: &Path) -> Command {
        cargo_command(&self.tree, build)
    }

    /// Каталог сборки рядом с основным: `target/gate-<суффикс>`.
    fn build_dir(&self, suffix: &str) -> PathBuf {
        let mut name = self.target.clone().into_os_string();
        name.push("-");
        name.push(suffix);
        PathBuf::from(name)
    }

    /// Запускает шаг с выводом в журнал; при отказе показывает строки ошибок и
    /// хвост журнала. Возвращает путь журнала.
    fn cargo_step(
        &mut self,
        label: &str,
        log: &str,
        command: &mut Command,
    ) -> Result<PathBuf, Fail> {
        let log = self.target.join(format!("{log}.log"));
        let status = run_logged(command, &log)?;
        if !status.success() {
            show_failure(&log);
            let code = status
                .code()
                .map_or_else(|| "сигнал".to_owned(), |code| code.to_string());
            return Err(fail(format!("{label} (код {code})")));
        }
        self.passed += 1;
        Ok(log)
    }

    /// Файлы пробы; перезаписываются только при изменении, иначе проба
    /// пересобиралась бы на каждом прогоне.
    fn write_probe(&self) -> Result<(), Fail> {
        let probe = self.target.join("probe");
        let write = |path: PathBuf, text: &str| -> Result<(), Fail> {
            if fs::read(&path).is_ok_and(|current| current == text.as_bytes()) {
                return Ok(());
            }
            fs::create_dir_all(path.parent().unwrap_or(&probe))
                .and_then(|()| fs::write(&path, text))
                .map_err(|error| start_fail(format!("проба не записана: {error}")))
        };
        write(probe.join("Cargo.toml"), PROBE_MANIFEST)?;
        write(probe.join("src").join("lib.rs"), PROBE_LIB)
    }

    /// Проба сверки кодов тулчейном дерева или явным `toolchain`.
    fn probe_codes(&self, log: &str, build: &Path, toolchain: Option<&str>) -> Result<(), Fail> {
        let mut command = self.cargo(build);
        if let Some(toolchain) = toolchain {
            command.arg(toolchain);
        }
        command
            .env("RUSTC_BOOTSTRAP", "1")
            .args(["test", "--manifest-path"])
            .arg(self.target.join("probe").join("Cargo.toml"))
            .arg("--doc");
        let log = self.target.join(format!("{log}.log"));
        run_logged(&mut command, &log)?;
        let text = read_lossy(&log);
        let wrong_code_refused = text.contains("Some expected error codes were not found");
        let one_of_each = text
            .lines()
            .any(|line| line.starts_with("test result: FAILED. 1 passed; 1 failed;"));
        if wrong_code_refused && one_of_each {
            return Ok(());
        }
        show_failure(&log);
        let on = toolchain.map(|t| format!(" на {t}")).unwrap_or_default();
        Err(fail(format!(
            "сверка кодов ошибок в атаках не работает{on} — атаки прошли бы вакуумно (решение 12)"
        )))
    }
}

/// Запускает команду с выводом в журнал `log`.
fn run_logged(command: &mut Command, log: &Path) -> Result<ExitStatus, Fail> {
    let not_opened =
        |error: std::io::Error| start_fail(format!("журнал {} не открыт: {error}", log.display()));
    let out = File::create(log).map_err(not_opened)?;
    let err = out.try_clone().map_err(not_opened)?;
    command
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .status()
        .map_err(|error| {
            start_fail(format!(
                "{} не запустился: {error}",
                command.get_program().to_string_lossy()
            ))
        })
}

/// Строки ошибок, затем хвост журнала и путь к нему.
fn show_failure(log: &Path) {
    let text = read_lossy(log);
    let lines: Vec<&str> = text.lines().collect();
    for line in lines
        .iter()
        .filter(|line| is_error_line(line))
        .take(ERROR_LINES)
    {
        println!("{line}");
    }
    for line in &lines[lines.len().saturating_sub(TAIL_LINES)..] {
        println!("{line}");
    }
    println!("полный вывод: {}", log.display());
}

/// Путь файла от корня дерева — для сообщений.
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Файлы дерева по порядку. Каталоги `target` и `.git` отсекаются по имени на
/// любом уровне внутри дерева, а не по подстроке пути: дерево коммита
/// выгружается внутрь каталога git, и исключение по подстроке «/.git/»
/// отсекало бы все его файлы.
fn walk(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                let name = entry.file_name();
                if !matches!(name.to_str(), Some("target" | ".git")) {
                    dirs.push(entry.path());
                }
            } else if kind.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Цели ссылок `](цель)` без пробелов, кроме внешних адресов и якорей.
///
/// Код документа ссылкой не считается: в тексте о правилах коммитов форма
/// темы — пример, а не путь к файлу (работа 31).
fn relative_links(text: &str) -> Vec<&str> {
    let mut links = Vec::new();
    for prose in prose_segments(text) {
        let mut rest = prose;
        while let Some(at) = rest.find("](") {
            rest = &rest[at + 2..];
            let end = rest
                .find(|c: char| c == ')' || c.is_whitespace())
                .unwrap_or(rest.len());
            let target = &rest[..end];
            let external = ["http:", "https:", "mailto:", "#"]
                .iter()
                .any(|prefix| target.starts_with(prefix));
            if rest[end..].starts_with(')') && !target.is_empty() && !external {
                links.push(target);
            }
        }
    }
    links
}

/// Куски текста вне кода: огороженные блоки и код в обратных кавычках
/// выброшены.
///
/// Ограда — три и больше знаков `` ` `` или `~` в начале строки; закрывающая
/// ограда того же знака и не короче открывающей, а незакрытая съедает текст до
/// конца, как в CommonMark. Отступ в четыре пробела кодом здесь не считается:
/// в документах Slipway код пишут оградой.
fn prose_segments(text: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let mark = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        if let Some(mark) = mark {
            let run = trimmed.chars().take_while(|c| *c == mark).count();
            if run >= 3 {
                match fence {
                    Some((open, len)) if open == mark && run >= len => {
                        fence = None;
                        continue;
                    }
                    Some(_) => {}
                    None => {
                        fence = Some((mark, run));
                        continue;
                    }
                }
            }
        }
        if fence.is_none() {
            segments.extend(outside_code_spans(line));
        }
    }
    segments
}

/// Куски строки вне кода в обратных кавычках: код закрывается таким же числом
/// кавычек подряд, а незакрытый — концом строки.
fn outside_code_spans(line: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let bytes = line.as_bytes();
    let mut at = 0;
    let mut start = 0;
    while at < bytes.len() {
        if bytes[at] != b'`' {
            at += 1;
            continue;
        }
        let open = bytes[at..].iter().take_while(|byte| **byte == b'`').count();
        parts.push(&line[start..at]);
        let mut cursor = at + open;
        let close = loop {
            let Some(next) = line[cursor..].find('`') else {
                break None;
            };
            let found = cursor + next;
            let run = bytes[found..]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            if run == open {
                break Some(found + run);
            }
            cursor = found + run;
        };
        match close {
            Some(end) => {
                at = end;
                start = end;
            }
            None => {
                at = bytes.len();
                start = bytes.len();
            }
        }
    }
    parts.push(&line[start..]);
    parts
}

/// Минимальная версия из `rust-version = "1.83"` в виде тулчейна `1.83.0`.
fn minimum_rust(manifest: &str) -> Option<String> {
    manifest.lines().find_map(|line| {
        let version = line.strip_prefix("rust-version = \"")?.strip_suffix('"')?;
        let parts: Vec<&str> = version.split('.').collect();
        let numeric = parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
        match (numeric, parts.len()) {
            (true, 2) => Some(format!("{version}.0")),
            (true, 3) => Some(version.to_owned()),
            _ => None,
        }
    })
}

/// Число прошедших doctest в журнале cargo test.
fn count_doctests(log: &Path) -> usize {
    read_lossy(log)
        .lines()
        .filter(|line| is_passed_doctest(line))
        .count()
}

/// `test <файл> - <элемент> (line N)[ - compile fail] ... ok`.
fn is_passed_doctest(line: &str) -> bool {
    let Some(rest) = line
        .strip_prefix("test ")
        .and_then(|rest| rest.strip_suffix(" ... ok"))
    else {
        return false;
    };
    let rest = rest.strip_suffix(" - compile fail").unwrap_or(rest);
    let Some(open) = rest.rfind("(line ") else {
        return false;
    };
    let Some(number) = rest[open + "(line ".len()..].strip_suffix(')') else {
        return false;
    };
    let head = &rest[..open];
    !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && head
            .split_once(" - ")
            .is_some_and(|(file, item)| !file.is_empty() && !item.trim().is_empty())
}

/// Строка журнала cargo, которую стоит показать при отказе шага.
fn is_error_line(line: &str) -> bool {
    let tagged = ["error", "warning", "bug"].into_iter().any(|kind| {
        let Some(rest) = line.strip_prefix(kind) else {
            return false;
        };
        let rest = match rest.strip_prefix('[') {
            Some(code) => match code.split_once(']') {
                Some((code, rest))
                    if !code.is_empty()
                        && code
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-') =>
                {
                    rest
                }
                _ => return false,
            },
            None => rest,
        };
        rest.starts_with(':')
    });
    tagged
        || (line.starts_with("test ") && line.ends_with(" FAILED"))
        || line.starts_with("---- ")
        || line.starts_with("Diff in ")
        || line.starts_with("Some expected error codes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passed_doctests_are_counted_by_their_line_form() {
        for line in [
            "test crates/slipway-core/src/nonempty.rs - nonempty::NonEmpty (line 12) ... ok",
            "test crates/slipway-knowledge/src/attacks.rs - attacks (line 11) - compile fail ... ok",
        ] {
            assert!(is_passed_doctest(line), "{line}");
        }
        for line in [
            "test attacks::e1 ... ok",
            "test crates/a.rs - x (line 3) ... FAILED",
            "test crates/a.rs - x (line three) ... ok",
            "test result: ok. 36 passed; 0 failed",
        ] {
            assert!(!is_passed_doctest(line), "{line}");
        }
    }

    #[test]
    fn error_lines_are_recognised() {
        for line in [
            "error[E0080]: evaluation of constant value failed",
            "error[wildcard]: found 3 wildcard dependencies",
            "error: could not compile `slipway-cli`",
            "warning: unused import",
            "bug[unresolved-workspace-dependency]: failed to resolve",
            "test attacks::e1 ... FAILED",
            "---- attacks::e1 stdout ----",
            "Diff in /tree/src/lib.rs:3:",
            "Some expected error codes were not found: [\"E0080\"]",
        ] {
            assert!(is_error_line(line), "{line}");
        }
        for line in [
            "   Compiling slipway-core",
            "errors: none",
            "error[]: x",
            "test x ... ok",
        ] {
            assert!(!is_error_line(line), "{line}");
        }
    }

    #[test]
    fn only_relative_links_are_checked() {
        let text =
            "[a](b.md) [c](https://x.org) [d](#якорь) [e](dir/f.md#раздел) [g](with space) [h]()";
        assert_eq!(relative_links(text), ["b.md", "dir/f.md#раздел"]);
    }

    #[test]
    fn code_in_a_document_is_not_a_link() {
        // Форма темы коммита в тексте документа — код, а не ссылка на файл
        // «область»: шаг отвергал документ за форму записи (работа 31).
        let text = concat!(
            "тема `[ТИП](область): суть`\n\n",
            "```text\n[FEAT](cli): суть\n```\n\n",
            "а это ссылка: [файл](real.md)\n"
        );
        assert_eq!(relative_links(text), ["real.md"]);
    }

    #[test]
    fn minimum_rust_becomes_a_toolchain() {
        assert_eq!(
            minimum_rust("[workspace.package]\nrust-version = \"1.83\"\n").as_deref(),
            Some("1.83.0")
        );
        assert_eq!(
            minimum_rust("rust-version = \"1.83.2\"").as_deref(),
            Some("1.83.2")
        );
        assert_eq!(minimum_rust("rust-version.workspace = true"), None);
        assert_eq!(minimum_rust("rust-version = \"1.x\""), None);
    }
}
