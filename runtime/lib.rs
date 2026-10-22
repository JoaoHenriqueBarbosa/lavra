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
#[path = "../src/diagnostics.rs"]
mod diagnostics;
#[path = "../src/textfmt.rs"]
mod textfmt;

use diagnostics::{Category, Diagnostic, Style};
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

/// Mostra o diagnóstico no mesmo formato do compilador e encerra.
unsafe fn report(category: Category, message: String, lo: u32, hi: u32, primary: Option<&str>, code: i32) -> ! {
    unsafe {
        flush();
        let text = str_of(&lavra_src, lavra_src_len);
        let path = str_of(&lavra_path, lavra_path_len);
        let src = SourceFile::new(path, text);
        let mut d = Diagnostic::new(category, message, Span { start: lo, end: hi });
        if let Some(p) = primary {
            d = d.primary(p);
        }
        let style = Style { color: std::io::stderr().is_terminal() && std::env::var_os("NO_COLOR").is_none() };
        eprint!("{}", diagnostics::render(&d, &src, style));
        std::process::exit(code);
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
            report(Category::Failure, f.message, f.lo, f.hi, Some("o programa desistiu aqui"), 1);
        }
        flush();
        0
    }
}

// ------------------------------------------------------------------ falhas

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault(msg: *const u8, len: u64, lo: u32, hi: u32) -> ! {
    unsafe { report(Category::Runtime, str_of(msg, len).to_string(), lo, hi, None, 101) }
}

/// Erro com um inteiro no lugar de "{}" na mensagem.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_int(tpl: *const u8, len: u64, value: i64, lo: u32, hi: u32) -> ! {
    unsafe { report(Category::Runtime, str_of(tpl, len).replacen("{}", &value.to_string(), 1), lo, hi, None, 101) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_abs(value: i64, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe { report(Category::Runtime, textfmt::abs_overflow(value as i128, bits as u8, signed != 0), lo, hi, None, 101) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_shift(value: i64, bits: u32, signed: u32, lo: u32, hi: u32) -> ! {
    unsafe { report(Category::Runtime, textfmt::shift_range(value as i128, bits as u8, signed != 0), lo, hi, None, 101) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_fault_sqrt(value: f64, lo: u32, hi: u32) -> ! {
    unsafe { report(Category::Runtime, textfmt::negative_sqrt(value), lo, hi, None, 101) }
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
            report(Category::Runtime, textfmt::stack_overflow(), lo, hi, None, 101);
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
            report(Category::Runtime, textfmt::position_error(n as i128, len), lo, hi, None, 101);
        }
        s.chars().nth(n as usize - 1).unwrap() as u32
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_last_char(t: *const String, lo: u32, hi: u32) -> u32 {
    unsafe {
        match (*t).chars().last() {
            Some(c) => c as u32,
            None => report(Category::Runtime, textfmt::EMPTY_LAST.to_string(), lo, hi, None, 101),
        }
    }
}

/// As letras do texto como lista de caracteres (para "Para cada letra do nome").
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_text_chars(t: *const String) -> *mut Vec<u64> {
    unsafe { Box::into_raw(Box::new((*t).chars().map(|c| c as u64).collect())) }
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
            report(Category::Runtime, textfmt::position_error(n as i128, v.len()), lo, hi, None, 101);
        }
        v.as_mut_ptr().add(n as usize - 1)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_list_last_addr(l: *mut Vec<u64>, lo: u32, hi: u32) -> *mut u64 {
    unsafe {
        let v = &mut *l;
        if v.is_empty() {
            report(Category::Runtime, textfmt::EMPTY_LAST.to_string(), lo, hi, None, 101);
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
                None => report(Category::Runtime, textfmt::EMPTY_TAKE.to_string(), lo, hi, None, 101),
            };
        }
        if n < 1 || n as usize > v.len() {
            report(Category::Runtime, textfmt::position_error(n as i128, v.len()), lo, hi, None, 101);
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
            KIND_LIST => {
                let v = Box::from_raw(slot as *mut Vec<u64>);
                if !is_copy(d.elem) {
                    for x in v.iter() {
                        rt_drop(d.elem, *x);
                    }
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

unsafe fn is_copy(d: *const Desc) -> bool {
    unsafe { matches!((*d).kind, KIND_INT | KIND_REAL | KIND_BOOL | KIND_CHAR) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_clone(d: *const Desc, slot: u64) -> u64 {
    unsafe {
        let d = &*d;
        match d.kind {
            KIND_TEXT => Box::into_raw(Box::new((*(slot as *const String)).clone())) as u64,
            KIND_LIST => {
                let v = &*(slot as *const Vec<u64>);
                let c: Vec<u64> = v.iter().map(|x| rt_clone(d.elem, *x)).collect();
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
            KIND_LIST => {
                let (x, y) = (&*(a as *const Vec<u64>), &*(b as *const Vec<u64>));
                let eq = x.len() == y.len() && x.iter().zip(y).all(|(p, q)| rt_cmp(d.elem, *p, *q) == 0);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rt_len(d: *const Desc, slot: u64) -> i64 {
    unsafe {
        match (*d).kind {
            KIND_TEXT => (*(slot as *const String)).chars().count() as i64,
            KIND_LIST => (*(slot as *const Vec<u64>)).len() as i64,
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
            report(Category::Runtime, textfmt::NEG_POW.to_string(), lo, hi, None, 101);
        }
        let mut r: i128 = 1;
        for _ in 0..yv.min(200) {
            match r.checked_mul(xv) {
                Some(v) if v >= textfmt::int_min(bits, signed) && v <= textfmt::int_max(bits, signed) => r = v,
                _ => report(Category::Runtime, textfmt::pow_overflow(bits, signed), lo, hi, None, 101),
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
            Ok(0) | Err(_) => report(Category::Runtime, textfmt::INPUT_ENDED.to_string(), lo, hi, None, 101),
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
    unsafe { report(Category::Runtime, textfmt::bad_input(str_of(expected, len), line), lo, hi, None, 101) }
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
