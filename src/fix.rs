//! As correções que o compilador sabe fazer, aplicadas ao texto. Servem ao `lavrac fix`,
//! ao "corrigir tudo" do editor e ao rename, que completa as concordâncias com elas.
//!
//! Uma correção pode revelar outra (a análise volta a ler o texto corrigido), então elas
//! se aplicam em voltas, e as edições de todas as voltas se acumulam num `Edited`, que
//! sabe de que trecho do original veio cada pedaço: no fim, o editor recebe trocas
//! pequenas sobre o texto que tem, e não o arquivo inteiro.

use crate::diagnostics::{Diagnostic, Edit};
use crate::span::Span;

/// Quantas voltas de correção, no máximo: cada volta só aplica o que a anterior revelou.
const ROUNDS: usize = 8;

/// Um texto com edições acumuladas, guardadas sobre o original: em ordem, sem
/// sobreposição.
#[derive(Clone, Debug)]
pub struct Edited {
    original: String,
    edits: Vec<Edit>,
}

impl Edited {
    pub fn new(original: &str) -> Edited {
        Edited { original: original.to_string(), edits: Vec::new() }
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    /// As edições sobre o texto original.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// O texto com as edições.
    pub fn text(&self) -> String {
        let mut out = String::with_capacity(self.original.len());
        let mut at = 0usize;
        for e in &self.edits {
            out.push_str(&self.original[at..e.span.start as usize]);
            out.push_str(&e.text);
            at = e.span.end as usize;
        }
        out.push_str(&self.original[at..]);
        out
    }

    /// Onde cada edição está no texto atual: (início, fim).
    fn current_ranges(&self) -> Vec<(u32, u32)> {
        let mut delta: i64 = 0;
        self.edits
            .iter()
            .map(|e| {
                let start = (e.span.start as i64 + delta) as u32;
                delta += e.text.len() as i64 - (e.span.end - e.span.start) as i64;
                (start, start + e.text.len() as u32)
            })
            .collect()
    }

    /// Onde uma posição do original está no texto atual. Dentro de um trecho editado,
    /// vale o começo da edição.
    pub fn map(&self, pos: u32) -> u32 {
        let mut delta: i64 = 0;
        for (e, (start, _)) in self.edits.iter().zip(self.current_ranges()) {
            if pos < e.span.start || (pos == e.span.start && e.span.is_empty()) {
                break;
            }
            if pos < e.span.end {
                return start;
            }
            delta += e.text.len() as i64 - (e.span.end - e.span.start) as i64;
        }
        (pos as i64 + delta) as u32
    }

    /// Aplica edições dadas sobre o texto atual, sem sobreposição entre elas.
    pub fn apply(&mut self, edits: &[Edit]) {
        let mut sorted: Vec<&Edit> = edits.iter().collect();
        sorted.sort_by_key(|e| (e.span.start, e.span.end));
        // De trás para a frente: as posições das que faltam não se movem.
        for e in sorted.into_iter().rev() {
            self.apply_one(e.span.start, e.span.end, &e.text);
        }
    }

    /// Uma edição sobre o texto atual. As edições anteriores que ela toca (ou encosta)
    /// viram uma só com ela, sobre o trecho do original que as duas cobrem.
    fn apply_one(&mut self, s: u32, e: u32, text: &str) {
        let ranges = self.current_ranges();
        let touched: Vec<usize> = (0..self.edits.len()).filter(|&i| ranges[i].0 <= e && s <= ranges[i].1).collect();
        // Uma posição atual fora das edições volta ao original pelo deslocamento das que
        // ficam antes dela.
        let back = |pos: u32| -> u32 {
            let mut delta: i64 = 0;
            for (i, ed) in self.edits.iter().enumerate() {
                if ranges[i].1 > pos {
                    break;
                }
                delta += ed.text.len() as i64 - (ed.span.end - ed.span.start) as i64;
            }
            (pos as i64 - delta) as u32
        };
        let (mut from, mut to) = (back(s), back(e));
        let (mut prefix, mut suffix) = ("", "");
        if let (Some(&f), Some(&l)) = (touched.first(), touched.last()) {
            if ranges[f].0 <= s {
                prefix = &self.edits[f].text[..(s - ranges[f].0) as usize];
                from = self.edits[f].span.start;
            }
            if e <= ranges[l].1 {
                suffix = &self.edits[l].text[(e - ranges[l].0) as usize..];
                to = self.edits[l].span.end;
            }
        }
        let merged = Edit { span: Span::new(from as usize, to as usize), text: format!("{prefix}{text}{suffix}") };
        for &i in touched.iter().rev() {
            self.edits.remove(i);
        }
        self.edits.push(merged);
        self.edits.sort_by_key(|x| (x.span.start, x.span.end));
        // Uma edição que deixa o original como estava não é edição.
        let original = &self.original;
        self.edits.retain(|x| original.get(x.span.start as usize..x.span.end as usize) != Some(x.text.as_str()));
    }
}

/// As correções seguras de um diagnóstico (a primeira, que é a preferida).
fn safe_edits(d: &Diagnostic) -> Option<&[Edit]> {
    d.fixes.iter().find(|f| f.safe).map(|f| f.edits.as_slice())
}

/// Escolhe, na ordem do texto, as correções que não se sobrepõem.
fn non_overlapping<'a>(fixes: impl Iterator<Item = &'a [Edit]>) -> Vec<Edit> {
    let mut chosen: Vec<Edit> = Vec::new();
    let mut fixes: Vec<&[Edit]> = fixes.filter(|f| !f.is_empty()).collect();
    fixes.sort_by_key(|f| f.iter().map(|e| e.span.start).min());
    for f in fixes {
        let overlaps = f.iter().any(|e| {
            chosen.iter().any(|c| e.span.start < c.span.end.max(c.span.start + 1) && c.span.start < e.span.end.max(e.span.start + 1))
        });
        if !overlaps {
            chosen.extend(f.iter().cloned());
        }
    }
    chosen
}

