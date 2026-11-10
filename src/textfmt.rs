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

/// O motivo de uma operação de arquivo não ter dado certo, em português e sem depender do
/// texto do sistema (que muda de máquina para máquina).
pub fn io_reason(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind::*;
    match kind {
        NotFound => "não existe",
        PermissionDenied => "não há permissão",
        AlreadyExists => "já existe",
        IsADirectory => "é uma pasta",
        NotADirectory => "não é uma pasta",
        InvalidData => "não é texto (UTF-8)",
        _ => "o sistema recusou",
    }
}

/// A mensagem da falha de uma operação de arquivo: "Não consegui carregar "x.txt": não existe."
pub fn file_failure(action: &str, path: &str, kind: std::io::ErrorKind) -> String {
    format!("Não consegui {action} \"{path}\": {}.", io_reason(kind))
}

/// "carregar o arquivo".
pub fn file_load(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| file_failure("carregar", path, e.kind()))
}

/// "gravar o texto no arquivo" e "acrescentar o texto ao arquivo".
pub fn file_save(text: &str, path: &str, append: bool) -> Result<(), String> {
    use std::io::Write;
    let action = if append { "acrescentar a" } else { "gravar em" };
    let mut options = std::fs::OpenOptions::new();
    options.create(true).write(true);
    if append {
        options.append(true);
    } else {
        options.truncate(true);
    }
    options.open(path).and_then(|mut f| f.write_all(text.as_bytes())).map_err(|e| file_failure(action, path, e.kind()))
}

/// "apagar o arquivo".
pub fn file_remove(path: &str) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| file_failure("apagar", path, e.kind()))
}

/// "o arquivo existir".
pub fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

/// "listar a pasta": os nomes, em ordem.
pub fn dir_list(path: &str) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(path).map_err(|e| file_failure("listar", path, e.kind()))?;
    let mut names: Vec<String> = entries.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    Ok(names)
}

/// O instante de agora: os milissegundos desde o começo de 1970.
pub fn now_millis() -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
    }
    #[cfg(target_arch = "wasm32")]
    {
        0
    }
}

/// Quantos segundos o horário do lugar está à frente do universal, no instante dado.
#[cfg(all(unix, not(target_arch = "wasm32")))]
fn local_offset(epoch_seconds: i64) -> i64 {
    // O `struct tm` do Linux (glibc e musl): nove inteiros, o deslocamento e o nome do fuso.
    #[repr(C)]
    struct Tm {
        fields: [i32; 9],
        gmtoff: std::ffi::c_long,
        zone: *const std::ffi::c_char,
    }
    unsafe extern "C" {
        fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    }
    let mut tm = Tm { fields: [0; 9], gmtoff: 0, zone: std::ptr::null() };
    // SAFETY: os dois ponteiros apontam para valores vivos, e a função só os lê e preenche.
    let filled = unsafe { localtime_r(&epoch_seconds, &mut tm) };
    if filled.is_null() { 0 } else { tm.gmtoff as i64 }
}

#[cfg(not(all(unix, not(target_arch = "wasm32"))))]
fn local_offset(_epoch_seconds: i64) -> i64 {
    0
}

/// As partes de um instante no horário do lugar: ano, mês, dia, hora, minuto e segundo.
pub fn civil(millis: i64) -> [i64; 6] {
    let utc = millis.div_euclid(1000);
    let local = utc.saturating_add(local_offset(utc));
    let days = local.div_euclid(86_400);
    let rest = local.rem_euclid(86_400);
    // Dos dias desde 1970 ao calendário (o algoritmo de Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    [year, month, day, rest / 3_600, rest % 3_600 / 60, rest % 60]
}

/// "datar": o instante por extenso, "10/11/2026 14:57:03".
pub fn date_text(millis: i64) -> String {
    let [y, mo, d, h, mi, s] = civil(millis);
    format!("{d:02}/{mo:02}/{y} {h:02}:{mi:02}:{s:02}")
}

/// "aguardar": para pelo tempo dado, em milissegundos.
pub fn sleep_millis(millis: i64) {
    #[cfg(not(target_arch = "wasm32"))]
    if millis > 0 {
        std::thread::sleep(std::time::Duration::from_millis(millis as u64));
    }
    #[cfg(target_arch = "wasm32")]
    let _ = millis;
}

/// O item de uma chave que o mapa não tem; `shown` é a chave como se escreve no programa.
pub fn missing_key(shown: &str) -> String {
    format!("Não há item {shown} no mapa.")
}

/// A chave como se escreve no programa: o texto entre aspas, o caractere entre aspas simples.
pub fn quoted_text(text: &str) -> String {
    format!("\"{text}\"")
}

pub fn quoted_char(c: char) -> String {
    format!("'{c}'")
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
