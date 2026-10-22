//! 1ª pessoa do singular do presente do indicativo: é a forma que executa
//! ("Meço a carta."). Regras que cobrem famílias inteiras de verbos, mais as tabelas
//! de exceções.
//!
//! Arquivo autocontido (sem dependências do resto do crate), para poder ser testado
//! sozinho contra o dicionário inteiro (`tools/morphology/check_first_person.py`).
//!
//! Ordem de decisão:
//! 1. `-se` no fim: conjuga o verbo sem o pronome e põe `-me` (queixar-se, queixo-me).
//! 2. Hífen: conjuga só a última parte (contra-atacar, contra-ataco).
//! 3. Defectivos sem 1ª pessoa: `None`.
//! 4. Tabela de formas soltas.
//! 5. Famílias irregulares com os prefixos que a língua usa (ter, deter, manter...).
//! 6. Famílias irregulares por terminação, que aceitam qualquer prefixo (-fazer,
//!    -dizer, -por...).
//! 7. Regras regulares por conjugação, com os ajustes de grafia e de timbre.

/// Verbos sem 1ª pessoa do singular do presente: os defectivos clássicos (abolir,
/// colorir, falir) e os unipessoais de fenômeno da natureza e de voz de animal
/// (chover, nevar, latir), como no Verbiste. Ordem alfabética, busca binária.
const DEFECTIVE: &[&str] = &[
    "abolir",
    "adir",
    "aguerrir",
    "alvorecer",
    "amanhecer",
    "anoitecer",
    "balar",
    "balir",
    "banir",
    "barrir",
    "bramir",
    "brandir",
    "branquir",
    "brotar",
    "brunir",
    "burnir",
    "carpir",
    "chilrar",
    "chilrear",
    "chover",
    "chuviscar",
    "coaxar",
    "colorir",
    "combalir",
    "comedir",
    "cucuricar",
    "delinquir",
    "delir",
    "demolir",
    "descolorir",
    "descomedir",
    "desempedernir",
    "desflorir",
    "desprecaver",
    "embair",
    "emergir",
    "emolir",
    "empedernir",
    "espargir",
    "espavorir",
    "exaurir",
    "exinanir",
    "explodir",
    "extorquir",
    "falir",
    "florir",
    "fornir",
    "fremir",
    "fulgir",
    "garrir",
    "gear",
    "grugulejar",
    "grunhir",
    "haurir",
    "imergir",
    "inanir",
    "injungir",
    "insculpir",
    "jungir",
    "languir",
    "latir",
    "lenir",
    "monir",
    "nevar",
    "neviscar",
    "nitrir",
    "orvalhar",
    "pascer",
    "precaver",
    "prefulgir",
    "pruir",
    "prurir",
    "puir",
    "pupilar",
    "reaver",
    "rebusnar",
    "reflorir",
    "refulgir",
    "regougar",
    "relinchar",
    "renhir",
    "repascer",
    "ressequir",
    "retinir",
    "retintinir",
    "retorquir",
    "rugir",
    "ruir",
    "saraivar",
    "soer",
    "submergir",
    "tinir",
    "trovejar",
    "urgir",
    "vagir",
    "ventar",
    "zurzir",
];

/// Formas soltas, que não se deduzem de regra nem de família.
const IRREGULAR_FIRST_PERSON: &[(&str, &str)] = &[
    // Supletivo: remir empresta a forma de redimir.
    ("remir", "redimo"),
    // Alternância o/u isolada.
    ("despavorir", "despavuro"),
    ("sortir", "surto"),
];

/// Famílias irregulares em que só alguns prefixos formam derivados de verdade
/// (`deter` vem de `ter`, `meter` não). Cada linha: raiz, 1ª pessoa da raiz,
/// prefixos aceitos (o vazio é a própria raiz).
const PREFIXED_FAMILIES: &[(&str, &str, &[&str])] = &[
    (
        "ter",
        "tenho",
        &[
            "", "a", "abs", "con", "de", "entre", "man", "ob", "re", "sus",
        ],
    ),
    (
        "vir",
        "venho",
        &[
            "", "a", "ad", "con", "contra", "de", "desa", "descon", "inter", "ob", "pro", "recon",
            "re", "sobre",
        ],
    ),
    (
        "ver",
        "vejo",
        &["", "ante", "circun", "entre", "inter", "pre", "re", "tele"],
    ),
    ("prover", "provejo", &["", "des"]),
    ("ler", "leio", &["", "re", "tre", "tres"]),
    ("crer", "creio", &["", "des"]),
    ("caber", "caibo", &["", "des"]),
    ("saber", "sei", &["", "des", "res"]),
    ("estar", "estou", &["", "sob", "sobr", "sobre"]),
    ("ser", "sou", &["", "sobres"]),
    ("dar", "dou", &["", "des", "re"]),
    ("ir", "vou", &[""]),
    ("rir", "rio", &["", "sor"]),
    ("haver", "hei", &[""]),
    ("poder", "posso", &[""]),
    ("requerer", "requeiro", &[""]),
    ("perder", "perco", &["", "des", "dis"]),
    // -iar que se conjugam como -ear (o grupo "MARIO": mediar, ansiar, remediar,
    // incendiar, odiar, e os derivados).
    ("ansiar", "anseio", &[""]),
    ("odiar", "odeio", &[""]),
    ("incendiar", "incendeio", &[""]),
    ("mediar", "medeio", &["", "re", "inter", "desre"]),
];

