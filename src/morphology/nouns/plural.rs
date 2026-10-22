//! Plural dos substantivos e adjetivos: liga "Para cada nota" a "as notas", dá o
//! plural dos tipos definidos pelo usuário ("Uma data" → "datas") e flexiona
//! adjetivos e particípios ("emprestados", "confiadas").
//!
//! Arquivo autocontido (sem dependências do resto do crate), para poder ser testado
//! sozinho contra o dicionário inteiro (`tools/morphology/check_plural.py`).
//!
//! O plural sai de regras pela terminação e pela tonicidade, e a tonicidade sai das
//! regras de acentuação gráfica: o acento agudo ou circunflexo marca a sílaba tônica;
//! sem acento, a palavra terminada em -a, -e, -o (com ou sem -s), -am, -em, -ens é
//! paroxítona, e as demais são oxítonas. Uma tabela pequena guarda o que as regras não
//! alcançam (o -ão em -ães e -ãos, "caráter", "cônsul", estrangeirismos).

/// Acentos que marcam a sílaba tônica (o til marca nasalidade, não tonicidade).
const STRESS_MARKS: &str = "áéíóúâêô";
const VOWELS: &str = "aeiouáéíóúâêôãõàü";

/// Palavras em -ão com plural em -ães (o padrão é -ões).
const AO_AES: &[&str] = &["ademão", "alavão", "alemão", "capelão", "capitão", "catalão", "cão", "escrivão", "pão", "tabelião"];

/// Palavras oxítonas em -ão com plural em -ãos. As paroxítonas ("órgão", "sótão",
/// "bênção") vão a -ãos pela regra, sem precisar estar aqui.
const AO_AOS: &[&str] = &[
    "abaritão",
    "afegão",
    "amontão",
    "arquissão",
    "bangão",
    "benção",
    "bordegão",
    "borguinhão",
    "chão",
    "cidadão",
    "coimbrão",
    "cristão",
    "demão",
    "desvão",
    "engatatão",
    "estrabão",
    "esvão",
    "foção",
    "fugião",
    "garnachão",
    "geresão",
    "gozão",
    "grão",
    "hortelão",
    "ignorantão",
    "irmão",
    "mão",
    "não",
    "pagão",
    "pardalão",
    "repontão",
    "são",
    "tinturão",
    "vão",
    "zombeirão",
    "zornão",
    "zorrão",
];

/// Plurais que nenhuma regra dá.
const EXCEPTIONS: &[(&str, &str)] = &[
    // A tônica muda de sílaba.
    ("caráter", "caracteres"),
    // Invariáveis por tradição, embora oxítonas.
    ("cais", "cais"),
    // -ul paroxítono em -ules.
    ("cônsul", "cônsules"),
    // -al e -ol fora da regra.
    ("gol", "gols"),
    ("mal", "males"),
    ("mol", "mols"),
    // Estrangeirismos não adaptados, sem marca gráfica de origem.
    ("center", "centers"),
    ("dossier", "dossiers"),
    ("gigapixel", "gigapixels"),
    ("hamster", "hamsters"),
    ("laser", "lasers"),
    ("megapixel", "megapixels"),
    ("topmodel", "topmodels"),
    ("trailer", "trailers"),
    ("var", "vars"),
];

/// Prefixos que se tiram para achar a palavra da tabela ("concidadão", "subchão").
/// Só vale se o que sobra tem quatro letras ou mais: "cocão" e "comão" não são
/// "co" + "cão" nem "co" + "mão".
const PREFIXES: &[&str] = &[
    "anti", "arqui", "ciber", "con", "des", "hiper", "macro", "micro", "mini", "multi", "nano", "neo", "proto", "pseudo", "semi", "sub",
    "super", "ultra",
];

