//! 3ª pessoa do presente do indicativo, singular e plural: a forma dos dados-sujeito
//! ("O saldo cresce", "As notas crescem").
//!
//! Arquivo autocontido (sem dependências do resto do crate), para poder ser testado
//! sozinho contra o dicionário inteiro (`tools/morphology`, `check_third_person.py`).
//!
//! A ordem de decisão é: verbos sem 3ª pessoa, tabela de verbos exatos, famílias por
//! terminação (que valem para qualquer prefixo), hiatos acentuados e, por fim, a
//! regra regular. Grafia do Acordo Ortográfico de 1990 ("leem", "apoia", "argui").

/// Verbos sem 3ª pessoa do presente: só se conjugam nas formas arrizotônicas
/// ("falimos", "precavemos", "reavemos"). "sobresser" entra aqui por ser duvidoso (o
/// Verbiste dá "sobresé", sem o prefixo inteiro). Casos de borda, a decidir.
const DEFECTIVE: &[&str] = &["falir", "precaver", "desprecaver", "reaver", "remir", "sobresser"];

/// Verbos tratados um a um, porque a terminação deles é compartilhada por verbos
/// regulares ("ter" e "bater", "vir" e "servir", "ver" e "mover", "sumir" e "assumir").
const IRREGULAR_THIRD_PERSON: &[(&str, &str, &str)] = &[
    ("ser", "é", "são"),
    ("estar", "está", "estão"),
    ("sobestar", "sobestá", "sobestão"),
    ("sobreestar", "sobreestá", "sobreestão"),
    ("sobrestar", "sobrestá", "sobrestão"),
    ("ir", "vai", "vão"),
    ("dar", "dá", "dão"),
    ("desdar", "desdá", "desdão"),
    ("redar", "redá", "redão"),
    ("haver", "há", "hão"),
    // ter e derivados
    ("ter", "tem", "têm"),
    ("abster", "abstém", "abstêm"),
    ("ater", "atém", "atêm"),
    ("conter", "contém", "contêm"),
    ("deter", "detém", "detêm"),
    ("entreter", "entretém", "entretêm"),
    ("manter", "mantém", "mantêm"),
    ("obter", "obtém", "obtêm"),
    ("reter", "retém", "retêm"),
    ("suster", "sustém", "sustêm"),
    // vir e derivados
    ("vir", "vem", "vêm"),
    ("advir", "advém", "advêm"),
    ("avir", "avém", "avêm"),
    ("contravir", "contravém", "contravêm"),
    ("convir", "convém", "convêm"),
    ("desavir", "desavém", "desavêm"),
    ("desconvir", "desconvém", "desconvêm"),
    ("devir", "devém", "devêm"),
    ("intervir", "intervém", "intervêm"),
    ("obvir", "obvém", "obvêm"),
    ("provir", "provém", "provêm"),
    ("reconvir", "reconvém", "reconvêm"),
    ("revir", "revém", "revêm"),
    ("sobrevir", "sobrevém", "sobrevêm"),
    // ver, prover, ler, crer e derivados: "-eem" sem circunflexo depois do Acordo
    ("ver", "vê", "veem"),
    ("antever", "antevê", "anteveem"),
    ("circunver", "circunvê", "circunveem"),
    ("entrever", "entrevê", "entreveem"),
    ("interver", "intervê", "interveem"),
    ("prever", "prevê", "preveem"),
    ("rever", "revê", "reveem"),
    ("telever", "televê", "televeem"),
    ("prover", "provê", "proveem"),
    ("desprover", "desprovê", "desproveem"),
    ("ler", "lê", "leem"),
    ("reler", "relê", "releem"),
    ("treler", "trelê", "treleem"),
    ("tresler", "treslê", "tresleem"),
    ("crer", "crê", "creem"),
    ("descrer", "descrê", "descreem"),
    // rir
    ("rir", "ri", "riem"),
    ("sorrir", "sorri", "sorriem"),
    // -iar que se conjugam como -ear (Mário: mediar, ansiar, remediar, incendiar, odiar)
    ("odiar", "odeia", "odeiam"),
    ("ansiar", "anseia", "anseiam"),
    ("incendiar", "incendeia", "incendeiam"),
    ("mediar", "medeia", "medeiam"),
    ("intermediar", "intermedeia", "intermedeiam"),
    ("remediar", "remedeia", "remedeiam"),
    ("desremediar", "desremedeia", "desremedeiam"),
    // -uir com alternância o/u: abundantes, ver o relatório do checker
    ("construir", "constrói", "constroem"),
    ("desconstruir", "desconstrói", "desconstroem"),
    ("reconstruir", "reconstrói", "reconstroem"),
    ("destruir", "destrói", "destroem"),
    // u tônico pronunciado depois de g/q: "argui", como os -uir
    ("arguir", "argui", "arguem"),
    ("redarguir", "redargui", "redarguem"),
    ("delinquir", "delinqui", "delinquem"),
    // alternância vocálica no radical (u → o, e → i, o → u)
    ("subir", "sobe", "sobem"),
    ("sumir", "some", "somem"),
    ("consumir", "consome", "consomem"),
    ("dessumir", "dessome", "dessomem"),
    ("subsumir", "subsome", "subsomem"),
    ("cuspir", "cospe", "cospem"),
    ("escapulir", "escapole", "escapolem"),
    ("prevenir", "previne", "previnem"),
    ("desprevenir", "desprevine", "desprevinem"),
    ("denegrir", "denigre", "denigrem"),
    ("fremir", "frime", "frimem"),
    ("rebolir", "rebule", "rebulem"),
    ("sortir", "surte", "surtem"),
];

