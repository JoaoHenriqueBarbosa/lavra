//! Rede de segurança do compilador: cada programa do corpus é mutilado de formas
//! determinísticas (cortado no fim de cada linha e de várias palavras, sem uma
//! palavra, sem uma linha, com uma linha repetida, com um caractere a menos ou um sinal
//! a mais) e a análise tem que sobreviver a todas as versões:
//!
//! - nunca entra em pânico;
//! - termina em tempo limitado;
//! - todo diagnóstico aponta para dentro do texto, em fronteira de caractere;
//! - duas rodadas sobre o mesmo texto dão os mesmos diagnósticos.
//!
//! É o que permite ao servidor de linguagem analisar o programa enquanto ele é escrito.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

/// Onde cada pânico aconteceu, por thread: o gancho guarda, a análise relata.
static PANIC_AT: Mutex<Vec<(std::thread::ThreadId, String)>> = Mutex::new(Vec::new());

/// Tempo máximo para analisar uma versão (compilação de teste, sem otimização).
const LIMIT: Duration = Duration::from_secs(10);

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["tests/programs/ok", "tests/programs/err", "samples"] {
        for e in std::fs::read_dir(root.join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "lav") {
                files.push(p);
            }
        }
    }
    files.sort();
    files
}

/// Gerador pseudoaleatório pequeno e determinístico (xorshift), semeado pelo nome do arquivo.
struct Rng(u64);

impl Rng {
    fn new(seed: &str) -> Rng {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in seed.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
        }
        Rng(h | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Fronteiras de caractere do texto, para cortar sem partir um acento.
fn boundaries(text: &str) -> Vec<usize> {
    text.char_indices().map(|(i, _)| i).chain(std::iter::once(text.len())).collect()
}

/// Intervalos (início, fim) das palavras separadas por espaço.
fn words(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(i),
            (true, Some(s)) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, text.len()));
    }
    out
}

/// Pega no máximo `n` itens espalhados por toda a lista.
fn spread<T: Copy>(items: &[T], n: usize) -> Vec<T> {
    if items.len() <= n {
        return items.to_vec();
    }
    (0..n).map(|k| items[k * items.len() / n]).collect()
}

fn variants(name: &str, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rng = Rng::new(name);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let ws = words(text);

    // Cortes no fim de cada linha e depois de várias palavras.
    let mut acc = 0;
    for l in &lines {
        acc += l.len();
        out.push(text[..acc].to_string());
        out.push(text[..acc].trim_end_matches('\n').to_string());
    }
    for &(_, end) in &spread(&ws, 60) {
        out.push(text[..end].to_string());
    }
    // Cortes no meio de palavras (a palavra pela metade é o caso comum no editor).
    for &(s, e) in &spread(&ws, 30) {
        let b = boundaries(&text[s..e]);
        let mid = s + b[b.len() / 2];
        out.push(text[..mid].to_string());
    }
    // Sem uma palavra.
    for &(s, e) in &spread(&ws, 60) {
        out.push(format!("{}{}", &text[..s], &text[e..]));
    }
    // Sem uma linha, e com uma linha repetida.
    for k in 0..lines.len() {
        let mut v: Vec<&str> = lines.clone();
        v.remove(k);
        out.push(v.concat());
    }
    for k in spread(&(0..lines.len()).collect::<Vec<_>>(), 20) {
        let mut v: Vec<&str> = lines.clone();
        v.insert(k, lines[k]);
        out.push(v.concat());
    }
    // Um caractere a menos, e um sinal a mais em lugar qualquer.
    let b = boundaries(text);
    for _ in 0..30 {
        let k = rng.below(b.len().saturating_sub(1));
        out.push(format!("{}{}", &text[..b[k]], &text[b[k + 1]..]));
    }
    const SIGNS: &[&str] = &[".", ":", ",", "(", ")", "\"", "'", "\n", "    ", "\t", "-", "--", "0", "é", "o", "da"];
    for _ in 0..30 {
        let k = b[rng.below(b.len())];
        let s = SIGNS[rng.below(SIGNS.len())];
        out.push(format!("{}{s}{}", &text[..k], &text[k..]));
    }
    // Duas palavras trocadas de lugar.
    for _ in 0..15 {
        if ws.len() < 2 {
            break;
        }
        let i = rng.below(ws.len() - 1);
        let (a, b2) = (ws[i], ws[i + 1]);
        out.push(format!("{}{}{}{}{}", &text[..a.0], &text[b2.0..b2.1], &text[a.1..b2.0], &text[a.0..a.1], &text[b2.1..]));
    }
    out.sort();
    out.dedup();
    out
}

