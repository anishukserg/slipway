//! Скан реестра решений и порождение модуля констант.
//!
//! Ключевой механизм методологии (решения 1 и 4):
//! ссылка между документами — путь к константе, а не число. Константы
//! порождаются здесь, из текста реестра, поэтому удаление документа делает
//! ссылку на него не резолвящейся — обычной ошибкой компилятора.
//!
//! Работает с ТЕКСТОМ файлов, а не со скомпилированным крейтом: цикла
//! «реестр порождает константы, которыми пользуется сам» не возникает.
//! Цена текстового разбора — второй источник истины: скан классифицирует
//! запись по токенам, компилятор вычисляет её значение. Поэтому порождённый
//! код сверяет одно с другим константными утверждениями, и расхождение
//! становится ошибкой компиляции, а не тихо неверным модулем констант.

// Нестабильные возможности запрещены и в doctest: lints манифеста на них не
// распространяются, а атаки выполняются под RUSTC_BOOTSTRAP (решение 12).
#![doc(test(attr(forbid(unstable_features))))]

pub mod anchors;
pub mod journal;
pub mod plan;
pub use anchors::{scan_anchors, AnchorMode, ScannedAnchor};

use std::{fmt::Write as _, fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDecision {
    pub id: u32,
    pub module: String,
    pub status: Status,
    /// Абсолютный путь к файлу. Нужен потому, что порождённый модуль лежит
    /// в OUT_DIR, а `#[path]` разрешается относительно него.
    pub file: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Draft,
    Active,
    Deprecated,
    SupersededBy,
}

/// Конструкторы, допустимые только в порождённом коде. Их появление в тексте
/// реестра — обход невыразимости, а не опечатка.
const BYPASS_IDENTS: [&str; 2] = ["__from_scan", "__new_unchecked"];

#[derive(Debug)]
pub enum ScanError {
    Io(String),
    Parse {
        file: String,
        detail: String,
    },
    /// Имя файла не каноническое для идентификатора внутри него.
    IdMismatch {
        file: String,
        declared: u32,
        expected: String,
    },
    /// В тексте реестра вызван конструктор, предназначенный скану.
    Bypass {
        file: String,
        ident: String,
    },
    /// Разметка кода записана с ошибкой.
    Anchor {
        file: String,
        line: usize,
        detail: String,
    },
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "ввод-вывод: {m}"),
            Self::Parse { file, detail } => write!(f, "{file}: не разобрано: {detail}"),
            Self::IdMismatch { file, declared, expected } => {
                write!(f, "{file}: идентификатор {declared} требует имени файла {expected}.rs")
            }
            Self::Bypass { file, ident } => write!(
                f,
                "{file}: `{ident}` допустим только в порождённом коде; ссылка в реестре — путь к константе"
            ),
            Self::Anchor { file, line, detail } => write!(f, "{file}:{line}: разметка кода: {detail}"),
        }
    }
}

/// Сканирует каталог реестра спецификаций (`rfc!`).
pub fn scan_specs(dir: &Path) -> Result<Vec<ScannedDecision>, ScanError> {
    scan_dir(dir, "rfc", 'r')
}

/// Сканирует каталог реестра решений (`adr!`).
pub fn scan_decisions(dir: &Path) -> Result<Vec<ScannedDecision>, ScanError> {
    scan_dir(dir, "adr", 'a')
}

fn scan_dir(dir: &Path, macro_name: &str, prefix: char) -> Result<Vec<ScannedDecision>, ScanError> {
    let mut found = Vec::new();
    let entries =
        fs::read_dir(dir).map_err(|e| ScanError::Io(format!("{}: {e}", dir.display())))?;

    for entry in entries {
        let path = entry.map_err(|e| ScanError::Io(e.to_string()))?.path();
        let is_rs = path.extension().is_some_and(|e| e == "rs");
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_owned();
        if !is_rs || stem == "mod" {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| ScanError::Io(e.to_string()))?;
        let mut decision = parse_entry(&text, &stem, macro_name, prefix)?;
        decision.file = path.display().to_string();
        found.push(decision);
    }
    found.sort_by_key(|d| d.id);
    Ok(found)
}

/// Разбирает один файл реестра решений.
pub fn parse_decision(text: &str, module: &str) -> Result<ScannedDecision, ScanError> {
    parse_entry(text, module, "adr", 'a')
}

