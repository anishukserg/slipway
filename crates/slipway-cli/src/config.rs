//! Настройка инструмента под продукт — файл `slipway.toml` в корне
//! репозитория (решение 20).
//!
//! Формат — то же плоское подмножество TOML, что у событий журнала: разбор
//! берётся из `slipway_journal::format`, своего парсера и новых зависимостей
//! нет (решения 14 и 15). Отсутствие файла означает соглашения Slipway:
//! умолчания повторяют прежнюю зашитую раскладку.
//!
//! Настройке подлежит форма, а не правило: корень реестра, набор типов темы и
//! её предел, ссылка на правила коммитов продукта, темы служебных коммитов
//! журнала и пол числа прошедших атакующих doctest. Трейлер основания,
//! неизменность события, доказательство, формат журнала и машинный контракт
//! вывода не настраиваются.
//!
//! Значение-команда — `gate_command` и `message_command` — делегирует проверку
//! проекту: Slipway запускает её и считает её вердикт своим.
//!
//! Настройка читается из того же дерева, что и проверяемое: калитка — из
//! проверяемого дерева, проверка сообщения — из индекса или из дерева коммита,
//! команды журнала — из рабочего дерева. Иначе правка настройки в рабочей копии
//! меняла бы вердикт для чужого дерева.
//!
//! Разбор строгий, как у журнала: неизвестный ключ, пустое значение,
//! нечисловой предел, тип не в верхнем регистре и неизвестная подстановка —
//! отказ с именем ключа. Негодная настройка — ошибка запуска команды, а не
//! отказ проверки: инструмент не работает по испорченной настройке.

use crate::{git, layout};
use slipway_journal::format;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// Настройка продукта; умолчания — соглашения Slipway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Корень реестра: из него выводятся таксономия, работы, срезы, журнал и
    /// манифест крейта документов.
    pub doc: String,
    /// Типы темы — закрытый набор.
    pub commit_types: Vec<String>,
    /// Предел длины темы в знаках.
    pub subject_limit: usize,
    /// Правила коммитов продукта — то, что названо в скобках в тексте отказа.
    pub commit_rules: String,
    /// Тема коммита события «работа начата».
    pub subject_started: String,
    /// Тема коммита события «работа приземлена».
    pub subject_landed: String,
    /// Тема коммита события «работа брошена».
    pub subject_abandoned: String,
    /// Тема коммита события «срез закрыт».
    pub subject_slice_closed: String,
    /// Тема коммита восстановления журнала из истории.
    pub subject_imported: String,
    /// Пол числа прошедших атакующих doctest: ниже него калитка отказывает.
    pub doctest_floor: usize,
    /// Команда проекта, выполняемая шагом калитки; `None` — шага нет.
    pub gate_command: Option<Vec<String>>,
    /// Команда проекта, проверяющая форму темы; `None` — форму проверяет сам
    /// Slipway.
    pub message_command: Option<Vec<String>>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            doc: "doc".to_owned(),
            commit_types: [
                "FEAT", "FIX", "REFACTOR", "TEST", "DOCS", "ADR", "PLAN", "CHORE",
            ]
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect(),
            subject_limit: 72,
            commit_rules: "commit rules".to_owned(),
            subject_started: "[PLAN]({scope}): work {id} started".to_owned(),
            subject_landed: "[PLAN]({scope}): work {id} landed".to_owned(),
            subject_abandoned: "[PLAN]({scope}): work {id} abandoned".to_owned(),
            subject_slice_closed: "[PLAN]({scope}): slice {id} closed".to_owned(),
            subject_imported: "[PLAN]({scope}): journal restored from history".to_owned(),
            doctest_floor: 36,
            gate_command: None,
            message_command: None,
        }
    }
}

impl Config {
    /// Таксономия реестра: значения оси подсистем — области темы коммита.
    pub fn taxonomy(&self) -> String {
        format!("{}/taxonomy.rs", self.doc)
    }

    /// Каталог единиц работы: основание коммита обязано лежать здесь.
    pub fn work_dir(&self) -> String {
        format!("{}/work", self.doc)
    }

    /// Каталог срезов: основание коммита закрытия среза (решение 15).
    pub fn slice_dir(&self) -> String {
        format!("{}/slice", self.doc)
    }

    /// Каталог журнала работы (решение 15).
    pub fn journal_dir(&self) -> String {
        format!("{}/journal", self.doc)
    }

    /// Манифест крейта документов: его сборка сворачивает журнал.
    pub fn doc_manifest(&self) -> String {
        format!("{}/Cargo.toml", self.doc)
    }

