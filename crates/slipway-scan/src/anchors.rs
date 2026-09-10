//! Скан разметки кода: `#[doc_anchor(id = "…")]`.
//!
//! Разметка живёт в рабочем коде, а не в реестре, поэтому её собирает
//! отдельный проход по исходникам продукта. Он же вычисляет диапазон строк:
//! сам атрибут этого сделать не может — на входе proc-макроса только
//! разобранный поток токенов без исходного форматирования.
//!
//! Именно поэтому MSRV остаётся 1.83: стабильные `Span::{start,end}`
//! появились в 1.88, но они нужны только внутри proc-макроса, а здесь
//! работает `proc-macro2` с включённой возможностью `span-locations`.
//!
//! Правила записи разметки общие с атрибутом (`slipway_core::anchor_rules`):
//! ошибка в разметке отвергается и при сборке продукта, и при скане, а не
//! превращается молча в отсутствующую или иначе отображаемую разметку.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use slipway_core::anchor_rules::{anchor_ident, check_anchor_id, check_anchor_mode};
use std::{fs, path::Path};
use syn::{spanned::Spanned, visit::Visit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedAnchor {
    /// Идентификатор в исходном виде: `direct-executor-plan-ir`.
    pub id: String,
    /// Он же как имя константы: `direct_executor_plan_ir`.
    pub ident: String,
    pub file: String,
    pub line_start: u32,
    pub line_end: u32,
    pub mode: AnchorMode,
    /// Дословный текст фрагмента, вырезанный по номерам строк.
    pub source_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorMode {
    /// Только ссылка на файл и строки при рендере.
    Ref,
    /// Полный текст вставляется в рендер.
    Embed,
    /// Вставляется сигнатура без тела.
    Snippet,
}

impl AnchorMode {
    fn parse(s: &str) -> Result<Self, String> {
        check_anchor_mode(s)?;
        Ok(match s {
            "embed" => Self::Embed,
            "snippet" => Self::Snippet,
            // Всё, кроме "ref", уже отвергнуто проверкой выше.
            _ => Self::Ref,
        })
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ref => "ref",
            Self::Embed => "embed",
            Self::Snippet => "snippet",
        }
    }
}

/// Обходит дерево исходников и собирает всю разметку.
pub fn scan_anchors(roots: &[&Path]) -> Result<Vec<ScannedAnchor>, super::ScanError> {
    let mut found = Vec::new();
    for root in roots {
        walk(root, &mut found)?;
    }
    found.sort_by(|a, b| a.id.cmp(&b.id));
    // Один идентификатор в двух местах дал бы две одноимённые константы
    // и ошибку в порождённом файле; здесь ошибка называет оба места.
    if let Some(pair) = found.windows(2).find(|w| w[0].id == w[1].id) {
        return Err(super::ScanError::Anchor {
            file: pair[1].file.clone(),
            line: pair[1].line_start as usize,
            detail: format!(
                "идентификатор {:?} уже занят в {}:{}",
                pair[1].id, pair[0].file, pair[0].line_start
            ),
        });
    }
    Ok(found)
}

fn walk(dir: &Path, out: &mut Vec<ScannedAnchor>) -> Result<(), super::ScanError> {
    // Нечитаемый каталог — ошибка, а не пустой результат: скан, не увидевший
    // предмета, не должен выглядеть успешным.
    let entries =
        fs::read_dir(dir).map_err(|e| super::ScanError::Io(format!("{}: {e}", dir.display())))?;
    for entry in entries {
        let path = entry
            .map_err(|e| super::ScanError::Io(e.to_string()))?
            .path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .is_some_and(|n| n == "target" || n == ".git");
            if !skip {
                walk(&path, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path)
                .map_err(|e| super::ScanError::Io(format!("{}: {e}", path.display())))?;
            // Полный разбор — только файлов, где разметка вообще упомянута:
            // на большом продукте это основная доля времени скана.
            if text.contains("doc_anchor") {
                out.extend(anchors_in_file(&text, &path.display().to_string())?);
            }
        }
    }
    Ok(())
}

/// Извлекает разметку из текста одного файла.
///
/// Обход через `syn::visit` видит все места, где компилятор допускает
/// атрибут: элементы любого уровня вложенности, в том числе внутри функций,
/// элементы `impl`, трейтов и `extern`-блоков.
pub fn anchors_in_file(text: &str, file: &str) -> Result<Vec<ScannedAnchor>, super::ScanError> {
    let parsed = syn::parse_file(text).map_err(|e| super::ScanError::Parse {
        file: file.to_owned(),
        detail: e.to_string(),
    })?;

    let lines: Vec<&str> = text.lines().collect();
    let mut collector = Collector {
        file,
        lines: &lines,
        found: Vec::new(),
        error: None,
    };
    collector.visit_file(&parsed);
    match collector.error {
        Some(e) => Err(e),
        None => Ok(collector.found),
    }
}

struct Collector<'a> {
    file: &'a str,
    lines: &'a [&'a str],
    found: Vec<ScannedAnchor>,
    error: Option<super::ScanError>,
}

