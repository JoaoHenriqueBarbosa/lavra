//! Conjugação: 1ª pessoa do presente (executar), futuro do subjuntivo (condição),
//! 3ª pessoa do presente (dados-sujeito) e infinitivo pessoal ("o fato de ... serem").

mod first_person;
mod subjunctive;
mod third_person;

use super::Number;

pub use first_person::first_person;

pub fn is_infinitive(w: &str) -> bool {
    w == "ir"
        || w.chars().count() > 2 && (w.ends_with("ar") || w.ends_with("er") || w.ends_with("ir") || w.ends_with("or") || w.ends_with("ôr"))
}

/// Futuro do subjuntivo ("ver" → "vir"; "vierem" no plural).
pub fn future_subjunctive(inf: &str, n: Number) -> String {
    let sing = subjunctive::future_subjunctive(inf);
    match n {
        Number::Singular => sing,
        Number::Plural => plural_em(&sing),
    }
}

/// Infinitivo pessoal: "ser" / "serem".
pub fn personal_infinitive(inf: &str, n: Number) -> String {
    match n {
        Number::Singular => inf.to_string(),
        Number::Plural => plural_em(inf),
    }
}

/// Plural do subjuntivo e do infinitivo pessoal: "-em", com o acento do hiato
/// ("cair" → "caírem", "construir" → "construírem"; mas "seguir" → "seguirem").
fn plural_em(sing: &str) -> String {
    if let Some(stem) = sing.strip_suffix("ir")
        && let Some(v) = stem.chars().last()
        && (v == 'a' || (v == 'u' && !stem.ends_with("gu") && !stem.ends_with("qu")))
    {
        return format!("{stem}írem");
    }
    format!("{sing}em")
}

/// 3ª pessoa do presente do indicativo ("crescer" → "cresce", "crescem").
pub fn third_person(inf: &str, n: Number) -> Option<String> {
    let (s, p) = third_person::third_person(inf)?;
    Some(match n {
        Number::Singular => s,
        Number::Plural => p,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plural_with_hiatus() {
        assert_eq!(future_subjunctive("cair", Number::Plural), "caírem");
        assert_eq!(future_subjunctive("construir", Number::Plural), "construírem");
        assert_eq!(future_subjunctive("seguir", Number::Plural), "seguirem");
        assert_eq!(future_subjunctive("ver", Number::Plural), "virem");
        assert_eq!(personal_infinitive("sair", Number::Plural), "saírem");
        assert_eq!(personal_infinitive("ser", Number::Plural), "serem");
    }
}