/// Famílias irregulares que valem com qualquer prefixo: toda palavra terminada assim
/// é derivado do mesmo verbo. Cada linha: terminação do infinitivo, terminação da
/// 1ª pessoa.
const SUFFIX_FAMILIES: &[(&str, &str)] = &[
    ("fazer", "faço"),
    ("dizer", "digo"),
    ("trazer", "trago"),
    ("valer", "valho"),
    ("ouvir", "ouço"),
    ("medir", "meço"),
    ("pedir", "peço"),
    ("pôr", "ponho"),
    ("por", "ponho"),
    // gu pronunciado: arguir e redarguir mantêm o u (arguo), ao contrário de
    // distinguir/distingo.
    ("arguir", "arguo"),
    // parir troca o timbre como caber/caibo.
    ("parir", "pairo"),
    // raer se conjuga como cair/caio.
    ("raer", "raio"),
    // Tônica no i do radical, com acento: mobílio.
    ("mobiliar", "mobílio"),
    // Hiatos com vogal tônica acentuada na 1ª pessoa (saúdo, proíbo, ciúmo).
    ("saudar", "saúdo"),
    ("graudar", "graúdo"),
    ("iudar", "iúdo"),
    ("baular", "baúlo"),
    ("paular", "paúlo"),
    ("faular", "faúlo"),
    ("iumar", "iúmo"),
    ("aunar", "aúno"),
    ("iuvar", "iúvo"),
    ("iuçar", "iúço"),
    ("aiucar", "aiúco"),
    ("baucar", "baúco"),
    ("balaustrar", "balaústro"),
    ("jesuitar", "jesuíto"),
    ("ruinar", "ruíno"),
    ("gauchar", "gaúcho"),
    ("faiscar", "faísco"),
    ("eunir", "eúno"),
];

/// Verbos em `-ir` com `e` no radical que não fazem a alternância e/i.
const NO_VOWEL_ALTERNATION: &[&str] = &[
    "convelir",
    "crespir",
    "digestir",
    "estresir",
    "excelir",
    "fretenir",
    "inquerir",
    "manutenir",
    "premir",
];

/// "medir" → "meço". `None` quando a palavra não é infinitivo ou o verbo não tem 1ª pessoa.
pub fn first_person(inf: &str) -> Option<String> {
    if let Some(base) = inf.strip_suffix("-se") {
        return first_person(base).map(|f| format!("{f}-me"));
    }
    if let Some((head, tail)) = inf.rsplit_once('-') {
        if head.is_empty() {
            return None;
        }
        return first_person(tail).map(|f| format!("{head}-{f}"));
    }
    if !inf.chars().all(is_lowercase_letter) {
        return None;
    }
    if DEFECTIVE.binary_search(&inf).is_ok() {
        return None;
    }
    if let Some((_, f)) = IRREGULAR_FIRST_PERSON.iter().find(|(k, _)| *k == inf) {
        return Some(f.to_string());
    }
    for (root, form, prefixes) in PREFIXED_FAMILIES {
        if let Some(prefix) = inf.strip_suffix(root)
            && prefixes.contains(&prefix)
        {
            return Some(format!("{prefix}{form}"));
        }
    }
    // "por" sozinho é a preposição; o verbo se escreve "pôr".
    if inf == "por" {
        return None;
    }
    for (ending, form) in SUFFIX_FAMILIES {
        if let Some(prefix) = inf.strip_suffix(ending) {
            return Some(format!("{prefix}{form}"));
        }
    }
    let (stem, ending) = split_ending(inf)?;
    match ending {
        "ar" => Some(first_conjugation(stem)),
        "er" => Some(second_conjugation(stem)),
        _ => Some(third_conjugation(inf, stem)),
    }
}

/// `-ar`: o radical mais `-o`, com `-ear` → `-eio` (passeio) e o acento do hiato em
/// `-izar` depois de vogal (enraízo, ajuízo).
fn first_conjugation(stem: &str) -> String {
    if let Some(s) = stem.strip_suffix('e') {
        return format!("{s}eio");
    }
    if let Some(s) = stem.strip_suffix("iz")
        && ends_in_hiatus_vowel(s)
    {
        return format!("{s}ízo");
    }
    format!("{stem}o")
}

