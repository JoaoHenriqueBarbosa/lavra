//! Runtime dos executáveis da Lavra: escrita e leitura, texto, lista, registro,
//! "talvez", falhas e erros de execução. Compilado como biblioteca estática pelo
//! `build.rs` e embutido no `lavrac`, que o linka em cada executável.
//!
//! Formatação e mensagens vêm dos mesmos arquivos que o interpretador usa, para que o
//! nativo escreva exatamente o mesmo, byte a byte.
//!
//! Representação: escalares vão em registradores; texto, lista, registro e "talvez" são
//! ponteiros para o monte. Dentro de listas e registros, cada valor ocupa uma casa de 8
//! bytes (`u64`). Um descritor de tipo (`Desc`), gerado pelo compilador como dado
//! estático, diz como escrever, comparar, copiar e liberar cada valor.

#![allow(clippy::missing_safety_doc, static_mut_refs, dead_code)]

#[path = "../src/span.rs"]
mod span;
#[path = "../src/codes.rs"]
mod codes;
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/textfmt.rs"]
mod textfmt;

use codes::*;
use diagnostics::{Diagnostic, Style};
use span::{SourceFile, Span};
use std::io::{BufRead, BufWriter, IsTerminal, Stdout, Write};

const KIND_INT: u32 = 0;
const KIND_REAL: u32 = 1;
const KIND_BOOL: u32 = 2;
const KIND_CHAR: u32 = 3;
const KIND_TEXT: u32 = 4;
const KIND_LIST: u32 = 5;
const KIND_RECORD: u32 = 6;
const KIND_MAYBE: u32 = 7;
const KIND_ENUM: u32 = 8;
/// Conjunto: os valores em ordem, sem repetição (`elem` é o tipo deles).
const KIND_SET: u32 = 9;
/// Mapa: chave e valor lado a lado, em ordem de chave (`elem` é a chave; `fields` aponta
/// para o descritor do valor).
const KIND_MAP: u32 = 10;

/// Descritor de tipo. O layout é o mesmo que o gerador de código escreve.
#[repr(C)]
pub struct Desc {
    kind: u32,
    bits: u32,
    signed: u32,
    feminine: u32,
    elem: *const Desc,
    nfields: u64,
    fields: *const FieldDesc,
    name: *const u8,
    name_len: u64,
}

#[repr(C)]
pub struct FieldDesc {
    desc: *const Desc,
    name: *const u8,
    name_len: u64,
    article: *const u8,
    article_len: u64,
}

/// Um caso de enumeração (o descritor da enumeração aponta para a lista deles).
#[repr(C)]
pub struct VariantDesc {
    name: *const u8,
    name_len: u64,
    feminine: u64,
    nfields: u64,
    fields: *const FieldDesc,
}

unsafe fn variant<'a>(d: &Desc, slot: u64) -> (&'a VariantDesc, *mut u64) {
    unsafe {
        let p = slot as *mut u64;
        let tag = *p as usize;
        (&*(d.fields as *const VariantDesc).add(tag), p)
    }
}

/// "o raio 2, o lado 3 e a altura 4".
unsafe fn fields_text(fields: *const FieldDesc, n: u64, base: *const u64) -> String {
    unsafe {
        let mut parts = Vec::new();
        for i in 0..n as usize {
            let f = &*fields.add(i);
            let mut part = format!("{} {} ", str_of(f.article, f.article_len), str_of(f.name, f.name_len));
            format_into(&mut part, f.desc, *base.add(i));
            parts.push(part);
        }
        textfmt::join_e(&parts)
    }
}

/// Falha declarada ("Desisto com ..."): sobe pelas funções até quem a trata.
pub struct Failure {
    message: String,
    lo: u32,
    hi: u32,
}

#[allow(improper_ctypes)]
unsafe extern "C" {
    static lavra_src: u8;
    static lavra_src_len: u64;
    static lavra_path: u8;
    static lavra_path_len: u64;
    static lavra_modules: u8;
    static lavra_modules_len: u64;
    fn lavra_init();
    fn lavra_main() -> *mut Failure;
}

static mut OUT: Option<BufWriter<Stdout>> = None;
static mut INPUT: Option<std::io::StdinLock<'static>> = None;
static mut ECHO: bool = false;
static mut DEPTH: u64 = 0;

unsafe fn out() -> &'static mut BufWriter<Stdout> {
    unsafe { OUT.get_or_insert_with(|| BufWriter::new(std::io::stdout())) }
}

unsafe fn flush() {
    unsafe {
        let _ = out().flush();
    }
}

unsafe fn str_of<'a>(ptr: *const u8, len: u64) -> &'a str {
    unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(ptr, len as usize)) }
}