impl Collector<'_> {
    fn consider<T: ToTokens>(&mut self, attrs: &[syn::Attribute], node: &T) {
        if self.error.is_some() {
            return;
        }
        let (id, mode) = match anchor_attr(attrs) {
            Ok(Some(found)) => found,
            Ok(None) => return,
            Err((line, detail)) => {
                self.error = Some(super::ScanError::Anchor {
                    file: self.file.to_owned(),
                    line,
                    detail,
                });
                return;
            }
        };
        let end = node.span().end().line.min(self.lines.len());
        let start = body_start_line(node.to_token_stream()).unwrap_or(end);
        self.found.push(ScannedAnchor {
            ident: anchor_ident(&id),
            id,
            file: self.file.to_owned(),
            line_start: start as u32,
            line_end: end as u32,
            mode,
            source_text: self
                .lines
                .get(start.saturating_sub(1)..end)
                .unwrap_or_default()
                .join("\n"),
        });
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, node: &'ast syn::Item) {
        self.consider(item_attrs(node), node);
        syn::visit::visit_item(self, node);
    }

    fn visit_impl_item(&mut self, node: &'ast syn::ImplItem) {
        self.consider(impl_item_attrs(node), node);
        syn::visit::visit_impl_item(self, node);
    }

    fn visit_trait_item(&mut self, node: &'ast syn::TraitItem) {
        self.consider(trait_item_attrs(node), node);
        syn::visit::visit_trait_item(self, node);
    }

    fn visit_foreign_item(&mut self, node: &'ast syn::ForeignItem) {
        self.consider(foreign_item_attrs(node), node);
        syn::visit::visit_foreign_item(self, node);
    }
}

/// Строка первого токена после внешних атрибутов. Doc-комментарии `syn`
/// хранит как `#[doc = …]`, поэтому они пропускаются тем же правилом.
fn body_start_line(tokens: TokenStream) -> Option<usize> {
    let mut tokens = tokens.into_iter();
    loop {
        match tokens.next()? {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                tokens.next()?;
            }
            other => return Some(other.span().start().line),
        }
    }
}

