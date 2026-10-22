//! Gênero dos adjetivos e particípios: "emprestado" ↔ "emprestada", "português" ↔
//! "portuguesa". Usado nos particípios dos parâmetros e nos predicados adjetivos
//! ("Para a posição ser acesa" aceita "Se o pino for aceso").
//!
//! Arquivo autocontido (sem dependências do resto do crate), para poder ser testado
//! sozinho contra o dicionário inteiro (`tools/morphology`).
//!
//! As duas funções trabalham no singular e olham só o último elemento de um composto
//! com hífen ("luso-brasileiro" → "luso-brasileira"). A ordem de decisão é: tabela de
//! pares, palavras uniformes, regras por terminação.

/// Pares (masculino, feminino) que nenhuma regra produz; valem nas duas direções e
/// casam com o último elemento inteiro.
const PAIRS: &[(&str, &str)] = &[
    // Determinantes e pronomes adjetivos.
    ("algum", "alguma"),
    ("aquele", "aquela"),
    ("dum", "duma"),
    ("esse", "essa"),
    ("este", "esta"),
    ("meu", "minha"),
    ("nalgum", "nalguma"),
    ("nenhum", "nenhuma"),
    ("num", "numa"),
    ("seu", "sua"),
    ("teu", "tua"),
    ("um", "uma"),
    // Ordinal e substantivo que a regra de -ona levaria a -ão.
    ("dono", "dona"),
    ("nono", "nona"),
    // Irregulares das gramáticas.
    ("andaluz", "andaluza"),
    ("bom", "boa"),
    ("cru", "crua"),
    ("espanhol", "espanhola"),
    ("ilhéu", "ilhoa"),
    ("judeu", "judia"),
    ("mau", "má"),
    ("nu", "nua"),
    ("réu", "ré"),
    ("sandeu", "sandia"),
    ("tabaréu", "tabaroa"),
    // -ão com feminino em -oa.
    ("beirão", "beiroa"),
    ("patrão", "patroa"),
    // -ão com feminino em -ã que as regras de -ona pegariam.
    ("cibercidadão", "cibercidadã"),
    ("cidadão", "cidadã"),
    ("concidadão", "concidadã"),
    ("sevilhão", "sevilhã"),
    ("chão", "chã"),
    ("rechão", "rechã"),
    // -ão com feminino em -ona que a regra geral (-ã) erraria.
    ("babão", "babona"),
    ("bobão", "bobona"),
    ("bonitão", "bonitona"),
    ("loucão", "loucona"),
    ("malandrão", "malandrona"),
    ("morenão", "morenona"),
    ("pretão", "pretona"),
    ("aldrabão", "aldrabona"),
    ("choramingão", "choramingona"),
    ("esquisitão", "esquisitona"),
    ("estranhão", "estranhona"),
    ("falastrão", "falastrona"),
    ("fracão", "fracona"),
    ("frescão", "frescona"),
    ("gabão", "gabona"),
    ("gritão", "gritona"),
    ("ingratão", "ingratona"),
    ("loirão", "loirona"),
    ("poltrão", "poltrona"),
    ("ruivão", "ruivona"),
    ("tranquilão", "tranquilona"),
    ("brigão", "brigona"),
    ("chorão", "chorona"),
    ("comilão", "comilona"),
    ("durão", "durona"),
    ("fofão", "fofona"),
    ("folião", "foliona"),
    ("fujão", "fujona"),
    ("glutão", "glutona"),
    ("gostosão", "gostosona"),
    ("machão", "machona"),
    ("mandão", "mandona"),
    ("mijão", "mijona"),
    ("mocetão", "mocetona"),
    ("pobretão", "pobretona"),
    ("resmungão", "resmungona"),
    // -ote cujo feminino a regra de -ota devolveria para -oto.
    ("grandote", "grandota"),
    ("lindote", "lindota"),
    ("malandrote", "malandrota"),
    ("acabadote", "acabadota"),
    ("atrevidote", "atrevidota"),
    ("frescalhote", "frescalhota"),
];