/// Разбирает один файл реестра: находит вызов макроса регистрации и
/// извлекает идентификатор и статус из его токенов.
pub fn parse_entry(
    text: &str,
    module: &str,
    macro_name: &str,
    prefix: char,
) -> Result<ScannedDecision, ScanError> {
    let file = syn::parse_file(text).map_err(|e| ScanError::Parse {
        file: module.into(),
        detail: e.to_string(),
    })?;

    let mac = file
        .items
        .iter()
        .find_map(|item| match item {
            // Путь может быть как `adr!`, так и `slipway_knowledge::adr!` —
            // значим только последний сегмент.
            syn::Item::Macro(m)
                if m.mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == macro_name) =>
            {
                Some(&m.mac)
            }
            _ => None,
        })
        .ok_or_else(|| ScanError::Parse {
            file: module.into(),
            detail: format!("вызов {macro_name}! не найден"),
        })?;

    let tokens: Vec<_> = mac.tokens.clone().into_iter().collect();
    let id = tokens
        .iter()
        .find_map(|t| match t {
            proc_macro2::TokenTree::Literal(l) => l.to_string().parse::<u32>().ok(),
            _ => None,
        })
        .ok_or_else(|| ScanError::Parse {
            file: module.into(),
            detail: "идентификатор решения не найден".into(),
        })?;

    // Каноническое имя файла: у идентификатора ровно одно допустимое имя,
    // поэтому второй файл с тем же идентификатором в каталоге невыразим —
    // ни через ведущие нули, ни через имя без номера.
    let expected = format!("{prefix}{id:04}");
    if module != expected {
        return Err(ScanError::IdMismatch {
            file: module.into(),
            declared: id,
            expected,
        });
    }

    if let Some(ident) = find_bypass(text) {
        return Err(ScanError::Bypass {
            file: module.into(),
            ident,
        });
    }

    let status = extract_status(&tokens);
    Ok(ScannedDecision {
        id,
        module: module.to_owned(),
        status,
        file: String::new(),
    })
}

/// Ищет конструкторы скана во всём тексте файла, включая вложенные группы.
/// Проза в строковых литералах и комментарии токенами-идентификаторами не
/// являются, поэтому упоминание в тексте решения ложной тревоги не даёт.
fn find_bypass(text: &str) -> Option<String> {
    let stream: proc_macro2::TokenStream = text.parse().ok()?;
    find_bypass_in(stream)
}

fn find_bypass_in(stream: proc_macro2::TokenStream) -> Option<String> {
    stream.into_iter().find_map(|t| match t {
        proc_macro2::TokenTree::Ident(i) => {
            let name = i.to_string();
            BYPASS_IDENTS.contains(&name.as_str()).then_some(name)
        }
        proc_macro2::TokenTree::Group(g) => find_bypass_in(g.stream()),
        _ => None,
    })
}

/// Классификация по токенам верхнего уровня. Может ошибиться на необычной
/// записи статуса — поэтому порождённый код сверяет её с вычисленным
/// значением (`emit_refs`).
fn extract_status(tokens: &[proc_macro2::TokenTree]) -> Status {
    let idents: Vec<String> = tokens
        .iter()
        .filter_map(|t| match t {
            proc_macro2::TokenTree::Ident(i) => Some(i.to_string()),
            _ => None,
        })
        .collect();

    let after_status = idents
        .iter()
        .position(|i| i == "status")
        .and_then(|p| idents.get(p + 1..));

    match after_status.and_then(|rest| {
        rest.iter().find(|i| {
            matches!(
                i.as_str(),
                "Draft" | "Active" | "Deprecated" | "SupersededBy"
            )
        })
    }) {
        Some(s) if s == "Active" => Status::Active,
        Some(s) if s == "Deprecated" => Status::Deprecated,
        Some(s) if s == "SupersededBy" => Status::SupersededBy,
        _ => Status::Draft,
    }
}