/// O arquivo trazido que contém a posição: a base, o caminho e o texto. O executável leva os
/// arquivos num bloco só: de cada um, a base, o tamanho do caminho e o do texto (três
/// inteiros de 32 bits), e os dois.
unsafe fn module_of<'a>(pos: u32) -> Option<(u32, &'a str, &'a str)> {
    unsafe {
        let blob = std::slice::from_raw_parts(&lavra_modules as *const u8, lavra_modules_len as usize);
        let mut at = 0;
        while at + 12 <= blob.len() {
            let int = |i: usize| u32::from_le_bytes([blob[i], blob[i + 1], blob[i + 2], blob[i + 3]]);
            let (base, path_len, text_len) = (int(at), int(at + 4) as usize, int(at + 8) as usize);
            let path = at + 12;
            let text = path + path_len;
            if pos >= base && ((pos - base) as usize) <= text_len {
                return Some((
                    base,
                    std::str::from_utf8_unchecked(&blob[path..text]),
                    std::str::from_utf8_unchecked(&blob[text..text + text_len]),
                ));
            }
            at = text + text_len;
        }
        None
    }
}

/// Mostra um erro de execução no mesmo formato do compilador e encerra com 101.
unsafe fn report(code: Code, message: String, lo: u32, hi: u32) -> ! {
    unsafe { show(Diagnostic::new(code, message, Span { start: lo, end: hi }), 101) }
}

