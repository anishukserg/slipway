//! Команды работы и среза (решение 15): события журнала пишет инструмент, а не
//! человек руками, и сразу коммитит их по правилам коммитов.
//!
//! ```text
//! cargo slipway work start <wNNNN> [--trailer <trailer>]…
//! cargo slipway work land <wNNNN> [--commit <revision>] [--trailer <trailer>]…
//! cargo slipway work drop <wNNNN> --reason <reason> [--trailer <trailer>]…
//! cargo slipway work state [<wNNNN>]
//! cargo slipway slice close <sNNNN> [--trailer <trailer>]…
//! ```
//!
//! Незаконный переход, отсутствующая работа и приземление без доказательства
//! отвергаются до записи события. Последняя строка — вердикт команды commit или
//! `WORK REFUSED: <reason>`.
//!
//! Код возврата: 0 — событие записано и закоммичено или свёртка напечатана;
//! 1 — переход незаконен, нет доказательства или журнал не сворачивается;
//! 2 — неверные аргументы или окружение; прочие коды — коды команды commit.

use crate::{commit, git, layout, proof};
use slipway_journal::event::relative_path;
use slipway_journal::{fold, time, Event, Evidence, Journal, Kind, Stage, Subject};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const WORK_USAGE: &str = "work start <wNNNN> | land <wNNNN> [--commit <revision>] | drop <wNNNN> --reason <reason> | state [<wNNNN>]";

const SLICE_USAGE: &str = "slice close <sNNNN>";

/// `cargo slipway work …`.
pub fn run_work(args: &[OsString]) -> u8 {
    finish(words(args).and_then(|(words, trailers)| {
        match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["start", id] => start(id, &trailers),
            ["land", id] => land(id, "HEAD", &trailers),
            ["land", id, "--commit", revision] => land(id, revision, &trailers),
            ["drop", id, "--reason", reason] => abandon(id, reason, &trailers),
            ["state"] => state(None),
            ["state", id] => state(Some(id)),
            _ => Err(usage(WORK_USAGE)),
        }
    }))
}

/// `cargo slipway slice …`.
pub fn run_slice(args: &[OsString]) -> u8 {
    finish(words(args).and_then(|(words, trailers)| {
        match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["close", id] => close(id, &trailers),
            _ => Err(usage(SLICE_USAGE)),
        }
    }))
}

/// Отказ команды: код и причина.
struct Refusal {
    code: u8,
    reason: String,
}

fn refused(reason: impl Into<String>) -> Refusal {
    Refusal {
        code: 1,
        reason: reason.into(),
    }
}

fn usage(reason: impl Into<String>) -> Refusal {
    Refusal {
        code: 2,
        reason: reason.into(),
    }
}

fn finish(outcome: Result<u8, Refusal>) -> u8 {
    match outcome {
        Ok(code) => code,
        Err(refusal) => {
            println!("WORK REFUSED: {}", refusal.reason);
            refusal.code
        }
    }
}

/// Аргументы словами и строки `--trailer`, извлечённые из любого места.
fn words(args: &[OsString]) -> Result<(Vec<String>, Vec<String>), Refusal> {
    let mut words = Vec::new();
    let mut trailers = Vec::new();
    let mut rest = args.iter().map(|arg| arg.to_string_lossy().into_owned());
    while let Some(word) = rest.next() {
        if word == "--trailer" {
            let trailer = rest
                .next()
                .ok_or_else(|| usage("--trailer needs a trailer line"))?;
            if !trailer.contains(": ") {
                return Err(usage(format!(
                    "trailer `{trailer}` must be of the form `Key: value`"
                )));
            }
            trailers.push(trailer);
        } else {
            words.push(word);
        }
    }
    Ok((words, trailers))
}

