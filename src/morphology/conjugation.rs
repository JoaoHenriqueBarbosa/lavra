//! Conjugação: 1ª pessoa do presente (executar), futuro do subjuntivo (condição) e
//! 3ª pessoa do presente (dados-sujeito e dicas).

use super::Number;

const IRREGULAR_FIRST_PERSON: &[(&str, &str)] = &[
    ("ser", "sou"),
    ("estar", "estou"),
    ("ter", "tenho"),
    ("ir", "vou"),
    ("vir", "venho"),
    ("fazer", "faço"),
    ("desfazer", "desfaço"),
    ("refazer", "refaço"),
    ("satisfazer", "satisfaço"),
    ("dizer", "digo"),
    ("trazer", "trago"),
    ("ler", "leio"),
    ("reler", "releio"),
    ("crer", "creio"),
    ("ver", "vejo"),
    ("rever", "revejo"),
    ("prever", "prevejo"),
    ("prover", "provejo"),
    ("medir", "meço"),
    ("pedir", "peço"),
    ("impedir", "impeço"),
    ("despedir", "despeço"),
    ("expedir", "expeço"),
    ("ouvir", "ouço"),
    ("perder", "perco"),
    ("poder", "posso"),
    ("saber", "sei"),
    ("querer", "quero"),
    ("requerer", "requeiro"),
    ("caber", "caibo"),
    ("valer", "valho"),
    ("dar", "dou"),
    ("haver", "hei"),
    ("conter", "contenho"),
    ("manter", "mantenho"),
    ("obter", "obtenho"),
    ("deter", "detenho"),
    ("reter", "retenho"),
    ("entreter", "entretenho"),
    ("intervir", "intervenho"),
    ("convir", "convenho"),
    ("provir", "provenho"),
    ("odiar", "odeio"),
    ("ansiar", "anseio"),
    ("remediar", "remedeio"),
    ("incendiar", "incendeio"),
    ("mediar", "medeio"),
    ("intermediar", "intermedeio"),
    ("rir", "rio"),
    ("sorrir", "sorrio"),
];

const IRREGULAR_SUBJUNCTIVE: &[(&str, &str)] = &[
    ("ser", "for"),
    ("ir", "for"),
    ("ter", "tiver"),
    ("estar", "estiver"),
    ("fazer", "fizer"),
    ("desfazer", "desfizer"),
    ("refazer", "refizer"),
    ("satisfazer", "satisfizer"),
    ("poder", "puder"),
    ("caber", "couber"),
    ("saber", "souber"),
    ("querer", "quiser"),
    ("trazer", "trouxer"),
    ("dizer", "disser"),
    ("vir", "vier"),
    ("ver", "vir"),
    ("rever", "revir"),
    ("prever", "previr"),
    ("haver", "houver"),
    ("dar", "der"),
    ("conter", "contiver"),
    ("manter", "mantiver"),
    ("obter", "obtiver"),
    ("deter", "detiver"),
    ("reter", "retiver"),
    ("intervir", "intervier"),
    ("convir", "convier"),
    ("provir", "provier"),
];

const IRREGULAR_THIRD_PERSON: &[(&str, &str, &str)] = &[
    ("ser", "é", "são"),
    ("estar", "está", "estão"),
    ("ter", "tem", "têm"),
    ("ir", "vai", "vão"),
    ("vir", "vem", "vêm"),
    ("dar", "dá", "dão"),
    ("ler", "lê", "leem"),
    ("ver", "vê", "veem"),
    ("crer", "crê", "creem"),
    ("haver", "há", "hão"),
    ("fazer", "faz", "fazem"),
    ("dizer", "diz", "dizem"),
    ("trazer", "traz", "trazem"),
    ("querer", "quer", "querem"),
    ("poder", "pode", "podem"),
    ("saber", "sabe", "sabem"),
    ("diminuir", "diminui", "diminuem"),
    ("incluir", "inclui", "incluem"),
    ("excluir", "exclui", "excluem"),
    ("possuir", "possui", "possuem"),
    ("conter", "contém", "contêm"),
    ("manter", "mantém", "mantêm"),
    ("obter", "obtém", "obtêm"),
];

fn lookup<'a>(table: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

pub fn is_infinitive(w: &str) -> bool {
    w.len() > 2 && (w.ends_with("ar") || w.ends_with("er") || w.ends_with("ir") || w.ends_with("or") || w.ends_with("ôr"))
}

/// 1ª pessoa do singular do presente do indicativo ("medir" → "meço").
pub fn first_person(inf: &str) -> Option<String> {
    if let Some(f) = lookup(IRREGULAR_FIRST_PERSON, inf) {
        return Some(f.to_string());
    }
    for suffix in ["por", "pôr"] {
        if let Some(stem) = inf.strip_suffix(suffix) {
            return Some(format!("{stem}ponho"));
        }
    }
    if let Some(stem) = inf.strip_suffix("ear") {
        return Some(format!("{stem}eio"));
    }
    if let Some(stem) = inf.strip_suffix("air") {
        return Some(format!("{stem}aio"));
    }
    let (stem, ending) = split_ending(inf)?;
    let mut r = stem.to_string();
    if ending == "ir" {
        r = alternate_vowel(&r);
    }
    if ending != "ar" {
        if let Some(s) = r.strip_suffix("gu") {
            r = format!("{s}g");
        } else if let Some(s) = r.strip_suffix('c') {
            r = format!("{s}ç");
        } else if let Some(s) = r.strip_suffix('g') {
            r = format!("{s}j");
        }
    }
    Some(format!("{r}o"))
}

