//! O formatador da Lavra (`lavrac fmt`). Conservador por desenho, porque os poemas são
//! programas: nunca muda quebra de linha nem linha em branco, nem o interior de um token
//! (texto entre aspas, `8,5`, `1 000 000`). Normaliza só:
//!
//! - o recuo das linhas que abrem uma linha lógica: quatro espaços por bloco aberto;
//! - o recuo das linhas de continuação (enjambement), que acompanham a linha que abriram,
//!   mantendo a distância que tinham dela;
//! - o espaço entre tokens: um só, nenhum antes de `.`, `,`, `:`, `;` e `)` nem depois de
//!   `(`; o comentário do fim da linha fica a um espaço do código;
//! - tabs no recuo dos comentários, espaço no fim da linha e a quebra final.
//!
//! Texto com erro léxico não se formata: o formatador não mexe no que não entendeu.

use crate::diagnostics::{Diagnostic, Diagnostics};
use crate::lexer::{self, LineRole, TokenKind};

const INDENT: usize = 4;

/// A largura do recuo, com tab valendo quatro (como o léxico conta).
fn width(indent: &str) -> usize {
    indent.chars().map(|c| if c == '\t' { INDENT } else { 1 }).sum()
}

/// O texto formatado, ou os erros léxicos que impedem a formatação.
pub fn format(src: &str) -> Result<String, Vec<Diagnostic>> {
    let mut diags = Diagnostics::default();
    let tokens = lexer::tokenize(src, &mut diags);
    if diags.has_errors() {
        return Err(diags.list);
    }
    let roles = lexer::layout(src);
    // Os trechos dos tokens com texto, sem repetir as duas metades de uma contração.
    // (início, fim, pontuação, locução, número)
    let mut spans: Vec<(usize, usize, Option<char>, bool, bool)> = Vec::new();
    for t in &tokens {
        let (a, b) = (t.span.start as usize, t.span.end as usize);
        if a == b || spans.last().is_some_and(|s| s.0 == a) {
            continue;
        }
        let punct = match t.kind {
            TokenKind::Punct(c) => Some(c),
            _ => None,
        };
        // Locução ("maior que", "seguido de"): os espaços de dentro também se normalizam.
        let locution = matches!(&t.kind, TokenKind::Word(w) if w.contains(' '));
        let number = matches!(t.kind, TokenKind::Int(_) | TokenKind::Real(_));
        spans.push((a, b, punct, locution, number));
    }
    let mut out = String::with_capacity(src.len());
    let mut next = 0; // primeiro token ainda não usado
    // Fim de um token que atravessou a quebra de linha (locução partida no enjambement).
    let mut carry: Option<usize> = None;
    let mut offset = 0;
    // O recuo da linha que abriu a linha lógica: antes e depois de formatar.
    let (mut old_start, mut new_start) = (0usize, 0usize);
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    for (i, raw) in lines.iter().enumerate() {
        let line_start = offset;
        offset += raw.len();
        let ending = if raw.ends_with("\r\n") {
            "\r\n"
        } else if raw.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let line = &raw[..raw.len() - ending.len()];
        let content = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - content.len()];
        let role = roles.get(i).copied().unwrap_or(LineRole::Blank);
        let new_indent = match role {
            LineRole::Blank => {
                out.push_str(ending);
                continue;
            }
            LineRole::Comment => {
                out.push_str(&" ".repeat(width(indent)));
                out.push_str(content.trim_end());
                out.push_str(ending);
                continue;
            }
            LineRole::Start(depth) => {
                old_start = width(indent);
                new_start = depth * INDENT;
                new_start
            }
            LineRole::Continuation => (new_start + width(indent)).saturating_sub(old_start),
        };
        out.push_str(&" ".repeat(new_indent));
        // Os tokens desta linha, com um espaço entre eles (nenhum junto da pontuação).
        let line_end = line_start + line.len();
        let mut prev: Option<(usize, Option<char>)> = None;
        let mut prev_number = false;
        if let Some(b) = carry.take().filter(|&b| b > line_start + indent.len()) {
            let end = b.min(line_end);
            out.push_str(src[line_start + indent.len()..end].trim_start());
            prev = Some((end, None));
            if b > line_end {
                carry = Some(b);
            }
        }
        while next < spans.len() && spans[next].0 < line_end {
            let (a, b, punct, locution, number) = spans[next];
            if let Some((prev_end, prev_punct)) = prev {
                let tight = matches!(punct, Some('.' | ',' | ':' | ';' | ')')) || prev_punct == Some('(');
                if prev_number && number {
                    // Dois números: um espaço só os juntaria num número com milhar ("1 000").
                    out.push_str(&src[prev_end..a]);
                } else if !tight {
                    out.push(' ');
                }
            }
            prev_number = number;
            // A locução com a preposição contraída ("seguido do") termina no começo da
            // contração e leva o espaço junto: o espaço é do formatador.
            let text = src[a..b.min(line_end)].trim_end();
            if locution {
                out.push_str(&text.split([' ', '\t']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "));
            } else {
                out.push_str(text);
            }
            prev = Some((b.min(line_end), punct));
            if b > line_end {
                carry = Some(b);
            }
            next += 1;
        }
        // O comentário do fim da linha, a um espaço do código.
        let tail_from = prev.map_or(line_start + indent.len(), |(b, _)| b.min(line_end));
        let tail = src[tail_from..line_end].trim();
        if !tail.is_empty() {
            if prev.is_some() {
                out.push(' ');
            }
            out.push_str(tail);
        }
        out.push_str(ending);
    }
    // Uma quebra final, e nenhuma linha vazia sobrando no fim.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}