fn start(id: &str, trailers: &[String]) -> Result<u8, Refusal> {
    let context = Context::open()?;
    let number = work_number(id)?;
    let work = context.record(layout::WORK_DIR, id)?;
    let stage = context.journal.stage(number);
    if stage != Stage::Planned {
        return Err(refused(format!(
            "work {id} is already {}",
            stage_text(stage)
        )));
    }
    if let Some(slice) = work.slice {
        if let Some(file) = context.journal.closed_slices.get(&slice) {
            return Err(refused(format!(
                "slice s{slice:04} of work {id} is closed by event {file}"
            )));
        }
    }
    let area = work.area(id)?;
    let event = Event::new(Subject::Work(number), time::now(), Kind::Started);
    let message = message(
        &format!("[PLAN]({area}): work {id} started"),
        &work.title,
        &format!("Slipway-Work: {id}"),
        trailers,
    );
    record_and_commit(&context.repo, vec![event], &message)
}

fn land(id: &str, revision: &str, trailers: &[String]) -> Result<u8, Refusal> {
    let context = Context::open()?;
    let number = work_number(id)?;
    let work = context.record(layout::WORK_DIR, id)?;
    let stage = context.journal.stage(number);
    if stage != Stage::Started {
        return Err(refused(format!(
            "only a started work can be landed, and {id} is {}",
            stage_text(stage)
        )));
    }
    let root = &context.repo.root;
    let object = format!("{revision}^{{commit}}");
    let commit = git::read(root, &["rev-parse", "--verify", "--quiet", &object])
        .ok_or_else(|| refused(format!("revision {revision} is not a commit")))?;
    if !git::succeeds(root, &["merge-base", "--is-ancestor", &commit, "HEAD"]) {
        return Err(refused(format!(
            "commit {} is not in the history of HEAD",
            short(&commit)
        )));
    }
    let trailer = format!("Slipway-Work: {id}");
    let body = git::read(root, &["log", "-1", "--format=%B", &commit]).unwrap_or_default();
    if !body.lines().any(|line| line == trailer) {
        return Err(refused(format!(
            "commit {} is not based on work {id}: it has no `{trailer}` trailer",
            short(&commit)
        )));
    }
    let tree = proof::content_hash(root, &commit).ok_or_else(|| {
        usage(format!(
            "the tree of commit {} cannot be read",
            short(&commit)
        ))
    })?;
    let verdict = proof::verdict(&context.repo.git_dir, &tree).ok_or_else(|| {
        refused(format!(
            "no proof for tree {} of commit {}: the gate did not pass on this tree here — the commit was made without the hook or on another machine",
            short(&tree),
            short(&commit)
        ))
    })?;
    let area = work.area(id)?;
    let at = time::now();
    let gate = Event::new(
        Subject::Work(number),
        at.clone(),
        Kind::Gate {
            gate: "commit".to_owned(),
            tree: tree.clone(),
            verdict,
        },
    );
    let landed = Event::new(
        Subject::Work(number),
        at,
        Kind::Landed {
            commit: commit.clone(),
            tree,
            evidence: Evidence::Gate,
        },
    );
    let message = message(
        &format!("[PLAN]({area}): work {id} landed"),
        &format!("{}\nCommit {}.", work.title, short(&commit)),
        &trailer,
        trailers,
    );
    record_and_commit(&context.repo, vec![gate, landed], &message)
}

fn abandon(id: &str, reason: &str, trailers: &[String]) -> Result<u8, Refusal> {
    if reason.trim().is_empty() {
        return Err(usage("a non-empty reason is required: --reason <reason>"));
    }
    let context = Context::open()?;
    let number = work_number(id)?;
    let work = context.record(layout::WORK_DIR, id)?;
    let stage = context.journal.stage(number);
    if stage.is_finished() {
        return Err(refused(format!(
            "work {id} is already {}",
            stage_text(stage)
        )));
    }
    let area = work.area(id)?;
    let event = Event::new(
        Subject::Work(number),
        time::now(),
        Kind::Abandoned {
            reason: reason.trim().to_owned(),
        },
    );
    let message = message(
        &format!("[PLAN]({area}): work {id} abandoned"),
        &format!("{}\nReason: {}", work.title, reason.trim()),
        &format!("Slipway-Work: {id}"),
        trailers,
    );
    record_and_commit(&context.repo, vec![event], &message)
}

