//! Plural dos substantivos: liga "Para cada nota" a "as notas" e dá
//! o plural dos tipos definidos pelo usuário ("Uma data" → "datas").

use super::{has_accent, strip_accents};

pub fn pluralize(p: &str) -> String {
    if p.contains('-') {
        return p.split('-').map(pluralize).collect::<Vec<_>>().join("-");
    }
    let ends = |s: &str| p.ends_with(s);
    let cut = |n: usize| p.chars().take(p.chars().count() - n).collect::<String>();
    if ends("ão") {
        return format!("{}ões", cut(2));
    }
    if p.chars().last().is_some_and(|c| "aeiouáéíóúâêôã".contains(c)) {
        return format!("{p}s");
    }
    if ends("m") {
        return format!("{}ns", cut(1));
    }
    if ends("al") {
        return format!("{}is", cut(1));
    }
    if ends("el") {
        let stem = cut(2);
        return format!("{stem}{}", if has_accent(&stem) { "eis" } else { "éis" });
    }
    if ends("ol") {
        let stem = cut(2);
        return format!("{stem}{}", if has_accent(&stem) { "ois" } else { "óis" });
    }
    if ends("ul") {
        return format!("{}is", cut(1));
    }
    if ends("il") {
        let stem = cut(2);
        return if has_accent(&stem) { format!("{stem}eis") } else { format!("{}s", cut(1)) };
    }
    if ends("r") || ends("z") {
        return format!("{p}es");
    }
    if ends("ês") || ends("és") || ends("ás") || ends("ós") {
        let vowel: String = p.chars().rev().nth(1).map(|c| strip_accents(&c.to_string())).unwrap_or_default();
        return format!("{}{vowel}ses", cut(2));
    }
    if ends("s") || ends("x") {
        return p.to_string();
    }
    format!("{p}s")
}

#[cfg(test)]
mod tests {
    use super::pluralize;

    #[test]
    fn plurals() {
        let cases = [
            ("nota", "notas"),
            ("canção", "canções"),
            ("item", "itens"),
            ("real", "reais"),
            ("papel", "papéis"),
            ("farol", "faróis"),
            ("azul", "azuis"),
            ("barril", "barris"),
            ("fóssil", "fósseis"),
            ("orador", "oradores"),
            ("mês", "meses"),
            ("lápis", "lápis"),
            ("segunda-feira", "segundas-feiras"),
            ("lógico", "lógicos"),
            ("caractere", "caracteres"),
        ];
        for (s, p) in cases {
            assert_eq!(pluralize(s), p, "{s}");
        }
    }
}