/// Palavras que vão sem plural na frente de um composto: prefixos e advérbios, formas
/// reduzidas de adjetivo pátrio ou técnico e pronomes ("ex-aluno", "luso-brasileiro",
/// "bem-te-vi").
const INVARIABLE_HEADS: &[&str] = &[
    "ab",
    "abaixo",
    "acima",
    "afro",
    "agro",
    "além",
    "ando",
    "anglo",
    "ante",
    "ântero",
    "anti",
    "aquém",
    "arqui",
    "ásio",
    "austro",
    "auto",
    "balto",
    "belgo",
    "bem",
    "bi",
    "brasilo",
    "celto",
    "centro",
    "chino",
    "circum",
    "contra",
    "di",
    "eletro",
    "entre",
    "euro",
    "ex",
    "extra",
    "franco",
    "germano",
    "grã",
    "grão",
    "greco",
    "hemi",
    "hiper",
    "hispano",
    "ibero",
    "imuno",
    "indo",
    "ínfero",
    "infra",
    "inter",
    "íntero",
    "intra",
    "israelo",
    "ítalo",
    "judeo",
    "látero",
    "latino",
    "lés",
    "leto",
    "lhe",
    "loés",
    "luso",
    "macro",
    "magneto",
    "mal",
    "me",
    "mega",
    "meta",
    "micro",
    "mini",
    "multi",
    "nano",
    "neerlando",
    "neo",
    "neuro",
    "nigero",
    "nilo",
    "nipo",
    "norte",
    "não",
    "ob",
    "oés",
    "pan",
    "para",
    "pluri",
    "poli",
    "polono",
    "pós",
    "póstero",
    "pré",
    "pró",
    "proto",
    "pseudo",
    "quase",
    "radio",
    "recém",
    "retro",
    "salpingo",
    "são",
    "se",
    "semi",
    "sem",
    "sempre",
    "sino",
    "siro",
    "sobre",
    "sota",
    "soto",
    "sub",
    "sul",
    "súpero",
    "super",
    "supra",
    "te",
    "termo",
    "teuto",
    "traco",
    "tri",
    "ultra",
    "vice",
];

/// Verbos que abrem composto de verbo e substantivo, em que só o substantivo vai ao
/// plural ("guarda-chuvas", "porta-aviões", "beija-flores").
const VERB_HEADS: &[&str] = &[
    "abaixa",
    "abraça",
    "abre",
    "afia",
    "agarra",
    "alça",
    "amola",
    "apaga",
    "aperta",
    "arranca",
    "arranha",
    "arrasta",
    "arrebenta",
    "bate",
    "bebe",
    "beija",
    "bota",
    "busca",
    "caça",
    "caga",
    "cata",
    "cerra",
    "cheira",
    "chora",
    "chupa",
    "cobre",
    "come",
    "corre",
    "corta",
    "cospe",
    "enche",
    "engana",
    "engole",
    "espanta",
    "espreme",
    "fecha",
    "finca",
    "fura",
    "furta",
    "ganha",
    "gira",
    "guarda",
    "joga",
    "lambe",
    "lança",
    "lava",
    "limpa",
    "marca",
    "mata",
    "mede",
    "mete",
    "mija",
    "mira",
    "papa",
    "passa",
    "pega",
    "perde",
    "pesa",
    "pica",
    "pinga",
    "pisa",
    "porta",
    "prende",
    "pula",
    "puxa",
    "quebra",
    "rapa",
    "rasga",
    "risca",
    "rompe",
    "saca",
    "salta",
    "salva",
    "segura",
    "sobe",
    "suga",
    "tapa",
    "tira",
    "toca",
    "torna",
    "tranca",
    "treme",
    "trinca",
    "troca",
    "vende",
    "vira",
    "volta",
];

/// Substantivos que, depois do primeiro, especificam o tipo e ficam no singular
/// ("palavras-chave", "públicos-alvo", "navios-escola"), e cores dadas por substantivo
/// ("camarões-rosa").
const SPECIFIERS: &[&str] = &[
    "abóbora",
    "alvo",
    "base",
    "bomba",
    "chave",
    "cinza",
    "creme",
    "escola",
    "fantasma",
    "gelo",
    "laranja",
    "limite",
    "modelo",
    "monstro",
    "oliva",
    "padrão",
    "palha",
    "piloto",
    "relâmpago",
    "rosa",
    "salmão",
    "satélite",
    "surpresa",
    "teste",
    "tipo",
    "vinho",
];

/// Sufixos de adjetivo, para reconhecer o adjetivo composto ("econômico-financeiro").
const ADJECTIVE_SUFFIXES: &[&str] = &[
    "al", "ano", "ana", "ar", "ário", "ária", "ês", "esa", "ico", "ica", "ino", "ina", "ivo", "iva", "oso", "osa", "ório", "ória", "eiro",
    "eira",
];