fn state(id: Option<&str>) -> Result<u8, Refusal> {
    let context = Context::open()?;
    let selected = id.map(work_number).transpose()?;
    let works = context.works()?;
    if let (Some(id), Some(number)) = (id, selected) {
        if !works.iter().any(|(n, _)| *n == number) {
            return Err(refused(format!(
                "{id} is not in the plan: no file {}/{id}.rs",
                layout::WORK_DIR
            )));
        }
    }
    for (number, work) in works
        .iter()
        .filter(|(n, _)| selected.is_none_or(|selected| selected == *n))
    {
        let slice = work
            .slice
            .map_or_else(|| "s????".to_owned(), |s| format!("s{s:04}"));
        println!(
            "w{number:04}  {:<22}  {slice}  {}",
            stage_text(context.journal.stage(*number)),
            work.title
        );
    }
    if selected.is_none() {
        let closed: Vec<String> = context
            .journal
            .closed_slices
            .keys()
            .map(|slice| format!("s{slice:04}"))
            .collect();
        if closed.is_empty() {
            println!("no closed slices");
        } else {
            println!("closed slices: {}", closed.join(", "));
        }
    }
    Ok(0)
}

fn close(id: &str, trailers: &[String]) -> Result<u8, Refusal> {
    let number = match Subject::parse(id) {
        Some(Subject::Slice(number)) => number,
        _ => return Err(usage(format!("{id} is not a slice id of the form s0001"))),
    };
    let context = Context::open()?;
    let slice = context.record(layout::SLICE_DIR, id)?;
    if let Some(file) = context.journal.closed_slices.get(&number) {
        return Err(refused(format!(
            "slice {id} is already closed by event {file}"
        )));
    }
    let works: Vec<(u32, Record)> = context
        .works()?
        .into_iter()
        .filter(|(_, work)| work.slice == Some(number))
        .collect();
    let Some((first, first_work)) = works.first() else {
        return Err(refused(format!("slice {id} has no works")));
    };
    let unfinished: Vec<String> = works
        .iter()
        .filter(|(n, _)| !context.journal.stage(*n).is_finished())
        .map(|(n, _)| format!("w{n:04}"))
        .collect();
    if !unfinished.is_empty() {
        return Err(refused(format!(
            "slice {id} is not closed: {} not finished",
            unfinished.join(", ")
        )));
    }
    let area = first_work.area(&format!("w{first:04}"))?;
    let event = Event::new(Subject::Slice(number), time::now(), Kind::Closed);
    let message = message(
        &format!("[PLAN]({area}): slice {id} closed"),
        &slice.title,
        &format!("Slipway-Slice: {id}"),
        trailers,
    );
    record_and_commit(&context.repo, vec![event], &message)
}

/// Репозиторий и свёртка журнала рабочего дерева.
struct Context {
    repo: git::Repo,
    journal: Journal,
}

impl Context {
    /// Открывает репозиторий и сворачивает журнал. Несворачиваемый журнал —
    /// отказ: новое событие поверх нарушения ничего не прояснит.
    fn open() -> Result<Context, Refusal> {
        let repo =
            git::Repo::discover(Path::new(".")).ok_or_else(|| usage("not a git repository"))?;
        let dir = repo.root.join(layout::JOURNAL_DIR);
        let (events, read_violations) = slipway_journal::read_dir(&dir)
            .map_err(|error| usage(format!("journal {} not read: {error}", dir.display())))?;
        let (journal, fold_violations) = fold(&events);
        if let Some(violation) = read_violations.iter().chain(&fold_violations).next() {
            return Err(refused(format!(
                "journal does not fold: {}: {}",
                violation.file, violation.reason
            )));
        }
        Ok(Context { repo, journal })
    }

    /// Запись плана `<каталог>/<id>.rs`.
    fn record(&self, dir: &str, id: &str) -> Result<Record, Refusal> {
        let path = self.repo.root.join(dir).join(format!("{id}.rs"));
        fs::read_to_string(&path)
            .map(|text| Record::parse(&text))
            .map_err(|_| refused(format!("{id} is not in the plan: no file {dir}/{id}.rs")))
    }

    /// Все работы плана по порядку номеров.
    fn works(&self) -> Result<Vec<(u32, Record)>, Refusal> {
        let dir = self.repo.root.join(layout::WORK_DIR);
        let entries = fs::read_dir(&dir)
            .map_err(|error| usage(format!("plan {} not read: {error}", dir.display())))?;
        let mut works = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(Subject::Work(number)) = name.strip_suffix(".rs").and_then(Subject::parse)
            else {
                continue;
            };
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            works.push((number, Record::parse(&text)));
        }
        works.sort_by_key(|(number, _)| *number);
        Ok(works)
    }
}

