//! Formatação de valores e mensagens de falha de execução. O interpretador e o runtime
//! nativo usam este mesmo arquivo (o runtime o inclui por `#[path]`), e é daí que vem a
//! paridade byte a byte entre os dois.
//!
//! Arquivo autocontido: nada de `use crate::`.

/// Profundidade máxima de chamadas antes de acusar estouro de pilha.
pub const MAX_DEPTH: u64 = 10_000;

/// Real com vírgula decimal, arredondado a 12 algarismos significativos.
pub fn format_real(v: f64) -> String {
    if v.is_nan() {
        return "indefinido".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "infinito" } else { "-infinito" }.into();
    }
    let rounded: f64 = format!("{v:.11e}").parse().unwrap_or(v);
    let s = if rounded != 0.0 && (rounded.abs() >= 1e15 || rounded.abs() < 1e-6) { format!("{rounded:e}") } else { format!("{rounded}") };
    let s = if s == "-0" { "0".to_string() } else { s };
    s.replace('.', ",")
}

pub fn format_bool(b: bool) -> &'static str {
    if b { "verdadeiro" } else { "falso" }
}

pub fn position_error(n: i128, len: usize) -> String {
    let items = if len == 1 { "1 item".to_string() } else { format!("{len} itens") };
    format!("Não há item na posição {n}: a coleção tem {items}.")
}

pub const EMPTY_LAST: &str = "A coleção está vazia: não há último.";
pub const EMPTY_TAKE: &str = "A coleção está vazia: não há último para tirar.";
pub const DIV_ZERO: &str = "Divisão por zero.";
pub const REM_ZERO: &str = "Resto de divisão por zero.";
pub const NEG_POW: &str = "Potência de inteiro com expoente negativo: use reais.";
pub const INPUT_ENDED: &str = "A entrada acabou antes da hora: o programa esperava ler mais.";

/// A falha de chamadas aninhadas demais, com o limite de quem roda (`MAX_DEPTH` no lavrac e
/// no nativo; menos no navegador, onde a pilha é a do motor JavaScript).
pub fn stack_overflow(limit: u64) -> String {
    format!("Estouro de pilha: chamadas demais aninhadas (mais de {limit}). Recursão sem fim?")
}

/// "inteiro", "natural de 32 bits", "byte".
pub fn int_noun(bits: u8, signed: bool) -> String {
    if !signed && bits == 8 {
        return "byte".to_string();
    }
    let base = if signed { "inteiro" } else { "natural" };
    if bits == 64 { base.to_string() } else { format!("{base} de {bits} bits") }
}

pub fn int_min(bits: u8, signed: bool) -> i128 {
    if signed { -(1i128 << (bits - 1)) } else { 0 }
}

pub fn int_max(bits: u8, signed: bool) -> i128 {
    if signed { (1i128 << (bits - 1)) - 1 } else { (1i128 << bits) - 1 }
}

pub fn overflow(bits: u8, signed: bool) -> String {
    format!("Estouro: o resultado não cabe num {} (vai de {} a {}).", int_noun(bits, signed), int_min(bits, signed), int_max(bits, signed))
}

pub fn pow_overflow(bits: u8, signed: bool) -> String {
    format!("Estouro: a potência não cabe num {} (vai de {} a {}).", int_noun(bits, signed), int_min(bits, signed), int_max(bits, signed))
}

pub fn abs_overflow(x: i128, bits: u8, signed: bool) -> String {
    format!("Estouro: o módulo de {x} não cabe num {}.", int_noun(bits, signed))
}

pub fn shift_range(y: i128, bits: u8, signed: bool) -> String {
    format!("Deslocamento de {y} bits num {}, que tem {bits} bits.", int_noun(bits, signed))
}

pub fn convert_error(shown: &str, bits: u8, signed: bool) -> String {
    format!("Conversão impossível: {shown} não cabe num {}.", int_noun(bits, signed))
}