/// Famílias por terminação: qualquer infinitivo que termine assim troca a terminação
/// pelas formas dadas e mantém o prefixo ("satisfazer" → "satisfaz", "compor" →
/// "compõe"). Cada terminação foi conferida contra o dicionário inteiro.
const FAMILIES: &[(&str, &str, &str)] = &[
    ("fazer", "faz", "fazem"),
    ("dizer", "diz", "dizem"),
    ("trazer", "traz", "trazem"),
    ("prazer", "praz", "prazem"),
    ("jazer", "jaz", "jazem"),
    ("querer", "quer", "querem"),
    ("uzir", "uz", "uzem"),
    ("pôr", "põe", "põem"),
    ("por", "põe", "põem"),
    ("gredir", "gride", "gridem"),
    ("fugir", "foge", "fogem"),
    ("cudir", "code", "codem"),
    ("bulir", "bole", "bolem"),
    ("polir", "pule", "pulem"),
    ("oer", "ói", "oem"),
    ("air", "ai", "aem"),
    ("ear", "eia", "eiam"),
];

/// Hiatos com i/u tônico, que levam acento agudo na 3ª pessoa ("saudar" → "saúda",
/// "reunir" → "reúne", "proibir" → "proíbe"). A grafia não distingue hiato de ditongo
/// ("saudar" e "causar"), por isso a lista é de terminações conferidas no dicionário.
/// Depois de ditongo decrescente não há acento ("abaiucar" → "abaiuca").
const HIATUS: &[&str] = &[
    "aizar",
    "eizar",
    "juizar",
    "buizar",
    "saudar",
    "miudar",
    "graudar",
    "caudar",
    "baular",
    "paular",
    "faular",
    "balaustrar",
    "baucar",
    "gauchar",
    "jesuitar",
    "ruinar",
    "aunar",
    "iumar",
    "viuvar",
    "miuçar",
    "faiscar",
    "reunir",
    "oibir",
];

/// "crescer" → ("cresce", "crescem"). `None` quando a palavra não é infinitivo ou o
/// verbo não tem 3ª pessoa do presente.
pub fn third_person(inf: &str) -> Option<(String, String)> {
    // Compostos com hífen ("bem-fazer", "co-produzir") flexionam só o último elemento.
    if let Some((head, tail)) = inf.rsplit_once('-') {
        if head.is_empty() {
            return None;
        }
        let (s, p) = third_person(tail)?;
        return Some((format!("{head}-{s}"), format!("{head}-{p}")));
    }
    if DEFECTIVE.contains(&inf) {
        return None;
    }
    if let Some((_, s, p)) = IRREGULAR_THIRD_PERSON.iter().find(|(k, _, _)| *k == inf) {
        return Some((s.to_string(), p.to_string()));
    }
    for (suffix, s, p) in FAMILIES {
        if let Some(prefix) = inf.strip_suffix(suffix)
            && (!prefix.is_empty() || *suffix != "por")
        {
            return Some((format!("{prefix}{s}"), format!("{prefix}{p}")));
        }
    }
    if let Some(stem) = inf.strip_suffix("uir")
        && !stem.ends_with('g')
        && !stem.ends_with('q')
    {
        return Some((format!("{stem}ui"), format!("{stem}uem")));
    }
    let (stem, ending) = split_ending(inf)?;
    let stem = if HIATUS.iter().any(|h| inf.ends_with(h)) { accent_last_high_vowel(stem) } else { stem.to_string() };
    let vowel = if ending == "ar" { "a" } else { "e" };
    Some((format!("{stem}{vowel}"), format!("{stem}{vowel}m")))
}