/// Порождает модуль констант и агрегированный список.
///
/// Константы разложены по подмодулям так, чтобы ссылка не того класса была
/// невыразима: `Retirement` принимает только `superseded::*`.
pub fn emit_refs(decisions: &[ScannedDecision]) -> String {
    let mut out = String::from("// ПОРОЖДЕНО slipway-scan. Не редактировать.\n\n");

    // Каждый публичный элемент документирован: порождённый код собирается под
    // строгим профилем lints продукта (решение 16).
    for d in decisions {
        let _ = writeln!(
            out,
            "/// Решение {}.\n#[path = {:?}]\npub mod {};",
            d.id, d.file, d.module
        );
    }

    out.push_str("\n/// Ссылки на решения: путь к константе вместо номера.\n#[allow(non_upper_case_globals, unused_imports)]\npub mod adr {\n    use slipway_core::AdrRef;\n");
    for d in decisions {
        let _ = writeln!(
            out,
            "    /// Ссылка на решение {id}.\n    pub const {m}: AdrRef = AdrRef::__from_scan({id});",
            m = d.module,
            id = d.id
        );
    }
    out.push_str("}\n\n");

    out.push_str("/// Только замещённые решения: уборка живого кода невыразима.\n");
    out.push_str("#[allow(non_upper_case_globals, unused_imports)]\npub mod superseded {\n    use slipway_core::SupersededRef;\n");
    for d in decisions
        .iter()
        .filter(|d| d.status == Status::SupersededBy)
    {
        let _ = writeln!(
            out,
            "    /// Ссылка на замещённое решение {id}.\n    pub const {m}: SupersededRef = SupersededRef::__from_scan({id});",
            m = d.module,
            id = d.id
        );
    }
    out.push_str("}\n\n");

    out.push_str("// Сверка скана с компилятором: скан классифицирует по токенам, компилятор\n");
    out.push_str("// вычисляет значение. Расхождение — ошибка вычисления константы (E0080).\n");
    for d in decisions {
        let _ = writeln!(
            out,
            "const _: () = assert!({m}::DECISION.id == {id}, \"slipway-scan: у {m} идентификатор разошёлся со сканом\");",
            m = d.module,
            id = d.id
        );
        let (neg, what) = if d.status == Status::SupersededBy {
            ("", "замещено")
        } else {
            ("!", "не замещено")
        };
        let _ = writeln!(
            out,
            "const _: () = assert!({neg}matches!({m}::DECISION.status, ::slipway_knowledge::DocStatus::SupersededBy(_)), \"slipway-scan: по тексту {m} {what}, компилятор вычислил иное\");",
            m = d.module
        );
    }

    out.push_str("\n/// Все решения реестра.\npub static ALL: &[&slipway_knowledge::ArchitectureDecision] = &[\n");
    for d in decisions {
        let _ = writeln!(out, "    &{}::DECISION,", d.module);
    }
    out.push_str("];\n");
    out
}