/// Preposições, contrações e conjunções que separam o núcleo de um composto: só o que
/// vem antes delas vai ao plural ("pés-de-moleque", "amores-perfeitos-do-mato").
const COMPOUND_LINKS: &[&str] = &[
    "a", "à", "ao", "aos", "às", "com", "da", "das", "de", "do", "dos", "e", "em", "na", "nas", "no", "nos", "ou", "pela", "pelas", "pelo",
    "pelos", "por", "sem", "sob",
];

fn is_vowel(c: char) -> bool {
    VOWELS.contains(c)
}

fn strip_stress_char(c: char) -> char {
    match c {
        'á' | 'â' => 'a',
        'é' | 'ê' => 'e',
        'í' => 'i',
        'ó' | 'ô' => 'o',
        'ú' => 'u',
        other => other,
    }
}

fn strip_stress(s: &str) -> String {
    s.chars().map(strip_stress_char).collect()
}

/// Corta os `n` últimos caracteres.
fn cut(s: &str, n: usize) -> &str {
    let idx = s.char_indices().rev().nth(n - 1).map_or(0, |(i, _)| i);
    &s[..idx]
}

/// A última sílaba é a tônica?
fn is_oxytone(w: &str) -> bool {
    let chars: Vec<char> = w.chars().collect();
    if let Some(i) = chars.iter().rposition(|c| STRESS_MARKS.contains(*c)) {
        let rest: String = chars[i + 1..].iter().collect();
        // Depois da tônica só consoante ou a semivogal de um ditongo ("herói", "réis").
        return !rest.chars().any(is_vowel) || matches!(rest.as_str(), "i" | "u" | "is" | "us");
    }
    if ["ã", "ãs", "ão", "ãos", "ãe", "ães", "õe", "ões"].iter().any(|e| w.ends_with(e)) {
        return true;
    }
    !["a", "e", "o", "as", "es", "os", "am", "em", "ens"].iter().any(|e| w.ends_with(e))
}

/// Estrangeirismo em -er com grafia de fora (letra k, w ou y, ou consoante dobrada
/// que o português não usa): "hacker", "plotter", "player". Só vale para palavra sem
/// acento, porque a acentuada já foi adaptada ("káiser", "pôster").
fn is_foreign_er(w: &str) -> bool {
    if !w.ends_with("er") || w.chars().any(|c| STRESS_MARKS.contains(c)) {
        return false;
    }
    if w.chars().any(|c| matches!(c, 'k' | 'w' | 'y')) {
        return true;
    }
    let chars: Vec<char> = w.chars().collect();
    chars.windows(2).any(|p| p[0] == p[1] && "bdfglptz".contains(p[0]))
}

fn is_monosyllable(w: &str) -> bool {
    let mut groups = 0;
    let mut in_vowel = false;
    for c in w.chars() {
        let v = is_vowel(c);
        if v && !in_vowel {
            groups += 1;
        }
        in_vowel = v;
    }
    groups <= 1
}

/// Tira o acento da última sílaba ao acrescentar -es ("freguês" → "fregues-"), menos
/// no í e no ú tônicos em hiato, que continuam acentuados ("país" → "países").
fn strip_final_stress(w: &str) -> String {
    let chars: Vec<char> = w.chars().collect();
    let Some(i) = chars.iter().rposition(|c| STRESS_MARKS.contains(*c)) else {
        return w.to_string();
    };
    if matches!(chars[i], 'í' | 'ú') && i > 0 && is_vowel(chars[i - 1]) {
        return w.to_string();
    }
    let mut out = chars;
    out[i] = strip_stress_char(out[i]);
    out.into_iter().collect()
}

fn table(w: &str) -> Option<String> {
    if let Some((_, p)) = EXCEPTIONS.iter().find(|(s, _)| *s == w) {
        return Some((*p).to_string());
    }
    if AO_AES.contains(&w) {
        return Some(format!("{}ães", cut(w, 2)));
    }
    if AO_AOS.contains(&w) {
        return Some(format!("{w}s"));
    }
    None
}

/// Procura na tabela a palavra inteira e, se não achar, a palavra sem um prefixo
/// ("concidadão" → "con" + "cidadão").
fn lookup(w: &str) -> Option<String> {
    if let Some(p) = table(w) {
        return Some(p);
    }
    PREFIXES.iter().find_map(|prefix| {
        let rest = w.strip_prefix(prefix)?;
        if rest.chars().count() < 4 {
            return None;
        }
        table(rest).map(|p| format!("{prefix}{p}"))
    })
}