/// Aplica, volta após volta, as correções seguras dos diagnósticos que `pick` escolhe,
/// até não sobrar nenhuma. `pick` recebe o diagnóstico e o texto em que ele foi achado.
pub fn fix_with(edited: &mut Edited, pick: impl Fn(&Diagnostic, &Edited) -> bool) -> usize {
    let mut applied = 0;
    for _ in 0..ROUNDS {
        let text = edited.text();
        let a = crate::analyze(&text);
        let edits = non_overlapping(a.diagnostics.iter().filter(|d| pick(d, edited)).filter_map(safe_edits));
        if edits.is_empty() {
            break;
        }
        applied += a.diagnostics.iter().filter(|d| pick(d, edited) && safe_edits(d).is_some()).count();
        edited.apply(&edits);
    }
    applied
}

/// Todas as correções seguras do texto (`lavrac fix`, "corrigir tudo" no editor), e
/// quantas foram.
pub fn fix_all(src: &str) -> (Edited, usize) {
    let mut edited = Edited::new(src);
    let applied = fix_with(&mut edited, |_, _| true);
    (edited, applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(start: usize, end: usize, text: &str) -> Edit {
        Edit { span: Span::new(start, end), text: text.into() }
    }

    #[test]
    fn edits_accumulate_over_the_original() {
        let mut e = Edited::new("o saldo do saldo");
        e.apply(&[edit(0, 1, "a"), edit(2, 7, "reserva"), edit(8, 10, "da"), edit(11, 16, "reserva")]);
        assert_eq!(e.text(), "a reserva da reserva");
        // Uma segunda volta mexe dentro de uma edição anterior e fora delas.
        e.apply(&[edit(2, 9, "conta"), edit(10, 12, "na")]);
        assert_eq!(e.text(), "a conta na reserva");
        let mut back = Edited::new(e.original());
        back.apply(e.edits());
        assert_eq!(back.text(), "a conta na reserva");
        assert_eq!(e.map(11), 11);
        assert_eq!(e.map(3), 2);
    }

    #[test]
    fn insertion_and_deletion() {
        let mut e = Edited::new("Escrevo o saldo");
        e.apply(&[edit(15, 15, ".")]);
        e.apply(&[edit(0, 8, "")]);
        assert_eq!(e.text(), "o saldo.");
        assert_eq!(e.edits().len(), 2);
    }
}