/// То, что команды читают из текста файла работы или среза.
struct Record {
    title: String,
    slice: Option<u32>,
    area: Option<String>,
}

impl Record {
    fn parse(text: &str) -> Record {
        const SLICE: &str = "slice: crate::slice::s";
        const TAXON: &str = "taxon!(Subsystem, ";
        Record {
            title: quoted_after(text, "title: NonEmptyStr::new(").unwrap_or_default(),
            slice: text
                .find(SLICE)
                .and_then(|at| text.get(at + SLICE.len()..at + SLICE.len() + 4))
                .and_then(|digits| digits.parse().ok()),
            area: text
                .find(TAXON)
                .and_then(|at| text[at + TAXON.len()..].split_once(')'))
                .map(|(name, _)| name.trim().to_lowercase()),
        }
    }

    /// Область темы коммита — подсистема работы.
    fn area(&self, id: &str) -> Result<&str, Refusal> {
        self.area.as_deref().ok_or_else(|| {
            refused(format!(
                "the subsystem taxon!(Subsystem, …) of {id} cannot be read"
            ))
        })
    }
}

/// `cargo slipway journal import --work <wNNNN> [--close-finished-slices]`.
pub fn run_import(args: &[OsString]) -> u8 {
    finish(words(args).and_then(|(words, trailers)| {
        match words
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            ["--work", basis] => import(basis, false, &trailers),
            ["--work", basis, "--close-finished-slices"]
            | ["--close-finished-slices", "--work", basis] => import(basis, true, &trailers),
            _ => Err(usage(
                "journal import --work <wNNNN> [--close-finished-slices]",
            )),
        }
    }))
}

/// Приземления из истории (решение 15): каждая запланированная работа, у
/// которой в истории HEAD есть коммит с её трейлером, приземляется по
/// последнему такому коммиту. Работа без коммита остаётся запланированной —
/// импорт не угадывает. Основание импорта пропускается: оно ещё в работе.
/// С `close_slices` закрываются срезы, все работы которых после импорта
/// завершены.
fn import(basis: &str, close_slices: bool, trailers: &[String]) -> Result<u8, Refusal> {
    let context = Context::open()?;
    let basis_number = work_number(basis)?;
    let area = context
        .record(layout::WORK_DIR, basis)?
        .area(basis)?
        .to_owned();
    let root = &context.repo.root;

    // История идёт от новых коммитов к старым: первый встреченный — последний.
    let log = git::read(
        root,
        &[
            "log",
            "--format=%H %(trailers:key=Slipway-Work,valueonly,separator=%x20)",
            "HEAD",
        ],
    )
    .unwrap_or_default();
    let mut latest = std::collections::BTreeMap::new();
    for line in log.lines() {
        let mut fields = line.split_whitespace();
        let Some(commit) = fields.next() else {
            continue;
        };
        for id in fields {
            if let Some(Subject::Work(number)) = Subject::parse(id) {
                latest.entry(number).or_insert_with(|| commit.to_owned());
            }
        }
    }

    let at = time::now();
    let works = context.works()?;
    let mut events = Vec::new();
    let mut imported = std::collections::BTreeSet::new();
    for (number, _) in &works {
        if *number == basis_number || context.journal.stage(*number) != Stage::Planned {
            continue;
        }
        let Some(commit) = latest.get(number) else {
            continue;
        };
        let tree = proof::content_hash(root, commit).ok_or_else(|| {
            usage(format!(
                "the tree of commit {} cannot be read",
                short(commit)
            ))
        })?;
        events.push(Event::new(
            Subject::Work(*number),
            at.clone(),
            Kind::Landed {
                commit: commit.clone(),
                tree,
                evidence: Evidence::History,
            },
        ));
        imported.insert(*number);
    }

    let mut closed = Vec::new();
    if close_slices {
        let mut finished = std::collections::BTreeMap::new();
        for (number, work) in &works {
            let Some(slice) = work.slice else {
                continue;
            };
            let done = context.journal.stage(*number).is_finished() || imported.contains(number);
            let all = finished.entry(slice).or_insert(true);
            *all = *all && done;
        }
        for (slice, done) in finished {
            if done && !context.journal.closed_slices.contains_key(&slice) {
                events.push(Event::new(Subject::Slice(slice), at.clone(), Kind::Closed));
                closed.push(format!("s{slice:04}"));
            }
        }
    }

    if events.is_empty() {
        return Err(refused(
            "nothing to import: planned works have no commits carrying their trailer",
        ));
    }
    let imported: Vec<String> = imported.iter().map(|n| format!("w{n:04}")).collect();
    let body = format!(
        "Landed from history: {}.\nClosed slices: {}.",
        listed(&imported),
        listed(&closed)
    );
    let message = message(
        &format!("[PLAN]({area}): journal restored from history"),
        &body,
        &format!("Slipway-Work: {basis}"),
        trailers,
    );
    record_and_commit(&context.repo, events, &message)
}