/// Порождает модуль констант для реестра спецификаций.
pub fn emit_spec_refs(specs: &[ScannedDecision]) -> String {
    let mut out = String::from("// ПОРОЖДЕНО slipway-scan. Не редактировать.\n\n");
    for s in specs {
        let _ = writeln!(
            out,
            "/// Спецификация {}.\n#[path = {:?}]\npub mod {};",
            s.id, s.file, s.module
        );
    }
    out.push_str("\n/// Ссылки на спецификации: путь к константе вместо номера.\n#[allow(non_upper_case_globals, unused_imports)]\npub mod rfc {\n    use slipway_core::RfcRef;\n");
    for s in specs {
        let _ = writeln!(
            out,
            "    /// Ссылка на спецификацию {id}.\n    pub const {m}: RfcRef = RfcRef::__from_scan({id});",
            m = s.module,
            id = s.id
        );
    }
    out.push_str("}\n\n");
    for s in specs {
        let _ = writeln!(
            out,
            "const _: () = assert!({m}::SPEC.id == {id}, \"slipway-scan: у {m} идентификатор разошёлся со сканом\");",
            m = s.module,
            id = s.id
        );
    }
    out.push_str("\n/// Все спецификации реестра.\npub static ALL_SPECS: &[&slipway_knowledge::DomainSpecification] = &[\n");
    for s in specs {
        let _ = writeln!(out, "    &{}::SPEC,", s.module);
    }
    out.push_str("];\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
        use slipway_knowledge::DocStatus;
        slipway_knowledge::adr!(2,
            title: "Прямая передача плана",
            status: DocStatus::Active,
        );
    "#;

    #[test]
    fn extracts_id_and_status() {
        let d = parse_decision(SAMPLE, "a0002").unwrap();
        assert_eq!(d.id, 2);
        assert_eq!(d.status, Status::Active);
    }

    #[test]
    fn rejects_id_not_matching_filename() {
        let err = parse_decision(SAMPLE, "a0009").unwrap_err();
        assert!(matches!(err, ScanError::IdMismatch { declared: 2, .. }));
    }

    #[test]
    fn superseded_module_holds_only_superseded() {
        let ds = vec![
            ScannedDecision {
                id: 1,
                module: "a0001".into(),
                status: Status::SupersededBy,
                file: "/x/a0001.rs".into(),
            },
            ScannedDecision {
                id: 2,
                module: "a0002".into(),
                status: Status::Active,
                file: "/x/a0002.rs".into(),
            },
        ];
        let out = emit_refs(&ds);
        assert!(out.contains("pub const a0001: SupersededRef"));
        assert!(!out.contains("pub const a0002: SupersededRef"));
        assert!(out.contains("pub const a0002: AdrRef"));
    }

    #[test]
    fn emitted_code_cross_checks_scan_against_compiler() {
        let ds = vec![
            ScannedDecision {
                id: 1,
                module: "a0001".into(),
                status: Status::SupersededBy,
                file: "/x/a0001.rs".into(),
            },
            ScannedDecision {
                id: 2,
                module: "a0002".into(),
                status: Status::Active,
                file: "/x/a0002.rs".into(),
            },
        ];
        let out = emit_refs(&ds);
        assert!(out.contains("assert!(a0001::DECISION.id == 1"));
        assert!(out.contains("assert!(matches!(a0001::DECISION.status"));
        assert!(out.contains("assert!(!matches!(a0002::DECISION.status"));
    }

    /// Каталог реестра во временной папке: имя файла → содержимое.
    fn registry(tag: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("slipway-scan-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for (name, text) in files {
            fs::write(dir.join(name), text).unwrap();
        }
        dir
    }

    const ACTIVE_2: &str = "slipway_knowledge::adr!(2, status: DocStatus::Active,);";
    const ACTIVE_3: &str = "slipway_knowledge::adr!(3, status: DocStatus::Active,);";
    const ACTIVE_7: &str = "slipway_knowledge::adr!(7, status: DocStatus::Active,);";

    /// Позитивный контроль для атак ниже: корректный каталог принимается.
    #[test]
    fn accepts_canonical_registry() {
        let dir = registry("ok", &[("a0002.rs", ACTIVE_2), ("a0003.rs", ACTIVE_3)]);
        assert_eq!(scan_decisions(&dir).unwrap().len(), 2);
    }

    /// Атака E2: второй файл с тем же идентификатором через ведущие нули.
    #[test]
    fn rejects_duplicate_id_via_leading_zeros() {
        let dir = registry("dup", &[("a0002.rs", ACTIVE_2), ("a00002.rs", ACTIVE_2)]);
        let err = scan_decisions(&dir).expect_err("два решения с id 2 приняты");
        assert!(err.to_string().contains("a0002.rs"), "{err}");
    }

    /// Атака E2: имя файла без номера не сверялось вовсе.
    #[test]
    fn rejects_file_name_without_number() {
        let dir = registry("loose", &[("a.rs", ACTIVE_7)]);
        let err = scan_decisions(&dir).expect_err("файл a.rs с id 7 принят");
        assert!(err.to_string().contains("a0007.rs"), "{err}");
    }

    /// Атака E1 в обход закрытого поля: конструктор скана, вызванный в реестре руками.
    #[test]
    fn rejects_scan_constructor_in_registry_text() {
        let text = "slipway_knowledge::adr!(1, status: DocStatus::SupersededBy(slipway_core::AdrRef::__from_scan(2)),);";
        let err = parse_decision(text, "a0001").expect_err("ручной __from_scan принят");
        assert!(err.to_string().contains("__from_scan"), "{err}");
    }

    /// Контроль к атаке выше: упоминание конструктора в прозе решения —
    /// строковый литерал, а не вызов, и отвергаться не должно.
    #[test]
    fn mention_in_prose_is_not_a_bypass() {
        let text = "slipway_knowledge::adr!(1, status: DocStatus::Active, context: r#\"вызов __from_scan руками запрещён\"#,);";
        assert!(parse_decision(text, "a0001").is_ok());
    }
}