/// Uniformes cuja terminação faria a regra flexionar (casam com o último elemento).
const INVARIABLE: &[&str] = &[
    // Comparativos e afins em -or.
    "anterior",
    "citerior",
    "exterior",
    "inferior",
    "interior",
    "júnior",
    "maior",
    "melhor",
    "menor",
    "mor",
    "multicor",
    "pior",
    "posterior",
    "prior",
    "sênior",
    "superior",
    "ulterior",
    // -ês uniformes (Cunha e Cintra).
    "cortês",
    "descortês",
    "montês",
    "pedrês",
    // Substantivos de cor usados como adjetivo.
    "abóbora",
    "cinza",
    "esmeralda",
    "laranja",
    "mostarda",
    "oliva",
    "rosa",
    "salmão",
    "turquesa",
    "vinho",
    "violeta",
    // Uniformes em -a (a direção feminino → masculino tiraria um -o).
    "asteca",
    "babaca",
    "canalha",
    "careca",
    "celta",
    "cosmopolita",
    "croata",
    "extra",
    "hipócrita",
    "idiota",
    "inca",
    "israelita",
    "jesuíta",
    "maia",
    "nômada",
    "panaca",
    "pateta",
    "persa",
    "sacana",
];

/// Terminações uniformes em -a (democrata, homicida, agrícola, poliglota, otimista).
const INVARIABLE_ENDINGS: &[&str] = &["crata", "icida", "ícola", "glota", "patriota", "semita", "ista"];

/// Femininos em -ista que vêm de particípio ou adjetivo em -isto (previsto, misto).
/// Casam como em `matches_any`; os -visto e "mista" só inteiros, porque "grevista",
/// "escravista" e "alarmista" são uniformes.
const ISTA_FROM_ISTO: &[&str] = &[
    "=antevista",
    "benquista",
    "=circunvista",
    "=entrevista",
    "=imprevista",
    "=intervista",
    "malquista",
    "=malvista",
    "=mista",
    "=permista",
    "=impermista",
    "=prevista",
    "=revista",
    "=televista",
    "=vista",
];

/// Femininos em -esa de masculinos em -eso (os demais vêm de -ês: portuguesa). Casam
/// também com prefixo ("reacesa", "indefesa"); os marcados com `=` só inteiros, porque
/// "inglesa", "madrilesa" e "montesa" terminam em "lesa", "ilesa" e "tesa".
const ESA_FROM_ESO: &[&str] = &["acesa", "=adesa", "blesa", "coesa", "defesa", "=ilesa", "=lesa", "obesa", "presa", "=tesa"];

/// Femininos em -eia de masculinos em -eio (os demais vêm de -eu: europeia). "feia" e
/// "meia" só inteiras, porque "ninfeia" e "pigmeia" vêm de "ninfeu" e "pigmeu".
const EIA_FROM_EIO: &[&str] = &["alheia", "cheia", "=feia", "=meia"];

/// Feminino singular a partir do masculino singular ("aceso" → "acesa").
pub fn feminine(masc: &str) -> String {
    let last = last_element(masc);
    let head = &masc[..masc.len() - last.len()];
    if let Some(&(_, fem)) = PAIRS.iter().find(|(m, _)| *m == last) {
        return format!("{head}{fem}");
    }
    if INVARIABLE.contains(&last) || last.ends_with("color") {
        return masc.to_string();
    }
    if let Some(stem) = masc.strip_suffix("ão") {
        // Aumentativos e pejorativos fazem -ona; o resto, -ã (alemã, cristã, sã).
        let augmentative = ["dão", "lhão", "rrão", "eirão", "ntão", "chão"];
        if augmentative.iter().any(|s| masc.ends_with(s)) {
            return format!("{stem}ona");
        }
        return format!("{stem}ã");
    }
    if let Some(stem) = masc.strip_suffix("ote") {
        return format!("{stem}ota");
    }
    if let Some(stem) = masc.strip_suffix('o') {
        return format!("{stem}a");
    }
    if let Some(stem) = masc.strip_suffix("ês") {
        return format!("{stem}esa");
    }
    if let Some(stem) = masc.strip_suffix("eu") {
        return format!("{stem}eia");
    }
    if masc.ends_with("or") {
        return format!("{masc}a");
    }
    masc.to_string()
}