fn listed(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// Строка в кавычках после `marker`; между маркером и кавычкой допустимы
/// пробелы и переводы строк.
fn quoted_after(text: &str, marker: &str) -> Option<String> {
    let rest = text[text.find(marker)? + marker.len()..]
        .trim_start()
        .strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.push(chars.next()?),
            '"' => return Some(out),
            c => out.push(c),
        }
    }
    None
}

/// Номер работы из `wNNNN`.
fn work_number(id: &str) -> Result<u32, Refusal> {
    match Subject::parse(id) {
        Some(Subject::Work(number)) => Ok(number),
        _ => Err(usage(format!("{id} is not a work id of the form w0001"))),
    }
}

fn stage_text(stage: Stage) -> &'static str {
    match stage {
        Stage::Planned => "planned",
        Stage::Started => "started",
        Stage::Landed => "landed",
        Stage::LandedFromHistory => "landed from history",
        Stage::Abandoned => "abandoned",
    }
}

fn short(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// Сообщение коммита события: тема, тело, трейлер основания и добавленные
/// трейлеры.
fn message(subject: &str, body: &str, basis: &str, trailers: &[String]) -> String {
    let mut text = format!("{subject}\n\n{body}\n\n{basis}\n");
    for trailer in trailers {
        text.push_str(trailer);
        text.push('\n');
    }
    text
}

/// Записывает события в журнал и коммитит ровно их. Если коммит не создан,
/// записанные файлы удаляются: событие без коммита — не история.
fn record_and_commit(repo: &git::Repo, events: Vec<Event>, message: &str) -> Result<u8, Refusal> {
    let journal = repo.root.join(layout::JOURNAL_DIR);
    let mut written: Vec<PathBuf> = Vec::new();
    for event in events {
        let mut attempt = 0;
        let mut relative = event.file.clone();
        while journal.join(&relative).exists() {
            attempt += 1;
            relative = relative_path(event.subject, &event.at, &event.kind, attempt);
        }
        let path = journal.join(&relative);
        let text = Event {
            file: relative,
            ..event
        }
        .to_text();
        let stored = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|()| fs::write(&path, text));
        if let Err(error) = stored {
            remove(&written);
            return Err(usage(format!(
                "event {} not written: {error}",
                path.display()
            )));
        }
        written.push(path);
    }
    let message_file = repo.git_dir.join(layout::JOURNAL_MESSAGE);
    if let Err(error) = fs::write(&message_file, message) {
        remove(&written);
        return Err(usage(format!("commit message not written: {error}")));
    }
    let mut args = vec![
        OsString::from("-F"),
        message_file.clone().into_os_string(),
        OsString::from("--"),
    ];
    for path in &written {
        args.push(
            path.strip_prefix(&repo.root)
                .unwrap_or(path)
                .as_os_str()
                .to_owned(),
        );
    }
    let code = commit::run(&args);
    let _ = fs::remove_file(&message_file);
    if code != 0 {
        remove(&written);
    }
    Ok(code)
}

fn remove(files: &[PathBuf]) {
    for file in files {
        let _ = fs::remove_file(file);
    }
}