    /// Настройка из каталога дерева — рабочего или выгруженного. Нет файла —
    /// умолчания.
    pub fn read_dir(dir: &Path) -> Result<Config, String> {
        match fs::read_to_string(dir.join(layout::CONFIG)) {
            Ok(text) => Config::parse(&text).map_err(named),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Config::default()),
            Err(error) => Err(format!("{} is not readable: {error}", layout::CONFIG)),
        }
    }

    /// Настройка из дерева git репозитория в `dir`: пустая строка — индекс,
    /// иначе ревизия. Нет файла в дереве — умолчания.
    pub fn read_tree(dir: &Path, tree: &str) -> Result<Config, String> {
        let spec = format!("{tree}:{}", layout::CONFIG);
        match git::read(dir, &["show", &spec]) {
            Some(text) => Config::parse(&text).map_err(named),
            None => Ok(Config::default()),
        }
    }

    /// Разбор текста настройки; ошибка называет ключ.
    fn parse(text: &str) -> Result<Config, String> {
        let record = format::parse(text).map_err(|error| {
            format!(
                "line {}: the flat TOML subset takes blank lines, `# comment` and `key = \"value\"`",
                error.line
            )
        })?;
        let mut config = Config::default();
        for (key, value) in &record.fields {
            let value = value.trim();
            if value.is_empty() {
                return Err(format!("key `{key}` has an empty value"));
            }
            match key.as_str() {
                "doc" => config.doc = directory(key, value)?,
                "commit_types" => config.commit_types = types(key, value)?,
                "subject_limit" => {
                    config.subject_limit = value.parse().map_err(|_| {
                        format!("key `{key}` takes a number of characters, not `{value}`")
                    })?;
                }
                "commit_rules" => config.commit_rules = value.to_owned(),
                "subject_started" => config.subject_started = template(key, value)?,
                "subject_landed" => config.subject_landed = template(key, value)?,
                "subject_abandoned" => config.subject_abandoned = template(key, value)?,
                "subject_slice_closed" => config.subject_slice_closed = template(key, value)?,
                "subject_imported" => config.subject_imported = template(key, value)?,
                "doctest_floor" => {
                    config.doctest_floor = value.parse().map_err(|_| {
                        format!("key `{key}` takes a number of passed doctests, not `{value}`")
                    })?;
                }
                "gate_command" => config.gate_command = Some(command(key, value)?),
                "message_command" => config.message_command = Some(command(key, value)?),
                _ => return Err(format!("unknown key `{key}`")),
            }
        }
        Ok(config)
    }
}

/// Тема служебного коммита из шаблона: подстановки `{scope}` и `{id}`.
pub fn fill(template: &str, scope: &str, id: &str) -> String {
    template.replace("{scope}", scope).replace("{id}", id)
}

/// Имя файла настройки в тексте ошибки: ошибка называет файл, а не только ключ.
fn named(problem: String) -> String {
    format!("{}: {problem}", layout::CONFIG)
}

/// Путь каталога от корня репозитория, без хвостовых `/`.
fn directory(key: &str, value: &str) -> Result<String, String> {
    let path = value.trim_end_matches('/');
    if path.is_empty() {
        return Err(format!("key `{key}` has an empty value"));
    }
    Ok(path.to_owned())
}

/// Набор типов темы через пробел; тип пишется прописными латинскими буквами —
/// иначе тема с ним не совпала бы ни с одним типом набора.
fn types(key: &str, value: &str) -> Result<Vec<String>, String> {
    let types: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
    if types.is_empty() {
        return Err(format!("key `{key}` names no subject type"));
    }
    if let Some(kind) = types
        .iter()
        .find(|kind| !kind.bytes().all(|b| b.is_ascii_uppercase()))
    {
        return Err(format!(
            "key `{key}`: type `{kind}` is not in uppercase Latin letters"
        ));
    }
    Ok(types)
}

/// Команда проекта: разбивается по пробелам, первое слово — программа,
/// остальные — её аргументы.
///
/// Оболочка не запускается: `&&`, `|`, перенаправления и кавычки в значении не
/// работают — они стали бы обычными аргументами программы. Составную проверку
/// проект прячет в свой скрипт и называет здесь его.
fn command(key: &str, value: &str) -> Result<Vec<String>, String> {
    let words: Vec<String> = value.split_whitespace().map(str::to_owned).collect();
    if words.is_empty() {
        return Err(format!("key `{key}` has an empty value"));
    }
    Ok(words)
}