fn item_attrs(node: &syn::Item) -> &[syn::Attribute] {
    use syn::Item;
    match node {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(node: &syn::ImplItem) -> &[syn::Attribute] {
    use syn::ImplItem;
    match node {
        ImplItem::Const(i) => &i.attrs,
        ImplItem::Fn(i) => &i.attrs,
        ImplItem::Type(i) => &i.attrs,
        ImplItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(node: &syn::TraitItem) -> &[syn::Attribute] {
    use syn::TraitItem;
    match node {
        TraitItem::Const(i) => &i.attrs,
        TraitItem::Fn(i) => &i.attrs,
        TraitItem::Type(i) => &i.attrs,
        TraitItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn foreign_item_attrs(node: &syn::ForeignItem) -> &[syn::Attribute] {
    use syn::ForeignItem;
    match node {
        ForeignItem::Fn(i) => &i.attrs,
        ForeignItem::Static(i) => &i.attrs,
        ForeignItem::Type(i) => &i.attrs,
        ForeignItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

/// Разбирает `#[doc_anchor(id = "…", mode = "…")]`, в том числе записанный
/// с путём: `#[slipway::doc_anchor(...)]`. Ошибка возвращается со строкой
/// атрибута.
fn anchor_attr(attrs: &[syn::Attribute]) -> Result<Option<(String, AnchorMode)>, (usize, String)> {
    let mut marks = attrs.iter().filter(|a| {
        a.path()
            .segments
            .last()
            .is_some_and(|s| s.ident == "doc_anchor")
    });
    let Some(attr) = marks.next() else {
        return Ok(None);
    };
    let line = attr.span().start().line;
    if marks.next().is_some() {
        return Err((line, "у элемента больше одной разметки".to_owned()));
    }

    let mut id: Option<String> = None;
    let mut mode: Option<String> = None;
    attr.parse_nested_meta(|meta| {
        let key = meta
            .path
            .get_ident()
            .map(ToString::to_string)
            .unwrap_or_default();
        let slot = match key.as_str() {
            "id" => &mut id,
            "mode" => &mut mode,
            _ => return Err(meta.error(format!("неизвестный ключ {key:?}; допустимы id и mode"))),
        };
        let value: syn::LitStr = meta.value()?.parse()?;
        if slot.replace(value.value()).is_some() {
            return Err(meta.error(format!("ключ {key:?} указан дважды")));
        }
        Ok(())
    })
    .map_err(|e| (line, e.to_string()))?;

    let id = id.ok_or_else(|| (line, "у разметки нет id".to_owned()))?;
    check_anchor_id(&id).map_err(|e| (line, e))?;
    let mode = match mode {
        Some(m) => AnchorMode::parse(&m).map_err(|e| (line, e))?,
        None => AnchorMode::Ref,
    };
    Ok(Some((id, mode)))
}

/// Порождает модуль констант разметки: ссылка из решения — путь сюда.
pub fn emit_anchor_refs(anchors: &[ScannedAnchor]) -> String {
    use std::fmt::Write as _;
    let mut out = String::from("// ПОРОЖДЕНО slipway-scan. Не редактировать.\n\n");
    out.push_str("#[allow(non_upper_case_globals, unused_imports)]\npub mod anchor {\n    use slipway_core::AnchorId;\n");
    for a in anchors {
        let _ = writeln!(
            out,
            "    /// `{}` — {}:{}-{}",
            a.id, a.file, a.line_start, a.line_end
        );
        let _ = writeln!(
            out,
            "    pub const {}: AnchorId = AnchorId::__from_scan({:?});",
            a.ident, a.id
        );
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
#[doc_anchor(id = "plan-ir", mode = "snippet")]
pub struct PlanIR {
    pub steps: Vec<PlanStep>,
}

pub mod inner {
    #[doc_anchor(id = "nested-thing")]
    pub const LIMIT: u32 = 10;
}

impl PlanIR {
    #[doc_anchor(id = "plan-validate")]
    pub fn validate(&self) -> bool { !self.steps.is_empty() }
}
"#;

    #[test]
    fn finds_top_level_nested_and_impl_anchors() {
        let found = anchors_in_file(SRC, "plan.rs").unwrap();
        let ids: Vec<_> = found.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"plan-ir"), "разметка верхнего уровня");
        assert!(
            ids.contains(&"nested-thing"),
            "разметка во вложенном модуле"
        );
        assert!(
            ids.contains(&"plan-validate"),
            "разметка на методе внутри impl"
        );
    }

    #[test]
    fn identifier_is_snake_case_of_id() {
        let found = anchors_in_file(SRC, "plan.rs").unwrap();
        let a = found.iter().find(|a| a.id == "plan-ir").unwrap();
        assert_eq!(a.ident, "plan_ir");
        assert_eq!(a.mode, AnchorMode::Snippet);
    }

    #[test]
    fn source_text_excludes_the_attribute_line() {
        let found = anchors_in_file(SRC, "plan.rs").unwrap();
        let a = found.iter().find(|a| a.id == "plan-ir").unwrap();
        assert!(
            a.source_text.starts_with("pub struct PlanIR"),
            "{:?}",
            a.source_text
        );
        assert!(!a.source_text.contains("doc_anchor"));
    }

    #[test]
    fn emits_one_constant_per_anchor() {
        let found = anchors_in_file(SRC, "plan.rs").unwrap();
        let out = emit_anchor_refs(&found);
        assert!(out.contains("pub const plan_ir: AnchorId = AnchorId::__from_scan(\"plan-ir\")"));
        assert!(out.contains("pub const plan_validate: AnchorId"));
    }

    /// Атака E4: неизвестный режим молча становился `ref`.
    #[test]
    fn rejects_unknown_mode() {
        let src = "#[doc_anchor(id = \"hdr\", mode = \"snipet\")]\npub struct Hdr;\n";
        let err = anchors_in_file(src, "h.rs").expect_err("режим snipet принят");
        assert!(err.to_string().contains("snipet"), "{err}");
    }

    /// Атака E4: опечатка в ключе молча убирала разметку.
    #[test]
    fn rejects_unknown_key() {
        let src = "#[doc_anchor(idd = \"hdr\")]\npub struct Hdr;\n";
        let err = anchors_in_file(src, "h.rs").expect_err("ключ idd принят");
        assert!(err.to_string().contains("idd"), "{err}");
    }

    /// Идентификатор, не отображаемый в имя константы, давал нечитаемую
    /// ошибку в порождённом файле вместо ошибки на месте разметки.
    #[test]
    fn rejects_id_that_cannot_be_a_constant() {
        for bad in ["2pc-commit", "plan.ir", "type", "Plan-IR"] {
            let src = format!("#[doc_anchor(id = \"{bad}\")]\npub struct X;\n");
            assert!(anchors_in_file(&src, "x.rs").is_err(), "id {bad:?} принят");
        }
    }

    /// Фрагмент начинается с самого элемента, а не с doc-комментария
    /// или строки атрибута.
    #[test]
    fn source_text_starts_at_item_after_doc_comment() {
        let src =
            "/// Заголовок.\n#[doc_anchor(id = \"hdr\")]\npub struct Hdr {\n    pub v: u8,\n}\n";
        let a = &anchors_in_file(src, "h.rs").unwrap()[0];
        assert_eq!(a.line_start, 3, "{:?}", a.source_text);
        assert!(
            a.source_text.starts_with("pub struct Hdr"),
            "{:?}",
            a.source_text
        );
    }

    /// Разметка на методе трейта, константе в impl и элементе внутри
    /// функции видна скану: компилятор допускает атрибут во всех трёх местах.
    #[test]
    fn finds_anchors_on_all_attributable_items() {
        let src = r#"
pub trait Codec {
    #[doc_anchor(id = "codec-encode")]
    fn encode(&self) -> Vec<u8>;
}
impl Hdr {
    #[doc_anchor(id = "hdr-size")]
    pub const SIZE: usize = 72;
}
fn outer() {
    #[doc_anchor(id = "inner-fn")]
    fn inner() {}
}
"#;
        let ids: Vec<String> = anchors_in_file(src, "c.rs")
            .unwrap()
            .into_iter()
            .map(|a| a.id)
            .collect();
        for want in ["codec-encode", "hdr-size", "inner-fn"] {
            assert!(ids.iter().any(|i| i == want), "{want} не найден: {ids:?}");
        }
    }

    /// Один идентификатор в двух местах называет оба места, а не падает
    /// нечитаемо в порождённом файле.
    #[test]
    fn rejects_same_id_in_two_places() {
        let dir = std::env::temp_dir().join(format!("slipway-anchors-dup-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("a.rs"),
            "#[doc_anchor(id = \"hdr\")]\npub struct A;\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.rs"),
            "#[doc_anchor(id = \"hdr\")]\npub struct B;\n",
        )
        .unwrap();
        let err = scan_anchors(&[&dir]).expect_err("один id в двух местах принят");
        assert!(err.to_string().contains("уже занят"), "{err}");
    }

    /// Скан несуществующего каталога — ошибка, а не пустой успешный результат.
    #[test]
    fn missing_root_is_an_error_not_an_empty_scan() {
        let err =
            scan_anchors(&[Path::new("/nonexistent/slipway-src")]).expect_err("пустой скан принят");
        assert!(err.to_string().contains("slipway-src"), "{err}");
    }
}