/// `-er`: o radical mais `-o`, com `c` → `ç` (conheço), `g` → `j` (protejo) e
/// `gu` → `g` (ergo).
fn second_conjugation(stem: &str) -> String {
    format!("{}o", soften(stem))
}

/// `-ir`: alternância de timbre (sinto, durmo), `-air` → `-aio`, o acento do hiato
/// em `-ibir` (proíbo), e os mesmos ajustes de grafia da segunda conjugação.
fn third_conjugation(inf: &str, stem: &str) -> String {
    if stem.ends_with('a') {
        return format!("{stem}io");
    }
    if let Some(s) = stem.strip_suffix("ib")
        && ends_in_hiatus_vowel(s)
    {
        return format!("{s}íbo");
    }
    let stem = if NO_VOWEL_ALTERNATION.contains(&inf) {
        stem.to_string()
    } else {
        alternate_vowel(stem)
    };
    format!("{}o", soften(&stem))
}

/// Ajuste de grafia antes do `-o` nas conjugações em `-er` e `-ir`, para manter o som
/// da consoante: `c` → `ç`, `g` → `j`, `gu` → `g`.
fn soften(stem: &str) -> String {
    if let Some(s) = stem.strip_suffix("gu") {
        format!("{s}g")
    } else if let Some(s) = stem.strip_suffix('c') {
        format!("{s}ç")
    } else if let Some(s) = stem.strip_suffix('g') {
        format!("{s}j")
    } else {
        stem.to_string()
    }
}

/// Alternância vocálica dos verbos em `-ir`: o `e` da última sílaba do radical vira
/// `i` (sentir/sinto, seguir/sigo, agredir/agrido) e o `o` vira `u` em `-obr`, `-orm`,
/// `-ol` e `-oss` (cobrir/cubro, dormir/durmo, polir/pulo, tossir/tusso).
fn alternate_vowel(stem: &str) -> String {
    for cluster in ["br", "rm", "l", "ss"] {
        if let Some(before) = stem.strip_suffix(cluster)
            && let Some(b) = before.strip_suffix('o')
        {
            return format!("{b}u{cluster}");
        }
    }
    // A última vogal do radical, com o `u` de `gu`/`qu` contado como consoante.
    let core = if stem.ends_with("gu") || stem.ends_with("qu") {
        &stem[..stem.len() - 1]
    } else {
        stem
    };
    if let Some((i, 'e')) = core.char_indices().rev().find(|(_, c)| is_vowel(*c))
        && i + 1 < core.len()
    {
        return format!("{}i{}", &stem[..i], &stem[i + 1..]);
    }
    stem.to_string()
}

/// O radical termina numa vogal que forma hiato com o `i`/`u` seguinte (arca-izar,
/// pro-ibir, aju-izar), e não num `u` mudo de `gu`/`qu`.
fn ends_in_hiatus_vowel(s: &str) -> bool {
    if s.ends_with("gu") || s.ends_with("qu") {
        return false;
    }
    s.chars().last().is_some_and(|c| "aeou".contains(c))
}

fn is_vowel(c: char) -> bool {
    "aeiouáéíóúâêôãõà".contains(c)
}