unsafe fn show(d: Diagnostic, exit: i32) -> ! {
    unsafe {
        flush();
        let mut d = d;
        let (path, text) = match module_of(d.span.start) {
            // A falha num arquivo trazido se mostra nele, com as posições contadas nele.
            Some((base, path, text)) => {
                d.span = Span { start: d.span.start - base, end: d.span.end - base };
                (path, text)
            }
            None => (str_of(&lavra_path, lavra_path_len), str_of(&lavra_src, lavra_src_len)),
        };
        let src = SourceFile::new(path, text);
        let style = Style { color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() };
        eprint!("{}", diagnostics::render(&d, &src, style));
        std::process::exit(exit);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: i32, _argv: *const *const u8) -> i32 {
    unsafe {
        ECHO = std::env::var_os("LAVRA_ECO").is_some();
        lavra_init();
        // "Para começar" conta como a primeira chamada, como no interpretador.
        rt_enter(0, 0);
        let f = lavra_main();
        if !f.is_null() {
            let f = *Box::from_raw(f);
            show(Diagnostic::failure(f.message, Span { start: f.lo, end: f.hi }).primary("o programa desistiu aqui"), 1);
        }
        flush();
        0
    }
}

// ------------------------------------------------------------------ falhas

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault(code: u32, msg: *const u8, len: u64, lo: u32, hi: u32) -> ! {
    unsafe { report(by_number(code).unwrap_or(L0900), str_of(msg, len).to_string(), lo, hi) }
}

/// Erro com um inteiro no lugar de "{}" na mensagem.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_int(code: u32, tpl: *const u8, len: u64, value: i64, lo: u32, hi: u32) -> ! {
    unsafe { report(by_number(code).unwrap_or(L0900), str_of(tpl, len).replacen("{}", &value.to_string(), 1), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_abs(value: i64, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe { report(L0902, textfmt::abs_overflow(value as i128, bits as u8, signed != 0), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_shift(value: i64, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe { report(L0903, textfmt::shift_range(value as i128, bits as u8, signed != 0), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_sqrt(value: f64, lo: u32, hi: u32) -> ! {
    unsafe { report(L0907, textfmt::negative_sqrt(value), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_convert_int(x: i64, src_signed: u32, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe {
        let shown = if src_signed != 0 { x.to_string() } else { (x as u64).to_string() };
        report(L0908, textfmt::convert_error(&shown, bits as u8, signed != 0), lo, hi)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_convert_real(x: f64, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe { report(L0908, textfmt::convert_error(&textfmt::format_real(x), bits as u8, signed != 0), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_to_int(t: *const String, bits: u32, signed: u32, lo: u32, hi: u32) -> i64 {
    unsafe {
        match textfmt::parse_int(&*t, bits as u8, signed != 0) {
            Some(v) => v as i64,
            None => report(L0915, textfmt::not_a_number(&*t, &textfmt::int_noun(bits as u8, signed != 0)), lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_to_real(t: *const String, lo: u32, hi: u32) -> f64 {
    unsafe {
        match textfmt::parse_real(&*t) {
            Some(v) => v,
            None => report(L0915, textfmt::not_a_number(&*t, "real"), lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_int_to_char(x: i64, src_signed: u32, lo: u32, hi: u32) -> u32 {
    unsafe {
        let wide = if src_signed != 0 { x as i128 } else { x as u64 as i128 };
        match textfmt::char_of(wide) {
            Some(c) => c as u32,
            None => report(L0916, textfmt::not_a_char(&wide.to_string()), lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_failure_new(text: *mut String, lo: u32, hi: u32) -> *mut Failure {
    unsafe {
        let message = *Box::from_raw(text);
        Box::into_raw(Box::new(Failure { message, lo, hi }))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_failure_drop(f: *mut Failure) {
    unsafe { drop(Box::from_raw(f)) }
}

/// Conta a profundidade das chamadas (antes de chamar) e acusa o estouro de pilha.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_enter(lo: u32, hi: u32) {
    unsafe {
        if DEPTH >= textfmt::MAX_DEPTH {
            report(L0914, textfmt::stack_overflow(textfmt::MAX_DEPTH), lo, hi);
        }
        DEPTH += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_exit() {
    unsafe { DEPTH -= 1 }
}

// ---------------------------------------------------------------- escrita

unsafe fn format_into(s: &mut String, d: *const Desc, slot: u64) {
    unsafe {
        let d = &*d;
        match d.kind {
            KIND_INT => {
                if d.signed != 0 {
                    s.push_str(&(slot as i64).to_string());
                } else {
                    s.push_str(&slot.to_string());
                }
            }
            KIND_REAL => s.push_str(&textfmt::format_real(real_of(d, slot))),
            KIND_BOOL => s.push_str(textfmt::format_bool(slot != 0)),
            KIND_CHAR => s.push(char::from_u32(slot as u32).unwrap_or('\u{fffd}')),
            KIND_TEXT => s.push_str(&*(slot as *const String)),
            KIND_LIST => {
                let v = &*(slot as *const Vec<u64>);
                s.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        s.push_str("; ");
                    }
                    format_into(s, d.elem, *x);
                }
                s.push(']');
            }
            KIND_SET => {
                let v = &*(slot as *const Vec<u64>);
                s.push('{');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        s.push_str("; ");
                    }
                    format_into(s, d.elem, *x);
                }
                s.push('}');
            }
            KIND_MAP => {
                let v = &*(slot as *const Vec<u64>);
                s.push('{');
                for (i, pair) in v.chunks(2).enumerate() {
                    if i > 0 {
                        s.push_str("; ");
                    }
                    format_into(s, d.elem, pair[0]);
                    s.push_str(": ");
                    format_into(s, map_value(d), pair[1]);
                }
                s.push('}');
            }
            KIND_RECORD => {
                s.push_str(if d.feminine != 0 { "uma " } else { "um " });
                s.push_str(str_of(d.name, d.name_len));
                s.push_str(" com ");
                s.push_str(&fields_text(d.fields, d.nfields, slot as *const u64));
            }
            KIND_ENUM => {
                let (v, p) = variant(d, slot);
                s.push_str(if v.feminine != 0 { "uma " } else { "um " });
                s.push_str(str_of(v.name, v.name_len));
                if v.nfields > 0 {
                    s.push_str(" com ");
                    s.push_str(&fields_text(v.fields, v.nfields, p.add(1)));
                }
            }
            KIND_MAYBE => {
                if slot == 0 {
                    s.push_str("nada");
                } else {
                    format_into(s, d.elem, *(slot as *const u64));
                }
            }
            _ => unreachable!(),
        }
    }
}

unsafe fn real_of(d: &Desc, slot: u64) -> f64 {
    if d.bits == 32 { f32::from_bits(slot as u32) as f64 } else { f64::from_bits(slot) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_write(d: *const Desc, slot: u64) {
    unsafe {
        let mut s = String::new();
        format_into(&mut s, d, slot);
        let _ = out().write_all(s.as_bytes());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_newline() {
    unsafe {
        let _ = out().write_all(b"\n");
    }
}

// ------------------------------------------------------------------ texto

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_lit(ptr: *const u8, len: u64) -> *mut String {
    unsafe { Box::into_raw(Box::new(str_of(ptr, len).to_string())) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_new() -> *mut String {
    Box::into_raw(Box::new(String::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_append(t: *mut String, d: *const Desc, slot: u64) {
    unsafe { format_into(&mut *t, d, slot) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_char(t: *const String, n: i64, lo: u32, hi: u32) -> u32 {
    unsafe {
        let s = &*t;
        let len = s.chars().count();
        if n < 1 || n as usize > len {
            report(L0909, textfmt::position_error(n as i128, len), lo, hi);
        }
        s.chars().nth(n as usize - 1).unwrap() as u32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_last_char(t: *const String, lo: u32, hi: u32) -> u32 {
    unsafe {
        match (*t).chars().last() {
            Some(c) => c as u32,
            None => report(L0910, textfmt::EMPTY_LAST.to_string(), lo, hi),
        }
    }
}

/// As letras do texto como lista de caracteres (para "Para cada letra do nome").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_chars(t: *const String) -> *mut Vec<u64> {
    unsafe { Box::into_raw(Box::new((*t).chars().map(|c| c as u64).collect())) }
}

// ------------------------------------------------------- operações embutidas

// As operações que as bibliotecas declaram (`src/builtins.rs`): recebem os argumentos
// emprestados, mais o trecho da chamada (para relatar uma falha), e devolvem um valor
// novo. Fazem o mesmo que o interpretador, byte a byte.

/// Um texto novo, como valor de lista.
fn text_slot(s: String) -> u64 {
    Box::into_raw(Box::new(s)) as u64
}

/// "dividir": os pedaços entre as vezes em que o separador aparece; o separador vazio dá
/// cada caractere.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_split(t: *const String, sep: *const String, _lo: u32, _hi: u32) -> *mut Vec<u64> {
    unsafe {
        let (t, sep) = (&*t, &*sep);
        let parts: Vec<u64> = if sep.is_empty() {
            t.chars().map(|c| text_slot(c.to_string())).collect()
        } else {
            t.split(sep.as_str()).map(|p| text_slot(p.to_string())).collect()
        };
        Box::into_raw(Box::new(parts))
    }
}

/// "juntar": os textos da lista um depois do outro, com o separador entre eles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_join(l: *const Vec<u64>, sep: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe {
        let parts: Vec<&str> = (*l).iter().map(|&slot| (*(slot as *const String)).as_str()).collect();
        Box::into_raw(Box::new(parts.join((*sep).as_str())))
    }
}

/// "conter": se o trecho aparece no texto.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_contains(t: *const String, part: *const String, _lo: u32, _hi: u32) -> u8 {
    unsafe { (*t).contains((*part).as_str()) as u8 }
}

/// "aparar": o texto sem o espaço em branco do começo e do fim.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_trim(t: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new((*t).trim().to_string())) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_upper(t: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new((*t).to_uppercase())) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_lower(t: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new((*t).to_lowercase())) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_reverse(t: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new((*t).chars().rev().collect::<String>())) }
}

/// "substituir o trecho pelo novo no texto": os argumentos na ordem do cabeçalho.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_replace(part: *const String, new: *const String, t: *const String, _lo: u32, _hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new(textfmt::replace(&*t, &*part, &*new))) }
}

/// "localizar o trecho no texto": a posição, ou nada (o "talvez" é a caixa de `rt_box`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_locate(part: *const String, t: *const String, _lo: u32, _hi: u32) -> *mut u64 {
    unsafe {
        match textfmt::locate(&*t, &*part) {
            Some(p) => rt_box(p as u64),
            None => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_slice(t: *const String, from: i64, to: i64, lo: u32, hi: u32) -> *mut String {
    unsafe {
        match textfmt::slice(&*t, from as i128, to as i128) {
            Ok(s) => Box::into_raw(Box::new(s)),
            Err(message) => report(L0917, message, lo, hi),
        }
    }
}

// --------------------------------------------------------------- arquivos

/// Deixa a falha da operação na casa que o programa passou, com o trecho da chamada.
unsafe fn fail_into(out: *mut *mut Failure, message: String, lo: u32, hi: u32) {
    unsafe { *out = Box::into_raw(Box::new(Failure { message, lo, hi })) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_load(path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) -> *mut String {
    unsafe {
        match textfmt::file_load(&*path) {
            Ok(t) => Box::into_raw(Box::new(t)),
            Err(m) => {
                fail_into(out, m, lo, hi);
                std::ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_lines(path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) -> *mut Vec<u64> {
    unsafe {
        match textfmt::file_lines(&*path) {
            Ok(lines) => Box::into_raw(Box::new(lines.into_iter().map(text_slot).collect::<Vec<u64>>())),
            Err(m) => {
                fail_into(out, m, lo, hi);
                std::ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_save(text: *const String, path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) {
    unsafe {
        if let Err(m) = textfmt::file_save(&*text, &*path, false) {
            fail_into(out, m, lo, hi);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_append(text: *const String, path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) {
    unsafe {
        if let Err(m) = textfmt::file_save(&*text, &*path, true) {
            fail_into(out, m, lo, hi);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_remove(path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) {
    unsafe {
        if let Err(m) = textfmt::file_remove(&*path) {
            fail_into(out, m, lo, hi);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_file_exists(path: *const String, _lo: u32, _hi: u32) -> u8 {
    unsafe { textfmt::file_exists(&*path) as u8 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_dir_list(path: *const String, out: *mut *mut Failure, lo: u32, hi: u32) -> *mut Vec<u64> {
    unsafe {
        match textfmt::dir_list(&*path) {
            Ok(names) => Box::into_raw(Box::new(names.into_iter().map(text_slot).collect::<Vec<u64>>())),
            Err(m) => {
                fail_into(out, m, lo, hi);
                std::ptr::null_mut()
            }
        }
    }
}

// ------------------------------------------------------------------ tempo

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_now(_lo: u32, _hi: u32) -> i64 {
    textfmt::now_millis()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_year(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[0]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_month(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[1]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_day(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[2]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_hour(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[3]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_minute(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[4]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_second(millis: i64, _lo: u32, _hi: u32) -> i64 {
    textfmt::civil(millis)[5]
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_time_text(millis: i64, _lo: u32, _hi: u32) -> *mut String {
    Box::into_raw(Box::new(textfmt::date_text(millis)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_time_sleep(millis: i64, _lo: u32, _hi: u32) {
    unsafe { flush() };
    textfmt::sleep_millis(millis);
}

// ------------------------------------------------------------- matemática

#[unsafe(no_mangle)]
pub extern "C" fn rt_math_sin(x: f64, _lo: u32, _hi: u32) -> f64 {
    x.sin()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_math_cos(x: f64, _lo: u32, _hi: u32) -> f64 {
    x.cos()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_math_tan(x: f64, _lo: u32, _hi: u32) -> f64 {
    x.tan()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_ln(x: f64, lo: u32, hi: u32) -> f64 {
    if x.is_nan() || x <= 0.0 {
        unsafe { report(L0918, textfmt::log_domain(x), lo, hi) }
    }
    x.ln()
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_math_exp(x: f64, _lo: u32, _hi: u32) -> f64 {
    x.exp()
}

/// O real já arredondado vira inteiro, se couber.
unsafe fn whole(x: f64, lo: u32, hi: u32) -> i64 {
    if !textfmt::real_fits(x, 64, true) {
        unsafe { report(L0908, textfmt::convert_error(&textfmt::format_real(x), 64, true), lo, hi) }
    }
    x as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_floor(x: f64, lo: u32, hi: u32) -> i64 {
    unsafe { whole(x.floor(), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_ceil(x: f64, lo: u32, hi: u32) -> i64 {
    unsafe { whole(x.ceil(), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_round(x: f64, lo: u32, hi: u32) -> i64 {
    unsafe { whole(x.round(), lo, hi) }
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_math_pi(_lo: u32, _hi: u32) -> f64 {
    std::f64::consts::PI
}

/// O estado dos sorteios; sem semente, começa pelo relógio.
static mut RANDOM: Option<u64> = None;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_random(a: i64, b: i64, _lo: u32, _hi: u32) -> i64 {
    unsafe {
        let state = &mut *std::ptr::addr_of_mut!(RANDOM);
        let s = state.get_or_insert_with(textfmt::clock_seed);
        textfmt::random_between(s, a, b)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_math_seed(seed: i64, _lo: u32, _hi: u32) {
    unsafe { *std::ptr::addr_of_mut!(RANDOM) = Some(seed as u64) }
}

// ------------------------------------------------------------------ lista

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_new() -> *mut Vec<u64> {
    Box::into_raw(Box::new(Vec::new()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_push(l: *mut Vec<u64>, slot: u64) {
    unsafe { (*l).push(slot) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_len(l: *const Vec<u64>) -> u64 {
    unsafe { (*l).len() as u64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_data(l: *mut Vec<u64>) -> *mut u64 {
    unsafe { (*l).as_mut_ptr() }
}

/// Endereço da casa na posição `n` (a partir de 1), conferindo o tamanho.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_addr(l: *mut Vec<u64>, n: i64, lo: u32, hi: u32) -> *mut u64 {
    unsafe {
        let v = &mut *l;
        if n < 1 || n as usize > v.len() {
            report(L0909, textfmt::position_error(n as i128, v.len()), lo, hi);
        }
        v.as_mut_ptr().add(n as usize - 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_last_addr(l: *mut Vec<u64>, lo: u32, hi: u32) -> *mut u64 {
    unsafe {
        let v = &mut *l;
        if v.is_empty() {
            report(L0910, textfmt::EMPTY_LAST.to_string(), lo, hi);
        }
        let n = v.len();
        v.as_mut_ptr().add(n - 1)
    }
}

/// Tira o item `n` (0 é o último) e devolve.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_take(l: *mut Vec<u64>, n: i64, lo: u32, hi: u32) -> u64 {
    unsafe {
        let v = &mut *l;
        if n == 0 {
            return match v.pop() {
                Some(x) => x,
                None => report(L0911, textfmt::EMPTY_TAKE.to_string(), lo, hi),
            };
        }
        if n < 1 || n as usize > v.len() {
            report(L0909, textfmt::position_error(n as i128, v.len()), lo, hi);
        }
        v.remove(n as usize - 1)
    }
}

// --------------------------------------------------------- registro, talvez

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_record_new(n: u64) -> *mut u64 {
    let b: Box<[u64]> = vec![0u64; n.max(1) as usize].into_boxed_slice();
    Box::into_raw(b) as *mut u64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_box(slot: u64) -> *mut u64 {
    Box::into_raw(Box::new(slot))
}

// ------------------------------------------------- liberar, copiar, comparar

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_drop(d: *const Desc, slot: u64) {
    unsafe {
        let d = &*d;
        match d.kind {
            KIND_TEXT => drop(Box::from_raw(slot as *mut String)),
            KIND_LIST | KIND_SET => {
                let v = Box::from_raw(slot as *mut Vec<u64>);
                if !is_copy(d.elem) {
                    for x in v.iter() {
                        rt_drop(d.elem, *x);
                    }
                }
            }
            KIND_MAP => {
                let v = Box::from_raw(slot as *mut Vec<u64>);
                for pair in v.chunks(2) {
                    drop_slot(d.elem, pair[0]);
                    drop_slot(map_value(d), pair[1]);
                }
            }
            KIND_RECORD => {
                let p = slot as *mut u64;
                for i in 0..d.nfields as usize {
                    let f = &*d.fields.add(i);
                    if !is_copy(f.desc) {
                        rt_drop(f.desc, *p.add(i));
                    }
                }
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, (d.nfields as usize).max(1))));
            }
            KIND_ENUM => {
                let (v, p) = variant(d, slot);
                for i in 0..v.nfields as usize {
                    let f = &*v.fields.add(i);
                    if !is_copy(f.desc) {
                        rt_drop(f.desc, *p.add(i + 1));
                    }
                }
                drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(p, v.nfields as usize + 1)));
            }
            KIND_MAYBE => {
                if slot != 0 {
                    let b = Box::from_raw(slot as *mut u64);
                    if !is_copy(d.elem) {
                        rt_drop(d.elem, *b);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Libera o valor, se ele tem dono.
unsafe fn drop_slot(d: *const Desc, slot: u64) {
    unsafe {
        if !is_copy(d) {
            rt_drop(d, slot);
        }
    }
}

/// O descritor dos valores de um mapa.
unsafe fn map_value(d: &Desc) -> *const Desc {
    d.fields as *const Desc
}

/// A chave como se escreve no programa, para a mensagem da chave que falta.
unsafe fn key_shown(d: *const Desc, slot: u64) -> String {
    unsafe {
        match (*d).kind {
            KIND_TEXT => textfmt::quoted_text(&*(slot as *const String)),
            KIND_CHAR => textfmt::quoted_char(char::from_u32(slot as u32).unwrap_or('\u{fffd}')),
            _ => {
                let mut s = String::new();
                format_into(&mut s, d, slot);
                s
            }
        }
    }
}

/// Onde a chave está (ou onde entraria) numa coleção em ordem, com `stride` casas por item.
unsafe fn find(v: &[u64], stride: usize, d: *const Desc, key: u64) -> Result<usize, usize> {
    unsafe {
        let (mut lo, mut hi) = (0, v.len() / stride);
        while lo < hi {
            let mid = (lo + hi) / 2;
            match rt_cmp(d, v[mid * stride], key) {
                0 => return Ok(mid),
                c if c < 0 => lo = mid + 1,
                _ => hi = mid,
            }
        }
        Err(lo)
    }
}

/// "Os vistos ganham "Ana".": o valor entra no lugar dele; o que já está lá não entra de
/// novo (e o valor que veio é liberado).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_set_insert(set: *mut Vec<u64>, d: *const Desc, slot: u64) {
    unsafe {
        let v = &mut *set;
        match find(v, 1, d, slot) {
            Ok(_) => drop_slot(d, slot),
            Err(i) => v.insert(i, slot),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_set_remove(set: *mut Vec<u64>, d: *const Desc, slot: u64) {
    unsafe {
        let v = &mut *set;
        if let Ok(i) = find(v, 1, d, slot) {
            drop_slot(d, v.remove(i));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_set_contains(d: *const Desc, set: *const Vec<u64>, slot: u64) -> u8 {
    unsafe { find(&*set, 1, d, slot).is_ok() as u8 }
}

/// A união (0), a interseção (1) e a diferença (2) de dois conjuntos: um conjunto novo.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_set_op(op: u32, d: *const Desc, a: *const Vec<u64>, b: *const Vec<u64>) -> *mut Vec<u64> {
    unsafe {
        let (a, b) = (&*a, &*b);
        let mut out: Vec<u64> = match op {
            0 => a.iter().chain(b.iter().filter(|x| find(a, 1, d, **x).is_err())).map(|x| rt_clone(d, *x)).collect(),
            1 => a.iter().filter(|x| find(b, 1, d, **x).is_ok()).map(|x| rt_clone(d, *x)).collect(),
            _ => a.iter().filter(|x| find(b, 1, d, **x).is_err()).map(|x| rt_clone(d, *x)).collect(),
        };
        out.sort_by(|x, y| rt_cmp(d, *x, *y).cmp(&0));
        Box::into_raw(Box::new(out))
    }
}

/// A casa do valor de uma chave; a chave que o mapa não tem é falha.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_map_addr(map: *mut Vec<u64>, kd: *const Desc, key: u64, lo: u32, hi: u32) -> *mut u64 {
    unsafe {
        let v = &mut *map;
        match find(v, 2, kd, key) {
            Ok(i) => v.as_mut_ptr().add(i * 2 + 1),
            Err(_) => report(L0919, textfmt::missing_key(&key_shown(kd, key)), lo, hi),
        }
    }
}

/// "O item "Ana" das idades vira 30.": troca o valor da chave, ou a põe no mapa (com uma
/// cópia da chave, que veio emprestada).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_map_put(map: *mut Vec<u64>, kd: *const Desc, vd: *const Desc, key: u64, value: u64) {
    unsafe {
        let v = &mut *map;
        match find(v, 2, kd, key) {
            Ok(i) => {
                drop_slot(vd, v[i * 2 + 1]);
                v[i * 2 + 1] = value;
            }
            Err(i) => {
                let own = rt_clone(kd, key);
                v.splice(i * 2..i * 2, [own, value]);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_map_remove(map: *mut Vec<u64>, kd: *const Desc, vd: *const Desc, key: u64) {
    unsafe {
        let v = &mut *map;
        if let Ok(i) = find(v, 2, kd, key) {
            let gone: Vec<u64> = v.drain(i * 2..i * 2 + 2).collect();
            drop_slot(kd, gone[0]);
            drop_slot(vd, gone[1]);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_map_contains(kd: *const Desc, map: *const Vec<u64>, key: u64) -> u8 {
    unsafe { find(&*map, 2, kd, key).is_ok() as u8 }
}

unsafe fn is_copy(d: *const Desc) -> bool {
    unsafe { matches!((*d).kind, KIND_INT | KIND_REAL | KIND_BOOL | KIND_CHAR) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_clone(d: *const Desc, slot: u64) -> u64 {
    unsafe {
        let d = &*d;
        match d.kind {
            KIND_TEXT => Box::into_raw(Box::new((*(slot as *const String)).clone())) as u64,
            KIND_LIST | KIND_SET => {
                let v = &*(slot as *const Vec<u64>);
                let c: Vec<u64> = v.iter().map(|x| rt_clone(d.elem, *x)).collect();
                Box::into_raw(Box::new(c)) as u64
            }
            KIND_MAP => {
                let v = &*(slot as *const Vec<u64>);
                let c: Vec<u64> = v.chunks(2).flat_map(|p| [rt_clone(d.elem, p[0]), rt_clone(map_value(d), p[1])]).collect();
                Box::into_raw(Box::new(c)) as u64
            }
            KIND_RECORD => {
                let p = slot as *const u64;
                let n = rt_record_new(d.nfields);
                for i in 0..d.nfields as usize {
                    let f = &*d.fields.add(i);
                    *n.add(i) = rt_clone(f.desc, *p.add(i));
                }
                n as u64
            }
            KIND_ENUM => {
                let (v, p) = variant(d, slot);
                let n = rt_record_new(v.nfields + 1);
                *n = *p;
                for i in 0..v.nfields as usize {
                    let f = &*v.fields.add(i);
                    *n.add(i + 1) = rt_clone(f.desc, *p.add(i + 1));
                }
                n as u64
            }
            KIND_MAYBE => {
                if slot == 0 {
                    0
                } else {
                    rt_box(rt_clone(d.elem, *(slot as *const u64))) as u64
                }
            }
            _ => slot,
        }
    }
}

/// -1, 0 ou 1 quando há ordem; 2 quando só se sabe que são diferentes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_cmp(d: *const Desc, a: u64, b: u64) -> i32 {
    unsafe {
        let d = &*d;
        let ord = |o: Option<std::cmp::Ordering>| match o {
            Some(std::cmp::Ordering::Less) => -1,
            Some(std::cmp::Ordering::Equal) => 0,
            Some(std::cmp::Ordering::Greater) => 1,
            None => 2,
        };
        match d.kind {
            KIND_INT => {
                if d.signed != 0 { ord((a as i64).partial_cmp(&(b as i64))) } else { ord(a.partial_cmp(&b)) }
            }
            KIND_REAL => ord(real_of(d, a).partial_cmp(&real_of(d, b))),
            KIND_BOOL => ord((a != 0).partial_cmp(&(b != 0))),
            KIND_CHAR => ord((a as u32).partial_cmp(&(b as u32))),
            KIND_TEXT => ord((*(a as *const String)).partial_cmp(&*(b as *const String))),
            KIND_LIST | KIND_SET => {
                let (x, y) = (&*(a as *const Vec<u64>), &*(b as *const Vec<u64>));
                let eq = x.len() == y.len() && x.iter().zip(y).all(|(p, q)| rt_cmp(d.elem, *p, *q) == 0);
                if eq { 0 } else { 2 }
            }
            KIND_MAP => {
                let (x, y) = (&*(a as *const Vec<u64>), &*(b as *const Vec<u64>));
                let eq = x.len() == y.len()
                    && x.chunks(2).zip(y.chunks(2)).all(|(p, q)| rt_cmp(d.elem, p[0], q[0]) == 0 && rt_cmp(map_value(d), p[1], q[1]) == 0);
                if eq { 0 } else { 2 }
            }
            KIND_RECORD => {
                let (x, y) = (a as *const u64, b as *const u64);
                let eq = (0..d.nfields as usize).all(|i| rt_cmp((*d.fields.add(i)).desc, *x.add(i), *y.add(i)) == 0);
                if eq { 0 } else { 2 }
            }
            KIND_ENUM => {
                let (va, pa) = variant(d, a);
                let pb = b as *const u64;
                if *pa != *pb {
                    return 2;
                }
                let eq = (0..va.nfields as usize).all(|i| rt_cmp((*va.fields.add(i)).desc, *pa.add(i + 1), *pb.add(i + 1)) == 0);
                if eq { 0 } else { 2 }
            }
            KIND_MAYBE => match (a, b) {
                (0, 0) => 0,
                (0, _) | (_, 0) => 2,
                _ => match rt_cmp(d.elem, *(a as *const u64), *(b as *const u64)) {
                    0 => 0,
                    _ => 2,
                },
            },
            _ => unreachable!(),
        }
    }
}

/// "Se houver 7 nas notas": algum item da lista é igual ao valor?
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_contains(elem: *const Desc, l: *const Vec<u64>, slot: u64) -> u8 {
    unsafe { (*l).iter().any(|x| rt_cmp(elem, *x, slot) == 0) as u8 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_has(t: *const String, part: *const String) -> u8 {
    unsafe { (*t).contains((*part).as_str()) as u8 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_has_char(t: *const String, c: u32) -> u8 {
    unsafe { char::from_u32(c).is_some_and(|c| (*t).contains(c)) as u8 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_len(d: *const Desc, slot: u64) -> i64 {
    unsafe {
        match (*d).kind {
            KIND_TEXT => (*(slot as *const String)).chars().count() as i64,
            KIND_LIST | KIND_SET => (*(slot as *const Vec<u64>)).len() as i64,
            KIND_MAP => (*(slot as *const Vec<u64>)).len() as i64 / 2,
            _ => unreachable!(),
        }
    }
}

// ------------------------------------------------------------------ contas

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_pow_int(x: i64, y: i64, bits: u32, signed: u32, lo: u32, hi: u32) -> i64 {
    unsafe {
        let (bits, signed) = (bits as u8, signed != 0);
        let xv: i128 = if signed { x as i128 } else { x as u64 as i128 };
        let yv: i128 = if signed { y as i128 } else { y as u64 as i128 };
        if yv < 0 {
            report(L0906, textfmt::NEG_POW.to_string(), lo, hi);
        }
        let mut r: i128 = 1;
        for _ in 0..yv.min(200) {
            match r.checked_mul(xv) {
                Some(v) if v >= textfmt::int_min(bits, signed) && v <= textfmt::int_max(bits, signed) => r = v,
                _ => report(L0901, textfmt::pow_overflow(bits, signed), lo, hi),
            }
        }
        r as i64
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_pow_real(x: f64, y: f64) -> f64 {
    x.powf(y)
}

#[unsafe(no_mangle)]
pub extern "C" fn rt_fmod(x: f64, y: f64) -> f64 {
    x % y
}

// ---------------------------------------------------------------- leitura

unsafe fn read_line(lo: u32, hi: u32) -> String {
    unsafe {
        flush();
        let input = INPUT.get_or_insert_with(|| std::io::stdin().lock());
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) | Err(_) => report(L0912, textfmt::INPUT_ENDED.to_string(), lo, hi),
            Ok(_) => {}
        }
        let line = line.trim_end_matches(['\n', '\r']).to_string();
        if ECHO {
            let _ = writeln!(out(), "{line}");
        }
        line
    }
}

unsafe fn bad_input(expected: *const u8, len: u64, line: &str, lo: u32, hi: u32) -> ! {
    unsafe { report(L0913, textfmt::bad_input(str_of(expected, len), line), lo, hi) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_read_int(bits: u32, signed: u32, exp: *const u8, len: u64, lo: u32, hi: u32) -> i64 {
    unsafe {
        let line = read_line(lo, hi);
        match textfmt::parse_int(&line, bits as u8, signed != 0) {
            Some(v) => v as i64,
            None => bad_input(exp, len, &line, lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_read_real(exp: *const u8, len: u64, lo: u32, hi: u32) -> f64 {
    unsafe {
        let line = read_line(lo, hi);
        match textfmt::parse_real(&line) {
            Some(v) => v,
            None => bad_input(exp, len, &line, lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_read_bool(exp: *const u8, len: u64, lo: u32, hi: u32) -> u8 {
    unsafe {
        let line = read_line(lo, hi);
        match textfmt::parse_bool(&line) {
            Some(v) => v as u8,
            None => bad_input(exp, len, &line, lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_read_char(exp: *const u8, len: u64, lo: u32, hi: u32) -> u32 {
    unsafe {
        let line = read_line(lo, hi);
        match textfmt::parse_char(&line) {
            Some(v) => v as u32,
            None => bad_input(exp, len, &line, lo, hi),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_read_text(lo: u32, hi: u32) -> *mut String {
    unsafe { Box::into_raw(Box::new(read_line(lo, hi))) }
}