/// "abc" como inteiro: o texto não é um número do tipo pedido.
pub fn not_a_number(text: &str, noun: &str) -> String {
    format!("Conversão impossível: \"{text}\" não é um {noun}.")
}

/// 1114112 como caractere: o número não é código de caractere nenhum.
pub fn not_a_char(shown: &str) -> String {
    format!("Conversão impossível: {shown} não é o código de um caractere.")
}

/// O caractere de código `x`, se existe.
pub fn char_of(x: i128) -> Option<char> {
    u32::try_from(x).ok().and_then(char::from_u32)
}

/// "localizar": a posição (a partir de 1, em caracteres) da primeira aparição do trecho.
pub fn locate(text: &str, part: &str) -> Option<i64> {
    text.find(part).map(|byte| text[..byte].chars().count() as i64 + 1)
}

/// "substituir": cada aparição do trecho trocada; o trecho vazio não troca nada.
pub fn replace(text: &str, part: &str, new: &str) -> String {
    if part.is_empty() { text.to_string() } else { text.replace(part, new) }
}

/// "recortar": os caracteres de `from` a `to` (a partir de 1, incluídos), ou a mensagem da
/// falha quando o recorte sai do texto.
pub fn slice(text: &str, from: i128, to: i128) -> Result<String, String> {
    let len = text.chars().count() as i128;
    if from < 1 || to > len || from > to + 1 {
        let size = if len == 1 { "1 caractere".to_string() } else { format!("{len} caracteres") };
        return Err(format!("Não há como recortar da posição {from} à {to}: o texto tem {size}."));
    }
    Ok(text.chars().skip(from as usize - 1).take((to - from + 1) as usize).collect())
}

pub fn log_domain(x: f64) -> String {
    format!("Logaritmo de número que não é positivo: {}.", format_real(x))
}

/// O próximo número da sequência dos sorteios (splitmix64): a mesma semente dá a mesma
/// sequência, no interpretador e no nativo.
pub fn next_random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Um inteiro ao acaso entre os dois limites, incluídos, em qualquer ordem.
pub fn random_between(state: &mut u64, a: i64, b: i64) -> i64 {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    let range = (hi as i128 - lo as i128 + 1) as u128;
    let pick = (next_random(state) as u128 * range) >> 64;
    (lo as i128 + pick as i128) as i64
}

/// A semente de quando o programa não semeou: o relógio.
pub fn clock_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x1234_5678, |d| d.as_nanos() as u64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0x1234_5678
    }
}

/// O real `x`, sem a parte fracionária, cabe no inteiro? (NaN e infinito não cabem.)
pub fn real_fits(x: f64, bits: u8, signed: bool) -> bool {
    let t = x.trunc();
    t >= int_min(bits, signed) as f64 && t < (int_max(bits, signed) + 1) as f64
}

pub fn negative_sqrt(x: f64) -> String {
    format!("Raiz de número negativo: {}.", format_real(x))
}

pub fn bad_input(expected: &str, line: &str) -> String {
    format!("Esperava {expected}, mas li \"{line}\".")
}

pub fn parse_int(line: &str, bits: u8, signed: bool) -> Option<i128> {
    line.trim().parse::<i128>().ok().filter(|v| *v >= int_min(bits, signed) && *v <= int_max(bits, signed))
}

pub fn parse_real(line: &str) -> Option<f64> {
    line.trim().replace(',', ".").parse::<f64>().ok()
}

pub fn parse_bool(line: &str) -> Option<bool> {
    match line.trim().to_lowercase().as_str() {
        "verdadeiro" | "v" | "sim" | "s" => Some(true),
        "falso" | "f" | "não" | "nao" | "n" => Some(false),
        _ => None,
    }
}

pub fn parse_char(line: &str) -> Option<char> {
    let mut c = line.trim().chars();
    let first = c.next()?;
    c.next().is_none().then_some(first)
}

/// "a, b e c".
pub fn join_e(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} e {last}", init.join(", ")),
    }
}