fn is_lowercase_letter(c: char) -> bool {
    c.is_ascii_lowercase() || "áéíóúâêôãõàçü".contains(c)
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
    use super::first_person;

    fn check(cases: &[(&str, &str)]) {
        for (inf, expected) in cases {
            assert_eq!(first_person(inf).as_deref(), Some(*expected), "{inf}");
        }
    }

    #[test]
    fn defective_table_is_sorted() {
        assert!(super::DEFECTIVE.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn regular_and_irregular() {
        check(&[
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
        ]);
    }

    #[test]
    fn spelling_adjustments() {
        check(&[
            ("ficar", "fico"),
            ("pagar", "pago"),
            ("começar", "começo"),
            ("nascer", "nasço"),
            ("ressarcir", "ressarço"),
            ("dirigir", "dirijo"),
            ("atingir", "atinjo"),
            ("distinguir", "distingo"),
            ("extinguir", "extingo"),
            ("arguir", "arguo"),
            ("redarguir", "redarguo"),
            ("averiguar", "averiguo"),
            ("distribuir", "distribuo"),
            ("possuir", "possuo"),
            ("voar", "voo"),
            ("perdoar", "perdoo"),
            ("moer", "moo"),
            ("corroer", "corroo"),
            ("apoiar", "apoio"),
            ("copiar", "copio"),
            ("negociar", "negocio"),
        ]);
    }

    #[test]
    fn third_conjugation() {
        check(&[
            ("sair", "saio"),
            ("atrair", "atraio"),
            ("subtrair", "subtraio"),
            ("sentir", "sinto"),
            ("vestir", "visto"),
            ("repetir", "repito"),
            ("agredir", "agrido"),
            ("prevenir", "previno"),
            ("despir", "dispo"),
            ("divergir", "divirjo"),
            ("discernir", "discirno"),
            ("cerzir", "cirzo"),
            ("repelir", "repilo"),
            ("coerir", "coiro"),
            ("perseguir", "persigo"),
            ("engolir", "engulo"),
            ("polir", "pulo"),
            ("tossir", "tusso"),
            ("descobrir", "descubro"),
            ("sortir", "surto"),
            ("subir", "subo"),
            ("fugir", "fujo"),
            ("cuspir", "cuspo"),
            ("parir", "pairo"),
            ("convelir", "convelo"),
            ("premir", "premo"),
        ]);
    }

    #[test]
    fn hiatus_accent() {
        check(&[
            ("enraizar", "enraízo"),
            ("ajuizar", "ajuízo"),
            ("europeizar", "europeízo"),
            ("anarquizar", "anarquizo"),
            ("proibir", "proíbo"),
            ("coibir", "coíbo"),
            ("exibir", "exibo"),
            ("reunir", "reúno"),
            ("saudar", "saúdo"),
            ("esmiudar", "esmiúdo"),
            ("fraudar", "fraudo"),
            ("abaular", "abaúlo"),
            ("enjaular", "enjaulo"),
            ("enciumar", "enciúmo"),
            ("viuvar", "viúvo"),
            ("esmiuçar", "esmiúço"),
            ("embaucar", "embaúco"),
            ("faiscar", "faísco"),
            ("arruinar", "arruíno"),
            ("ajesuitar", "ajesuíto"),
            ("aceitar", "aceito"),
            ("mobiliar", "mobílio"),
        ]);
    }

    #[test]
    fn families_with_any_prefix() {
        check(&[
            ("fazer", "faço"),
            ("satisfazer", "satisfaço"),
            ("dizer", "digo"),
            ("contradizer", "contradigo"),
            ("trazer", "trago"),
            ("valer", "valho"),
            ("equivaler", "equivalho"),
            ("ouvir", "ouço"),
            ("entreouvir", "entreouço"),
            ("pedir", "peço"),
            ("despedir", "despeço"),
            ("remedir", "remeço"),
            ("pôr", "ponho"),
            ("pressupor", "pressuponho"),
            ("raer", "raio"),
        ]);
    }

    #[test]
    fn families_with_known_prefixes() {
        check(&[
            ("ter", "tenho"),
            ("manter", "mantenho"),
            ("abster", "abstenho"),
            ("meter", "meto"),
            ("bater", "bato"),
            ("vir", "venho"),
            ("intervir", "intervenho"),
            ("servir", "sirvo"),
            ("ver", "vejo"),
            ("prever", "prevejo"),
            ("escrever", "escrevo"),
            ("prover", "provejo"),
            ("reler", "releio"),
            ("valer", "valho"),
            ("descrer", "descreio"),
            ("caber", "caibo"),
            ("saber", "sei"),
            ("estar", "estou"),
            ("sobrestar", "sobrestou"),
            ("restar", "resto"),
            ("ser", "sou"),
            ("coser", "coso"),
            ("dar", "dou"),
            ("nadar", "nado"),
            ("ir", "vou"),
            ("rir", "rio"),
            ("sorrir", "sorrio"),
            ("haver", "hei"),
            ("poder", "posso"),
            ("querer", "quero"),
            ("requerer", "requeiro"),
            ("remediar", "remedeio"),
            ("intermediar", "intermedeio"),
            ("incendiar", "incendeio"),
            ("ansiar", "anseio"),
            ("custodiar", "custodio"),
            ("remir", "redimo"),
        ]);
    }

    #[test]
    fn pronominal_and_hyphenated() {
        check(&[
            ("queixar-se", "queixo-me"),
            ("pôr-se", "ponho-me"),
            ("ir-se", "vou-me"),
            ("despedir-se", "despeço-me"),
            ("bem-querer", "bem-quero"),
            ("contra-atacar", "contra-ataco"),
            ("auto-analisar-se", "auto-analiso-me"),
        ]);
    }

    #[test]
    fn invented_verbs_follow_the_rules() {
        check(&[
            ("deletar", "deleto"),
            ("printar", "printo"),
            ("resetar", "reseto"),
            ("tuitar", "tuito"),
            ("googlar", "googlo"),
        ]);
    }

    #[test]
    fn no_first_person() {
        for inf in [
            "abolir",
            "colorir",
            "falir",
            "explodir",
            "chover",
            "latir",
            "reaver",
            "precaver",
            "abolir-se",
            "por",
            "casa",
            "ar",
            "Medir",
            "",
        ] {
            assert_eq!(first_person(inf), None, "{inf}");
        }
    }
}