/// Acento agudo no último i/u do radical: "reun" → "reún".
fn accent_last_high_vowel(stem: &str) -> String {
    match stem.rfind(['i', 'u']) {
        Some(i) => {
            let accented = if stem[i..].starts_with('i') { "í" } else { "ú" };
            format!("{}{accented}{}", &stem[..i], &stem[i + 1..])
        }
        None => stem.to_string(),
    }
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

#[cfg(test)]
mod tests {
    use super::third_person;

    fn t(inf: &str) -> Option<(String, String)> {
        third_person(inf)
    }

    #[test]
    fn regular_and_irregular() {
        let cases = [
            ("crescer", "cresce", "crescem"),
            ("virar", "vira", "viram"),
            ("partir", "parte", "partem"),
            ("começar", "começa", "começam"),
            ("seguir", "segue", "seguem"),
            ("extinguir", "extingue", "extinguem"),
            ("diminuir", "diminui", "diminuem"),
            ("possuir", "possui", "possuem"),
            ("construir", "constrói", "constroem"),
            ("arguir", "argui", "arguem"),
            ("cair", "cai", "caem"),
            ("atrair", "atrai", "atraem"),
            ("doer", "dói", "doem"),
            ("roer", "rói", "roem"),
            ("passear", "passeia", "passeiam"),
            ("frear", "freia", "freiam"),
            ("odiar", "odeia", "odeiam"),
            ("variar", "varia", "variam"),
            ("ser", "é", "são"),
            ("estar", "está", "estão"),
            ("ir", "vai", "vão"),
            ("dar", "dá", "dão"),
            ("haver", "há", "hão"),
            ("ter", "tem", "têm"),
            ("conter", "contém", "contêm"),
            ("bater", "bate", "batem"),
            ("vir", "vem", "vêm"),
            ("intervir", "intervém", "intervêm"),
            ("servir", "serve", "servem"),
            ("ver", "vê", "veem"),
            ("prever", "prevê", "preveem"),
            ("mover", "move", "movem"),
            ("ler", "lê", "leem"),
            ("crer", "crê", "creem"),
            ("pôr", "põe", "põem"),
            ("compor", "compõe", "compõem"),
            ("fazer", "faz", "fazem"),
            ("satisfazer", "satisfaz", "satisfazem"),
            ("dizer", "diz", "dizem"),
            ("trazer", "traz", "trazem"),
            ("cozer", "coze", "cozem"),
            ("requerer", "requer", "requerem"),
            ("produzir", "produz", "produzem"),
            ("subir", "sobe", "sobem"),
            ("fugir", "foge", "fogem"),
            ("sacudir", "sacode", "sacodem"),
            ("consumir", "consome", "consomem"),
            ("assumir", "assume", "assumem"),
            ("agredir", "agride", "agridem"),
            ("polir", "pule", "pulem"),
            ("rir", "ri", "riem"),
            ("saudar", "saúda", "saúdam"),
            ("reunir", "reúne", "reúnem"),
            ("proibir", "proíbe", "proíbem"),
            ("enraizar", "enraíza", "enraízam"),
            ("causar", "causa", "causam"),
            ("apoiar", "apoia", "apoiam"),
            ("bem-fazer", "bem-faz", "bem-fazem"),
            ("co-produzir", "co-produz", "co-produzem"),
            ("instruir", "instrui", "instruem"),
            ("anarquizar", "anarquiza", "anarquizam"),
            ("traduzir", "traduz", "traduzem"),
            ("jazer", "jaz", "jazem"),
            ("abolir", "abole", "abolem"),
            ("negociar", "negocia", "negociam"),
            ("arquear", "arqueia", "arqueiam"),
            ("abaiucar", "abaiuca", "abaiucam"),
            ("sobrestar", "sobrestá", "sobrestão"),
            ("enxaguar", "enxagua", "enxaguam"),
            ("sortir", "surte", "surtem"),
        ];
        for (inf, s, p) in cases {
            assert_eq!(t(inf), Some((s.into(), p.into())), "{inf}");
        }
    }

    #[test]
    fn not_a_verb_or_defective() {
        for w in ["por", "x", "saldo", "falir", "reaver", "precaver", "remir", "-ar", "queixar-se"] {
            assert_eq!(t(w), None, "{w}");
        }
    }
}