fn is_link(part: &str) -> bool {
    COMPOUND_LINKS.contains(&part) || part.starts_with("d'")
}

/// "tico-tico", "reco-reco", "zigue-zague", "pingue-pongue": iguais tirando as vogais.
fn is_reduplication(a: &str, b: &str) -> bool {
    let consonants = |s: &str| s.chars().filter(|c| !is_vowel(*c)).collect::<String>();
    a == b || (!consonants(a).is_empty() && consonants(a) == consonants(b))
}

/// Partes do núcleo (sem preposição) que vão ao plural.
fn inflected_parts(head: &[&str]) -> Vec<usize> {
    let n = head.len();
    if n == 1 {
        return vec![0];
    }
    let first = head[0];
    // Adjetivo composto com o primeiro em -ico e o último com sufixo de adjetivo
    // ("político-sociais", "médico-cirúrgicos"); "público-alvo" é substantivo.
    let adjective_form =
        first.ends_with("ico") && first.chars().count() >= 6 && ADJECTIVE_SUFFIXES.iter().any(|s| head[n - 1].ends_with(s));
    let only_tail = INVARIABLE_HEADS.contains(&first)
        || VERB_HEADS.contains(&first)
        || adjective_form
        || is_reduplication(first, head[1])
        // Gentílico de lugar composto: só o adjetivo varia ("rio-grandenses").
        || head[n - 1].ends_with("ense");
    if only_tail {
        return inflected_parts(&head[1..]).into_iter().map(|i| i + 1).collect();
    }
    (0..n).filter(|&i| i == 0 || !SPECIFIERS.contains(&head[i])).collect()
}

fn pluralize_compound(w: &str) -> String {
    let parts: Vec<&str> = w.split('-').collect();
    let end = parts.iter().skip(1).position(|p| is_link(p)).map_or(parts.len(), |i| i + 1);
    let inflected = inflected_parts(&parts[..end]);
    parts
        .iter()
        .enumerate()
        .map(|(i, p)| if inflected.contains(&i) { pluralize_word(p) } else { (*p).to_string() })
        .collect::<Vec<_>>()
        .join("-")
}

pub fn pluralize(p: &str) -> String {
    if p.contains('-') {
        return pluralize_compound(p);
    }
    pluralize_word(p)
}