/// Masculino singular a partir do feminino singular ("acesa" → "aceso").
pub fn masculine(fem: &str) -> String {
    let last = last_element(fem);
    let head = &fem[..fem.len() - last.len()];
    if let Some(&(masc, _)) = PAIRS.iter().find(|(_, f)| *f == last) {
        return format!("{head}{masc}");
    }
    if INVARIABLE.contains(&last) {
        return fem.to_string();
    }
    if matches_any(last, ISTA_FROM_ISTO) {
        return format!("{}o", &fem[..fem.len() - 1]);
    }
    if INVARIABLE_ENDINGS.iter().any(|e| last.ends_with(e)) {
        return fem.to_string();
    }
    if let Some(stem) = fem.strip_suffix('ã') {
        return format!("{stem}ão");
    }
    if let Some(stem) = fem.strip_suffix("ona") {
        // Proparoxítonos em -ono (monótona, anglófona, e a grafia sem acento do VERO,
        // hispanofona); o resto é -ão (chorona).
        if has_accent(last_element(stem)) || stem.ends_with("of") {
            return format!("{stem}ono");
        }
        return format!("{stem}ão");
    }
    if let Some(stem) = fem.strip_suffix("esa") {
        if matches_any(last, ESA_FROM_ESO) {
            return format!("{stem}eso");
        }
        return format!("{stem}ês");
    }
    if let Some(stem) = fem.strip_suffix("eia") {
        if matches_any(last, EIA_FROM_EIO) {
            return format!("{stem}eio");
        }
        return format!("{stem}eu");
    }
    if let Some(stem) = fem.strip_suffix("ora") {
        // -oro: -flora, sonora, canora, inodora e os proparoxítonos (carnívora,
        // fósfora); os agentivos em -tora, -dora e -sora são -or mesmo com acento antes
        // (agrário-exportadora, alcândora).
        let from_oro = ["flora", "sonora", "canora", "odora"];
        if from_oro.iter().any(|s| last.ends_with(s)) {
            return format!("{stem}oro");
        }
        if !stem.ends_with(['t', 'd', 's']) && has_accent(last_element(stem)) {
            return format!("{stem}oro");
        }
        return format!("{stem}or");
    }
    if let Some(stem) = fem.strip_suffix('a') {
        return format!("{stem}o");
    }
    fem.to_string()
}

/// O último elemento de um composto com hífen ("luso-brasileiro" → "brasileiro").
fn last_element(word: &str) -> &str {
    word.rsplit('-').next().unwrap_or(word)
}

/// `word` casa com alguma das formas: `=forma` exige a palavra inteira; sem `=`, vale
/// também precedida de prefixo ("reacesa" casa com "acesa").
fn matches_any(word: &str, forms: &[&str]) -> bool {
    forms.iter().any(|f| match f.strip_prefix('=') {
        Some(whole) => word == whole,
        None => word.ends_with(f),
    })
}

