//! O formatador: o corpus já está na forma canônica, e o que se estraga nele (recuo com
//! tabs, espaços sobrando entre tokens e no fim da linha) volta exatamente ao original. Em
//! todo caso, formatar não muda os tokens nem os comentários e é idempotente.

use lavra::diagnostics::Diagnostics;
use lavra::format::format;
use lavra::lexer::{self, TokenKind};
use std::path::{Path, PathBuf};

fn corpus(dirs: &[&str]) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in dirs {
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

/// Os tokens com a estrutura de blocos, sem as posições.
fn shape(src: &str) -> Vec<String> {
    lexer::tokenize(src, &mut Diagnostics::default())
        .into_iter()
        .map(|t| match t.kind {
            TokenKind::Word(w) => w,
            TokenKind::Text(s) => format!("{s:?}"),
            other => format!("{other:?}"),
        })
        .collect()
}

/// Os comentários, de linha inteira e do fim da linha (fora de texto entre aspas).
fn comments(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let t = l.trim();
            if t.to_lowercase().starts_with("nota:") {
                return Some(t.to_string());
            }
            let mut quoted = false;
            let b = t.as_bytes();
            for i in 0..b.len() {
                match b[i] {
                    b'"' => quoted = !quoted,
                    b'-' if !quoted && b.get(i + 1) == Some(&b'-') => return Some(t[i..].trim().to_string()),
                    _ => {}
                }
            }
            None
        })
        .collect()
}

/// Estraga o texto sem mudar o sentido: recuo com tab, espaço a mais entre tokens (fora
/// de texto entre aspas e de comentário) e espaço no fim da linha.
fn spoil(src: &str, seed: usize) -> String {
    let mut out = String::new();
    for (n, line) in src.split_inclusive('\n').enumerate() {
        let (body, end) = line.strip_suffix('\n').map_or((line, ""), |b| (b, "\n"));
        let content = body.trim_start_matches(' ');
        let indent = body.len() - content.len();
        // Comentário de linha inteira fica como está (o formatador não mexe dentro dele).
        if content.to_lowercase().starts_with("nota:") {
            out.push_str(line);
            continue;
        }
        if (n + seed).is_multiple_of(2) && indent.is_multiple_of(4) {
            out.push_str(&"\t".repeat(indent / 4));
        } else {
            out.push_str(&body[..indent]);
        }
        let mut quoted = false;
        let chars: Vec<char> = content.chars().collect();
        let mut comment = false;
        for (i, c) in chars.iter().enumerate() {
            if *c == '"' || *c == '\'' {
                quoted = !quoted;
            }
            if !quoted && *c == '-' && chars.get(i + 1) == Some(&'-') {
                comment = true;
            }
            out.push(*c);
            // Entre algarismos o espaço é do milhar ("1 000"): mexer nele muda o número.
            let digits = i > 0 && chars[i - 1].is_ascii_digit() && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit());
            if *c == ' ' && !quoted && !comment && !digits && (i + n + seed).is_multiple_of(3) {
                out.push_str("  ");
            }
        }
        if !content.is_empty() && (n + seed) % 4 == 1 {
            out.push_str("   ");
        }
        out.push_str(end);
    }
    out
}

#[test]
fn the_corpus_is_already_formatted() {
    // Os programas válidos; os errados de propósito podem estar fora da forma.
    let mut changed = Vec::new();
    for path in corpus(&["samples", "tests/programs/ok"]) {
        let src = std::fs::read_to_string(&path).unwrap();
        let Ok(f) = format(&src) else { continue };
        if f != src {
            let line = src.lines().zip(f.lines()).position(|(a, b)| a != b).map_or(0, |i| i + 1);
            changed.push(format!("{} (linha {line})", path.display()));
        }
    }
    assert!(changed.is_empty(), "o formatador mudaria:\n{}", changed.join("\n"));
}

#[test]
fn spoiled_programs_come_back_and_keep_their_tokens() {
    let mut failures = Vec::new();
    for path in corpus(&["samples", "tests/programs/ok", "tests/programs/err"]) {
        let src = std::fs::read_to_string(&path).unwrap();
        if format(&src).is_err() {
            continue;
        }
        for seed in 0..3 {
            let bad = spoil(&src, seed);
            let name = format!("{} (versão {seed})", path.display());
            let Ok(f) = format(&bad) else {
                failures.push(format!("{name}: não formatou"));
                continue;
            };
            // Só o que já estava na forma canônica tem de voltar a ela.
            if f != src && format(&src).ok().as_deref() == Some(src.as_str()) {
                let line = src.lines().zip(f.lines()).position(|(a, b)| a != b).map_or(0, |i| i + 1);
                failures.push(format!("{name}: não voltou ao original (linha {line})"));
            }
            if shape(&f) != shape(&bad) {
                failures.push(format!("{name}: mudou os tokens"));
            }
            if comments(&f) != comments(&bad) {
                failures.push(format!("{name}: mudou os comentários"));
            }
            if format(&f).ok().as_deref() != Some(f.as_str()) {
                failures.push(format!("{name}: formatar de novo muda"));
            }
        }
    }
    assert!(failures.is_empty(), "{} problemas:\n{}", failures.len(), failures.join("\n"));
}

#[test]
fn text_with_a_lexical_error_is_not_touched() {
    assert!(format("Para começar:\n    Escrevo 1 + 2.\n").is_err());
}
