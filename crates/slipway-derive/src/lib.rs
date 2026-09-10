//! Атрибут-маркер разметки кода.
//!
//! **Возвращает аннотированный элемент без единого изменения.** Вся работа
//! выполняется отдельным сканом исходников (`slipway-scan`), который читает
//! текст файлов и порождает константы ссылок.
//!
//! Это принципиально. Прежняя схема регистрировала разметку через `inventory`,
//! то есть атрибут порождал дополнительный элемент верхнего уровня — из-за
//! чего разметку нельзя было ставить на методы внутри `impl` и невозможно
//! было проверить ссылки из `build.rs` (решение 1).
//! Маркер, ничего не порождающий, свободен от обоих ограничений.
//!
//! Сам атрибут делает одно — проверяет свои аргументы по тем же правилам,
//! что скан (`slipway_core::anchor_rules`). Иначе опечатка в ключе молча
//! убирала бы разметку, а неизвестный режим молча становился бы `ref`.
//! Разбор написан на `proc_macro` без `syn`: атрибут стоит в продуктовом
//! коде, и его сборка не должна тянуть парсер.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use slipway_core::anchor_rules::{check_anchor_id, check_anchor_mode};

/// Помечает фрагмент кода идентификатором, на который могут ссылаться решения.
///
/// ```
/// #[slipway_derive::doc_anchor(id = "plan-ir", mode = "snippet")]
/// pub struct PlanIr { pub steps: Vec<u32> }
/// ```
///
/// Режим отображения (`ref` | `embed` | `snippet`) принадлежит владельцу кода,
/// а не автору решения: он определяет, вставлять ли текст фрагмента в рендер.
///
/// Аргументы проверяются здесь же, на месте разметки. Опечатка в ключе не
/// убирает разметку молча (атака E4):
///
/// ```compile_fail
/// #[slipway_derive::doc_anchor(idd = "plan-ir")]
/// pub struct PlanIr;
/// ```
///
/// Неизвестный режим не становится молча `ref`:
///
/// ```compile_fail
/// #[slipway_derive::doc_anchor(id = "plan-ir", mode = "snipet")]
/// pub struct PlanIr;
/// ```
///
/// Идентификатор обязан отображаться в имя константы:
///
/// ```compile_fail
/// #[slipway_derive::doc_anchor(id = "type")]
/// pub struct PlanIr;
/// ```
#[proc_macro_attribute]
pub fn doc_anchor(args: TokenStream, item: TokenStream) -> TokenStream {
    match check_args(args) {
        Ok(()) => item,
        Err((span, message)) => {
            // Элемент возвращается рядом с ошибкой, чтобы за ней не шла
            // лавина вторичных ошибок «элемент не найден».
            let mut out = compile_error(span, &message);
            out.extend(item);
            out
        }
    }
}

/// Отказ: место ошибки и текст.
type Rejection = (Span, String);

/// Разбирает `id = "…"` и необязательный `mode = "…"`.
fn check_args(args: TokenStream) -> Result<(), Rejection> {
    let mut tokens = args.into_iter();
    let mut id: Option<(String, Span)> = None;
    let mut mode: Option<(String, Span)> = None;
    let mut last_key = Span::call_site();

    while let Some(tree) = tokens.next() {
        let key = match tree {
            TokenTree::Ident(key) => key,
            other => return Err((other.span(), "ожидается ключ id или mode".to_owned())),
        };
        last_key = key.span();
        match tokens.next() {
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => {}
            other => {
                return Err((
                    span_or(other, key.span()),
                    format!("после ключа {key} ожидается `=`"),
                ))
            }
        }
        let value = match tokens.next() {
            Some(TokenTree::Literal(lit)) => match string_value(&lit) {
                Some(text) => (text, lit.span()),
                None => {
                    return Err((
                        lit.span(),
                        "ожидается строка в двойных кавычках без экранирования".to_owned(),
                    ))
                }
            },
            other => {
                return Err((
                    span_or(other, key.span()),
                    format!("после `{key} =` ожидается строка"),
                ))
            }
        };
        let slot = match key.to_string().as_str() {
            "id" => &mut id,
            "mode" => &mut mode,
            other => {
                return Err((
                    key.span(),
                    format!("неизвестный ключ разметки {other:?}; допустимы id и mode"),
                ))
            }
        };
        if slot.replace(value).is_some() {
            return Err((key.span(), format!("ключ {key} указан дважды")));
        }
        match tokens.next() {
            None => break,
            Some(TokenTree::Punct(p)) if p.as_char() == ',' => {}
            Some(other) => {
                return Err((
                    other.span(),
                    "между аргументами ожидается запятая".to_owned(),
                ))
            }
        }
    }

    let (id, id_span) = id.ok_or_else(|| (last_key, "у разметки нет id".to_owned()))?;
    check_anchor_id(&id).map_err(|e| (id_span, e))?;
    if let Some((mode, mode_span)) = mode {
        check_anchor_mode(&mode).map_err(|e| (mode_span, e))?;
    }
    Ok(())
}

fn span_or(tree: Option<TokenTree>, fallback: Span) -> Span {
    tree.map_or(fallback, |t| t.span())
}

/// Значение обычного строкового литерала без экранирования.
fn string_value(lit: &Literal) -> Option<String> {
    let text = lit.to_string();
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains('\\')).then(|| inner.to_owned())
}

/// `::core::compile_error!("…");` с местом, указывающим на ошибочный аргумент.
fn compile_error(span: Span, message: &str) -> TokenStream {
    let mut text = Literal::string(message);
    text.set_span(span);
    let mut args = Group::new(
        Delimiter::Parenthesis,
        TokenStream::from(TokenTree::Literal(text)),
    );
    args.set_span(span);
    [
        punct(':', Spacing::Joint, span),
        punct(':', Spacing::Alone, span),
        TokenTree::Ident(Ident::new("core", span)),
        punct(':', Spacing::Joint, span),
        punct(':', Spacing::Alone, span),
        TokenTree::Ident(Ident::new("compile_error", span)),
        punct('!', Spacing::Alone, span),
        TokenTree::Group(args),
        punct(';', Spacing::Alone, span),
    ]
    .into_iter()
    .collect()
}

fn punct(ch: char, spacing: Spacing, span: Span) -> TokenTree {
    let mut p = Punct::new(ch, spacing);
    p.set_span(span);
    TokenTree::Punct(p)
}