/// Os diagnósticos de uma análise, em texto, e o primeiro problema de posição achado.
fn analyze(text: &str) -> (String, Option<String>) {
    let mut report = String::new();
    let mut bad = None;
    let diags = lavra::analyze(text).diagnostics;
    for d in &diags {
        let mut spans = vec![d.span];
        spans.extend(d.labels.iter().map(|l| l.span));
        for s in spans {
            let (a, b) = (s.start as usize, s.end as usize);
            if a > b || b > text.len() || !text.is_char_boundary(a) || !text.is_char_boundary(b) {
                bad.get_or_insert_with(|| format!("span {a}..{b} fora do texto ({} bytes) em \"{}\"", text.len(), d.message));
            }
        }
        report.push_str(&format!("{:?} {} {:?}\n", d.category, d.message, d.span));
    }
    (report, bad)
}

#[test]
fn mutilated_corpus_never_breaks_the_analysis() {
    // Os pânicos esperados aqui são pegos e relatados; o gancho padrão só poluiria a saída.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|info| {
        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        PANIC_AT.lock().unwrap().push((std::thread::current().id(), at));
    }));

    // Falhas agrupadas pelo problema, com o menor texto que o provoca.
    let mut failures: BTreeMap<String, (usize, String, String)> = BTreeMap::new();
    let mut checked = 0usize;
    for path in corpus() {
        let text = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        for v in variants(&name, &text) {
            checked += 1;
            let (tx, rx) = mpsc::channel();
            let input = v.clone();
            std::thread::Builder::new()
                .stack_size(256 << 20)
                .spawn(move || {
                    let r = catch_unwind(AssertUnwindSafe(|| {
                        let first = analyze(&input);
                        let second = analyze(&input);
                        (first, second)
                    }));
                    let _ = tx.send(r.map_err(|e| {
                        let msg = e
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "pânico sem mensagem".into());
                        let me = std::thread::current().id();
                        let at = PANIC_AT.lock().unwrap().iter().rev().find(|(t, _)| *t == me).map(|(_, a)| a.clone());
                        format!("{} ({msg})", at.unwrap_or_default())
                    }));
                })
                .unwrap();
            let problem = match rx.recv_timeout(LIMIT) {
                Err(_) => Some(format!("não terminou em {} s", LIMIT.as_secs())),
                Ok(Err(msg)) => Some(format!("pânico: {msg}")),
                Ok(Ok(((r1, bad), (r2, _)))) => bad.or_else(|| (r1 != r2).then(|| "duas rodadas deram diagnósticos diferentes".into())),
            };
            if let Some(p) = problem {
                let key = if p.starts_with("span") { "span fora do texto".to_string() } else { p };
                let e = failures.entry(key).or_insert((0, name.clone(), v.clone()));
                e.0 += 1;
                if v.len() < e.2.len() {
                    e.1 = name.clone();
                    e.2 = v.clone();
                }
            }
        }
    }
    std::panic::set_hook(hook);
    assert!(checked > 1000, "o corpus gerou poucas versões ({checked})");
    if !failures.is_empty() {
        let total: usize = failures.values().map(|f| f.0).sum();
        let report: Vec<String> = failures
            .iter()
            .map(|(p, (n, name, v))| format!("{p}: {n} versões; a menor vem de {name}:\n----- texto -----\n{v}\n-----------------"))
            .collect();
        panic!("{total} de {checked} versões quebraram a análise, em {} problemas:\n\n{}", failures.len(), report.join("\n\n"));
    }
}