/// Alternância vocálica dos verbos em -ir: transferir/transfiro, dormir/durmo.
fn alternate_vowel(stem: &str) -> String {
    for cluster in ["rt", "rv", "rn", "st", "nt", "gu", "r", "t", "d"] {
        if let Some(before) = stem.strip_suffix(cluster)
            && let Some(b) = before.strip_suffix('e')
        {
            return format!("{b}i{cluster}");
        }
    }
    for cluster in ["rm", "br", "ss", "l"] {
        if let Some(before) = stem.strip_suffix(cluster)
            && let Some(b) = before.strip_suffix('o')
        {
            return format!("{b}u{cluster}");
        }
    }
    stem.to_string()
}

fn split_ending(inf: &str) -> Option<(&str, &str)> {
    for e in ["ar", "er", "ir"] {
        if let Some(stem) = inf.strip_suffix(e)
            && !stem.is_empty()
        {
            return Some((stem, e));
        }
    }
    None
}

/// Futuro do subjuntivo ("ver" → "vir", "vierem" no plural).
pub fn future_subjunctive(inf: &str, n: Number) -> String {
    let sing = if let Some(f) = lookup(IRREGULAR_SUBJUNCTIVE, inf) {
        f.to_string()
    } else if let Some(stem) = inf.strip_suffix("por").or_else(|| inf.strip_suffix("pôr")) {
        format!("{stem}puser")
    } else {
        inf.to_string()
    };
    match n {
        Number::Singular => sing,
        Number::Plural => format!("{sing}em"),
    }
}

/// 3ª pessoa do presente do indicativo ("crescer" → "cresce", "crescem").
pub fn third_person(inf: &str, n: Number) -> Option<String> {
    if let Some((_, s, p)) = IRREGULAR_THIRD_PERSON.iter().find(|(k, _, _)| *k == inf) {
        return Some(match n {
            Number::Singular => s.to_string(),
            Number::Plural => p.to_string(),
        });
    }
    let (stem, ending) = split_ending(inf)?;
    let vowel = if ending == "ar" { "a" } else { "e" };
    Some(match n {
        Number::Singular => format!("{stem}{vowel}"),
        Number::Plural => format!("{stem}{vowel}m"),
    })
}

/// Infinitivo provável de uma forma na 3ª pessoa do singular, para dicas
/// ("escreve" → "escrever").
pub fn infinitive_from_third_person(w: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((k, _, _)) = IRREGULAR_THIRD_PERSON.iter().find(|(_, s, _)| *s == w) {
        out.push(k.to_string());
    }
    if let Some(stem) = w.strip_suffix('a') {
        out.push(format!("{stem}ar"));
    }
    if let Some(stem) = w.strip_suffix('e') {
        out.push(format!("{stem}er"));
        out.push(format!("{stem}ir"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_person_regular_and_irregular() {
        let cases = [
            ("somar", "somo"),
            ("escrever", "escrevo"),
            ("partir", "parto"),
            ("conhecer", "conheço"),
            ("proteger", "protejo"),
            ("erguer", "ergo"),
            ("rastrear", "rastreio"),
            ("transferir", "transfiro"),
            ("seguir", "sigo"),
            ("servir", "sirvo"),
            ("dormir", "durmo"),
            ("cobrir", "cubro"),
            ("compor", "componho"),
            ("medir", "meço"),
            ("ler", "leio"),
            ("perder", "perco"),
            ("odiar", "odeio"),
            ("render", "rendo"),
            ("arquivar", "arquivo"),
            ("acusar", "acuso"),
            ("responder", "respondo"),
        ];
        for (inf, expected) in cases {
            assert_eq!(first_person(inf).as_deref(), Some(expected), "{inf}");
        }
    }

    #[test]
    fn subjunctive() {
        assert_eq!(future_subjunctive("ser", Number::Singular), "for");
        assert_eq!(future_subjunctive("ser", Number::Plural), "forem");
        assert_eq!(future_subjunctive("ver", Number::Singular), "vir");
        assert_eq!(future_subjunctive("exceder", Number::Plural), "excederem");
        assert_eq!(future_subjunctive("compor", Number::Singular), "compuser");
        assert_eq!(future_subjunctive("conter", Number::Plural), "contiverem");
    }

    #[test]
    fn third() {
        assert_eq!(third_person("crescer", Number::Singular).as_deref(), Some("cresce"));
        assert_eq!(third_person("virar", Number::Plural).as_deref(), Some("viram"));
        assert_eq!(third_person("diminuir", Number::Plural).as_deref(), Some("diminuem"));
        assert_eq!(third_person("ser", Number::Plural).as_deref(), Some("são"));
    }
}