/// Há acento gráfico agudo ou circunflexo (sílaba tônica marcada antes do sufixo).
fn has_accent(stem: &str) -> bool {
    stem.chars().any(|c| "áéíóúâêô".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Confere as duas direções de cada par (masculino, feminino).
    fn both(pairs: &[(&str, &str)]) {
        for &(m, f) in pairs {
            assert_eq!(feminine(m), f, "feminine({m})");
            assert_eq!(masculine(f), m, "masculine({f})");
        }
    }

    #[test]
    fn genders() {
        assert_eq!(feminine("emprestado"), "emprestada");
        assert_eq!(feminine("português"), "portuguesa");
        assert_eq!(feminine("par"), "par");
        assert_eq!(masculine("acesa"), "aceso");
    }

    #[test]
    fn participles() {
        both(&[
            ("emprestado", "emprestada"),
            ("confiado", "confiada"),
            ("aceso", "acesa"),
            ("reaceso", "reacesa"),
            ("preso", "presa"),
            ("surpreso", "surpresa"),
            ("eleito", "eleita"),
            ("morto", "morta"),
            ("posto", "posta"),
            ("visto", "vista"),
            ("previsto", "prevista"),
            ("escrito", "escrita"),
            ("aberto", "aberta"),
        ]);
        // Particípio em -e é uniforme.
        both(&[("entregue", "entregue")]);
    }

    #[test]
    fn regular_endings() {
        both(&[
            ("fixo", "fixa"),
            ("francês", "francesa"),
            ("inglês", "inglesa"),
            ("madrilês", "madrilesa"),
            ("europeu", "europeia"),
            ("pigmeu", "pigmeia"),
            ("ninfeu", "ninfeia"),
            ("trabalhador", "trabalhadora"),
            ("agrário-exportador", "agrário-exportadora"),
            ("alemão", "alemã"),
            ("cristão", "cristã"),
            ("são", "sã"),
            ("cidadão", "cidadã"),
            ("grandote", "grandota"),
            ("ótimo", "ótima"),
            ("luso-brasileiro", "luso-brasileira"),
        ]);
    }

    #[test]
    fn ambiguous_feminines() {
        // -esa: -ês ou -eso.
        both(&[("coeso", "coesa"), ("ileso", "ilesa"), ("teso", "tesa"), ("montês", "montês")]);
        assert_eq!(masculine("montesa"), "montês");
        // -eia: -eu ou -eio.
        both(&[("cheio", "cheia"), ("semicheio", "semicheia"), ("feio", "feia"), ("alheio", "alheia")]);
        // -ia: -io, salvo judia e sandia.
        both(&[("frio", "fria"), ("judeu", "judia"), ("sandeu", "sandia")]);
        // -ua: -uo, salvo crua e nua.
        both(&[("contínuo", "contínua"), ("cru", "crua"), ("nu", "nua")]);
        // -ora: -or ou -oro.
        both(&[("sonoro", "sonora"), ("carnívoro", "carnívora"), ("multifloro", "multiflora")]);
        // -ona: -ão (aumentativo) ou -ono (proparoxítono).
        both(&[("chorão", "chorona"), ("cansadão", "cansadona"), ("monótono", "monótona")]);
        both(&[("dono", "dona"), ("nono", "nona")]);
        // -ista: uniforme, salvo -visto e misto.
        both(&[("otimista", "otimista"), ("grevista", "grevista"), ("misto", "mista")]);
    }

    #[test]
    fn irregular_pairs() {
        both(&[
            ("bom", "boa"),
            ("mau", "má"),
            ("réu", "ré"),
            ("ilhéu", "ilhoa"),
            ("espanhol", "espanhola"),
            ("andaluz", "andaluza"),
            ("um", "uma"),
            ("algum", "alguma"),
            ("anglo-judeu", "anglo-judia"),
        ]);
    }

    #[test]
    fn uniform() {
        for w in [
            "feliz",
            "verde",
            "comum",
            "simples",
            "cortês",
            "superior",
            "melhor",
            "incolor",
            "regular",
            "hipócrita",
            "democrata",
            "agrícola",
            "homicida",
            "rosa",
            "vinho",
        ] {
            assert_eq!(feminine(w), w, "feminine({w})");
            assert_eq!(masculine(w), w, "masculine({w})");
        }
    }
}
