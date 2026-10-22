//! Morfologia do português que a Lavra usa: gênero, número, artigos, contrações,
//! conjugação, plural, flexão de adjetivo e particípio, ordinais.

pub mod adjectives;
pub mod nouns;
pub mod verbs;

pub use nouns::pluralize;
pub use verbs::{first_person, future_subjunctive, personal_infinitive, third_person};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gender {
    Masculine,
    Feminine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Number {
    Singular,
    Plural,
}

impl Gender {
    pub fn describe(self) -> &'static str {
        match self {
            Gender::Masculine => "masculino",
            Gender::Feminine => "feminino",
        }
    }
}

impl Number {
    pub fn describe(self) -> &'static str {
        match self {
            Number::Singular => "singular",
            Number::Plural => "plural",
        }
    }
}

pub fn describe_gn(g: Gender, n: Number) -> String {
    format!("{} {}", g.describe(), n.describe())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Article {
    pub definite: bool,
    pub gender: Gender,
    pub number: Number,
}

pub fn article(word: &str) -> Option<Article> {
    crate::lexicon::ARTICLES.iter().find(|a| a.0 == word).map(|&(_, definite, gender, number)| Article { definite, gender, number })
}

pub fn definite_article(g: Gender, n: Number) -> &'static str {
    match (g, n) {
        (Gender::Masculine, Number::Singular) => "o",
        (Gender::Feminine, Number::Singular) => "a",
        (Gender::Masculine, Number::Plural) => "os",
        (Gender::Feminine, Number::Plural) => "as",
    }
}

pub fn indefinite_article(g: Gender, n: Number) -> &'static str {
    match (g, n) {
        (Gender::Masculine, Number::Singular) => "um",
        (Gender::Feminine, Number::Singular) => "uma",
        (Gender::Masculine, Number::Plural) => "uns",
        (Gender::Feminine, Number::Plural) => "umas",
    }
}

/// Contrações obrigatórias e preposições: as tabelas ficam no léxico.
pub use crate::lexicon::{CONTRACTIONS, PREPOSITIONS};

pub fn split_contraction(word: &str) -> Option<(&'static str, &'static str, &'static str)> {
    CONTRACTIONS.iter().find(|(c, _, _)| *c == word).copied()
}

/// A forma contraída de preposição + artigo, se a língua exige contrair.
pub fn contract(prep: &str, art: &str) -> Option<&'static str> {
    CONTRACTIONS.iter().find(|(_, p, a)| *p == prep && *a == art).map(|(c, _, _)| *c)
}

/// "de" + "o" = "do"; sem contração possível, junta com espaço.
pub fn join_prep(prep: &str, art: &str) -> String {
    match contract(prep, art) {
        Some(c) => c.to_string(),
        None => format!("{prep} {art}"),
    }
}

pub fn is_preposition(w: &str) -> bool {
    PREPOSITIONS.contains(&w)
}

/// Flexão de adjetivo ou particípio. A base vem escrita no gênero `g0`.
pub fn inflect_adjective(base: &str, g0: Gender, g: Gender, n: Number) -> String {
    let f = match (g0, g) {
        (Gender::Masculine, Gender::Feminine) => adjectives::feminine(base),
        (Gender::Feminine, Gender::Masculine) => adjectives::masculine(base),
        _ => base.to_string(),
    };
    if n == Number::Plural { pluralize(&f) } else { f }
}

/// Lê uma forma flexionada de um adjetivo terminado em -o (emprestado, vazio...).
/// Devolve o gênero e o número, se a palavra for uma flexão de `base_masc_sing`.
pub fn adjective_form(word: &str, base_masc_sing: &str) -> Option<(Gender, Number)> {
    for g in [Gender::Masculine, Gender::Feminine] {
        for n in [Number::Singular, Number::Plural] {
            if inflect_adjective(base_masc_sing, Gender::Masculine, g, n) == word {
                return Some((g, n));
            }
        }
    }
    None
}

/// Ordinais de primeiro a décimo, e último. Devolve (posição, gênero, número); o último
/// tem posição 0.
pub fn ordinal(word: &str) -> Option<(u32, Gender, Number)> {
    for (stem, value) in crate::lexicon::ORDINALS {
        if let Some(rest) = word.strip_prefix(stem) {
            let (g, n) = match rest {
                "o" => (Gender::Masculine, Number::Singular),
                "a" => (Gender::Feminine, Number::Singular),
                "os" => (Gender::Masculine, Number::Plural),
                "as" => (Gender::Feminine, Number::Plural),
                _ => continue,
            };
            return Some((*value, g, n));
        }
    }
    None
}

pub fn ordinal_word(value: u32, g: Gender) -> String {
    let stem = crate::lexicon::ORDINALS.iter().find(|o| o.1 == value.min(10)).map_or("décim", |o| o.0);
    format!("{stem}{}", if g == Gender::Feminine { "a" } else { "o" })
}

/// Tira acentos e cedilha (para comparar palavras ao sugerir correções).
pub fn strip_accents(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

pub fn has_accent(s: &str) -> bool {
    s.chars().any(|c| "áéíóúâêôàãõ".contains(c))
}

/// Gênero provável de um substantivo solto, quando nenhum artigo o fixou.
pub fn guess_gender(word: &str) -> Gender {
    if word.ends_with('a') || word.ends_with("ção") || word.ends_with("dade") || word.ends_with("agem") {
        Gender::Feminine
    } else {
        Gender::Masculine
    }
}

/// Símbolo: uma letra minúscula com dígitos opcionais, sem artigo. "a", "e" e "o"
/// sozinhos são artigo e conjunção, então não servem de símbolo (mas "a1" serve).
pub fn is_symbol(w: &str) -> bool {
    let mut chars = w.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let rest: Vec<char> = chars.collect();
    if !rest.iter().all(|c| c.is_ascii_digit()) {
        return false;
    }
    !(rest.is_empty() && matches!(first, 'a' | 'e' | 'o'))
}

/// Distância de edição, para sugerir a palavra parecida.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn articles_and_contractions() {
        assert_eq!(article("as").unwrap().gender, Gender::Feminine);
        assert_eq!(split_contraction("pelas"), Some(("pelas", "por", "as")));
        assert_eq!(contract("em", "um"), Some("num"));
        assert_eq!(join_prep("para", "o"), "para o");
    }

    #[test]
    fn adjectives() {
        assert_eq!(inflect_adjective("emprestado", Gender::Masculine, Gender::Feminine, Number::Plural), "emprestadas");
        assert_eq!(inflect_adjective("português", Gender::Masculine, Gender::Feminine, Number::Singular), "portuguesa");
        assert_eq!(adjective_form("confiadas", "confiado"), Some((Gender::Feminine, Number::Plural)));
        assert_eq!(adjective_form("vazio", "vazio"), Some((Gender::Masculine, Number::Singular)));
    }

    #[test]
    fn ordinals() {
        assert_eq!(ordinal("terceira"), Some((3, Gender::Feminine, Number::Singular)));
        assert_eq!(ordinal("último"), Some((0, Gender::Masculine, Number::Singular)));
        assert_eq!(ordinal("primo"), None);
    }

    #[test]
    fn symbols() {
        assert!(is_symbol("x"));
        assert!(is_symbol("a1"));
        assert!(!is_symbol("a"));
        assert!(!is_symbol("xy"));
    }
}