/// Шаблон темы: подстановки только `{scope}` и `{id}`. Неизвестная осталась бы
/// в теме буквально, и служебный коммит ушёл бы в историю с `{work}` в теме.
fn template(key: &str, value: &str) -> Result<String, String> {
    let mut rest = value;
    while let Some(at) = rest.find('{') {
        let (name, tail) = rest[at + 1..]
            .split_once('}')
            .ok_or_else(|| format!("key `{key}`: `{{` without `}}`"))?;
        if name != "scope" && name != "id" {
            return Err(format!(
                "key `{key}`: unknown substitution `{{{name}}}` — only `{{scope}}` and `{{id}}`"
            ));
        }
        rest = tail;
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_conventions_of_slipway() {
        let config = Config::default();
        assert_eq!(config.taxonomy(), "doc/taxonomy.rs");
        assert_eq!(config.work_dir(), "doc/work");
        assert_eq!(config.slice_dir(), "doc/slice");
        assert_eq!(config.journal_dir(), "doc/journal");
        assert_eq!(config.doc_manifest(), "doc/Cargo.toml");
        assert_eq!(config.subject_limit, 72);
        assert_eq!(config.commit_rules, "commit rules");
        assert_eq!(config.doctest_floor, 36);
        assert_eq!(config.gate_command, None);
        assert_eq!(config.message_command, None);
        assert!(config.commit_types.contains(&"CHORE".to_owned()));
        assert_eq!(
            fill(&config.subject_started, "cli", "w0033"),
            "[PLAN](cli): work w0033 started"
        );
        assert_eq!(
            fill(&config.subject_imported, "cli", "w0033"),
            "[PLAN](cli): journal restored from history"
        );
    }

    #[test]
    fn values_replace_the_conventions() {
        let config = Config::parse(concat!(
            "# раскладка продукта\n",
            "doc = \"docs/registry/\"\n",
            "commit_types = \"FEAT CHANGE\"\n",
            "subject_limit = \"90\"\n",
            "commit_rules = \"docs/COMMITS.md\"\n",
            "subject_started = \"[CHANGE]({scope}): started {id}\"\n",
            "doctest_floor = \"5\"\n",
            "gate_command = \"make  check --all\"\n",
            "message_command = \"scripts/commit-msg\"\n",
        ))
        .expect("настройка разобрана");
        assert_eq!(config.doctest_floor, 5);
        // Команда разбивается по пробелам: первое слово — программа, остальные —
        // аргументы; оболочки нет.
        assert_eq!(
            config.gate_command.as_deref(),
            Some(["make".to_owned(), "check".to_owned(), "--all".to_owned()].as_slice())
        );
        assert_eq!(
            config.message_command.as_deref(),
            Some(["scripts/commit-msg".to_owned()].as_slice())
        );
        assert_eq!(config.work_dir(), "docs/registry/work");
        assert_eq!(config.journal_dir(), "docs/registry/journal");
        assert_eq!(config.commit_types, ["FEAT", "CHANGE"]);
        assert_eq!(config.subject_limit, 90);
        assert_eq!(config.commit_rules, "docs/COMMITS.md");
        assert_eq!(
            fill(&config.subject_started, "cli", "w0001"),
            "[CHANGE](cli): started w0001"
        );
        // Ключи, которых нет в файле, остаются умолчаниями.
        assert_eq!(config.subject_landed, Config::default().subject_landed);
    }

    #[test]
    fn each_refusal_names_its_key() {
        for (text, expected) in [
            ("docs = \"x\"\n", "unknown key `docs`"),
            (
                "subject_limit = \"много\"\n",
                "key `subject_limit` takes a number",
            ),
            (
                "commit_types = \"\"\n",
                "key `commit_types` has an empty value",
            ),
            (
                "commit_types = \"feat\"\n",
                "type `feat` is not in uppercase",
            ),
            ("doc = \"/\"\n", "key `doc` has an empty value"),
            (
                "doctest_floor = \"половина\"\n",
                "key `doctest_floor` takes a number of passed doctests",
            ),
            (
                "gate_command = \"  \"\n",
                "key `gate_command` has an empty value",
            ),
            (
                "message_command = \"\"\n",
                "key `message_command` has an empty value",
            ),
            (
                "subject_landed = \"[PLAN]({scope}): {work} landed\"\n",
                "unknown substitution `{work}`",
            ),
            (
                "subject_landed = \"[PLAN]({scope): landed\"\n",
                "`{` without `}`",
            ),
            ("[table]\n", "line 1: the flat TOML subset"),
        ] {
            let problem = Config::parse(text).expect_err(text);
            assert!(problem.contains(expected), "{text}: {problem}");
        }
    }

    #[test]
    fn a_missing_file_means_the_conventions() {
        // Каталог исходников инструмента: настройки в нём нет и быть не может.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(
            Config::read_dir(&dir).expect("нет файла"),
            Config::default()
        );
    }
}