fn pluralize_word(p: &str) -> String {
    if p.is_empty() {
        return String::new();
    }
    if let Some(found) = lookup(p) {
        return found;
    }
    let ends = |s: &str| p.ends_with(s);
    let oxytone = is_oxytone(p);
    let last = p.chars().last().unwrap_or_default();

    if ends("ão") {
        // Paroxítona em -ão vai a -ãos ("órgão", "bênção"); oxítona, a -ões.
        return if oxytone { format!("{}ões", cut(p, 2)) } else { format!("{p}s") };
    }
    if is_vowel(last) {
        return format!("{p}s");
    }
    if is_foreign_er(p) || ends("ll") {
        return format!("{p}s");
    }
    let before_last = p.chars().rev().nth(1).is_some_and(is_vowel);
    match last {
        'm' if before_last => format!("{}ns", cut(p, 1)),
        // -m depois de consoante só aparece em estrangeirismo ("napalm", "ohm").
        'm' => format!("{p}s"),
        // Paroxítona em -en perde o acento: o -ens não se acentua ("hífen" → "hifens").
        'n' if ends("en") && !oxytone => format!("{}s", strip_stress(p)),
        'l' if ends("al") || ends("ul") => format!("{}is", cut(p, 1)),
        'l' if ends("el") => format!("{}{}", cut(p, 2), if oxytone { "éis" } else { "eis" }),
        'l' if ends("ol") => format!("{}{}", cut(p, 2), if oxytone { "óis" } else { "ois" }),
        'l' if ends("il") => {
            if oxytone {
                format!("{}s", cut(p, 1))
            } else {
                format!("{}eis", cut(p, 2))
            }
        }
        // Paroxítona em -ior passa a tônica para o o ("júnior" → "juniores").
        'r' if ends("ior") && !oxytone => format!("{}es", strip_stress(p)),
        'r' | 'z' => format!("{p}es"),
        's' if oxytone => format!("{}es", strip_final_stress(p)),
        's' => p.to_string(),
        'x' if is_monosyllable(p) => format!("{p}es"),
        'x' => p.to_string(),
        'h' if ends("sh") || ends("ch") => format!("{p}es"),
        _ => format!("{p}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::pluralize;

    fn check(cases: &[(&str, &str)]) {
        for (s, p) in cases {
            assert_eq!(pluralize(s), *p, "{s}");
        }
    }

    #[test]
    fn vowels_and_nasals() {
        check(&[
            ("nota", "notas"),
            ("lógico", "lógicos"),
            ("caractere", "caracteres"),
            ("café", "cafés"),
            ("irmã", "irmãs"),
            ("mãe", "mães"),
            ("herói", "heróis"),
            ("item", "itens"),
            ("armazém", "armazéns"),
            ("álbum", "álbuns"),
            ("bom", "bons"),
            ("hífen", "hifens"),
            ("pólen", "polens"),
            ("abdômen", "abdomens"),
            ("próton", "prótons"),
        ]);
    }

    #[test]
    fn words_in_ao() {
        check(&[
            ("canção", "canções"),
            ("leão", "leões"),
            ("pão", "pães"),
            ("alemão", "alemães"),
            ("capitão", "capitães"),
            ("mão", "mãos"),
            ("irmão", "irmãos"),
            ("cidadão", "cidadãos"),
            ("concidadão", "concidadãos"),
            ("subchão", "subchãos"),
            ("órgão", "órgãos"),
            ("sótão", "sótãos"),
            ("bênção", "bênçãos"),
            ("zângão", "zângãos"),
            // "co" + "cão" não é prefixo mais palavra da tabela.
            ("cocão", "cocões"),
            ("limão", "limões"),
            ("pulmão", "pulmões"),
        ]);
    }

    #[test]
    fn words_in_l() {
        check(&[
            ("real", "reais"),
            ("jângal", "jângais"),
            ("papel", "papéis"),
            ("pixel", "pixéis"),
            ("nível", "níveis"),
            ("níquel", "níqueis"),
            ("farol", "faróis"),
            ("álcool", "álcoois"),
            ("azul", "azuis"),
            ("barril", "barris"),
            ("fóssil", "fósseis"),
            ("réptil", "répteis"),
            ("cônsul", "cônsules"),
            ("mal", "males"),
            ("gol", "gols"),
        ]);
    }

    #[test]
    fn words_in_r_z_s_x() {
        check(&[
            ("orador", "oradores"),
            ("açúcar", "açúcares"),
            ("pôster", "pôsteres"),
            ("júnior", "juniores"),
            ("sênior", "seniores"),
            ("superior", "superiores"),
            ("caráter", "caracteres"),
            ("hacker", "hackers"),
            ("player", "players"),
            ("luz", "luzes"),
            ("mês", "meses"),
            ("freguês", "fregueses"),
            ("gás", "gases"),
            ("país", "países"),
            ("deus", "deuses"),
            ("lápis", "lápis"),
            ("ônibus", "ônibus"),
            ("vírus", "vírus"),
            ("pires", "pires"),
            ("cais", "cais"),
            ("tórax", "tórax"),
            ("fax", "faxes"),
            ("napalm", "napalms"),
        ]);
    }

    #[test]
    fn compounds() {
        check(&[
            ("segunda-feira", "segundas-feiras"),
            ("amor-perfeito", "amores-perfeitos"),
            ("conta-corrente", "contas-correntes"),
            ("guarda-chuva", "guarda-chuvas"),
            ("beija-flor", "beija-flores"),
            ("porta-avião", "porta-aviões"),
            ("ex-aluno", "ex-alunos"),
            ("vice-presidente", "vice-presidentes"),
            ("bem-te-vi", "bem-te-vis"),
            ("luso-brasileiro", "luso-brasileiros"),
            ("político-social", "político-sociais"),
            ("rio-grandense", "rio-grandenses"),
            ("tico-tico", "tico-ticos"),
            ("zigue-zague", "zigue-zagues"),
            ("pé-de-moleque", "pés-de-moleque"),
            ("amor-perfeito-do-mato", "amores-perfeitos-do-mato"),
            ("pau-d'alho", "paus-d'alho"),
            ("palavra-chave", "palavras-chave"),
            ("público-alvo", "públicos-alvo"),
            ("camarão-rosa", "camarões-rosa"),
        ]);
    }
}
