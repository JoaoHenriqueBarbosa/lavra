//! Futuro do subjuntivo, 3ª pessoa do singular: a forma da condição ("Se o saldo
//! exceder o teto"). O plural é o singular mais "-em" ("excederem", "forem"), feito
//! fora deste arquivo.
//!
//! O futuro do subjuntivo sai do tema do pretérito perfeito: tira-se o "-am" da 3ª
//! do plural (falaram → falar, fizeram → fizer, puseram → puser). Nos verbos
//! regulares o resultado coincide com o infinitivo; nos irregulares, o tema forte
//! do pretérito vale para a família inteira de derivados (desfizeram → desfizer,
//! compuseram → compuser), e por isso as regras abaixo são por família.
//!
//! Arquivo autocontido (sem dependências do resto do crate), para poder ser testado
//! sozinho contra o dicionário inteiro (`tools/morphology`).

/// Quais prefixos uma família aceita na frente da raiz.
enum Prefixes {
    /// Qualquer prefixo, inclusive nenhum, salvo os listados: a terminação só
    /// aparece em derivados da raiz (todo verbo em "-fazer" vem de "fazer").
    AnyExcept(&'static [&'static str]),
    /// Só os prefixos listados ("" é a raiz sozinha): a terminação também aparece
    /// em verbos que não são da família ("bater" não vem de "ter", "ouvir" não vem
    /// de "vir", "mover" não vem de "ver").
    Only(&'static [&'static str]),
}

/// Famílias de derivados: (terminação do infinitivo, terminação do subjuntivo,
/// prefixos aceitos). A primeira família cuja terminação casa decide.
const FAMILIES: &[(&str, &str, Prefixes)] = &[
    // pôr e derivados (compor, dispor, supor...): puseram → puser. Os derivados
    // perdem o acento ("compor"); "pôr" o mantém.
    ("pôr", "puser", Prefixes::AnyExcept(&[])),
    ("por", "puser", Prefixes::AnyExcept(&[])),
    // fazer, desfazer, satisfazer, liquefazer...: fizeram → fizer.
    ("fazer", "fizer", Prefixes::AnyExcept(&[])),
    // dizer, bendizer, contradizer, predizer...: disseram → disser.
    ("dizer", "disser", Prefixes::AnyExcept(&[])),
    // querer, malquerer, bem-querer: quiseram → quiser. "requerer" é regular
    // (requereram → requerer).
    ("querer", "quiser", Prefixes::AnyExcept(&["re"])),
    // caber, descaber: couberam → couber.
    ("caber", "couber", Prefixes::AnyExcept(&[])),
    // saber, ressaber, dessaber: souberam → souber.
    ("saber", "souber", Prefixes::AnyExcept(&[])),
    // prazer, aprazer, comprazer: prouveram → prouver. "jazer" e "adjazer" são
    // regulares e não terminam em "-prazer".
    ("prazer", "prouver", Prefixes::AnyExcept(&[])),
    // trazer: trouxeram → trouxer.
    ("trazer", "trouxer", Prefixes::AnyExcept(&[])),
    // poder: puderam → puder.
    ("poder", "puder", Prefixes::AnyExcept(&[])),
    // haver e reaver (que perde o "h"): houveram → houver, reouveram → reouver.
    ("aver", "ouver", Prefixes::Only(&["h", "re"])),
    // ter e derivados: tiveram → tiver. Fora da lista ficam os regulares em
    // "-ter" (bater, meter, verter, later).
    (
        "ter",
        "tiver",
        Prefixes::Only(&[
            "", "abs", "a", "con", "de", "entre", "man", "ob", "re", "sus",
        ]),
    ),
    // estar e derivados: estiveram → estiver. Fora ficam prestar, testar,
    // manifestar, protestar...
    (
        "estar",
        "estiver",
        Prefixes::Only(&["", "sob", "sobre", "sobr"]),
    ),
    // vir e derivados: vieram → vier. Fora ficam ouvir e servir (e derivados).
    (
        "vir",
        "vier",
        Prefixes::Only(&[
            "", "a", "ad", "con", "contra", "de", "desa", "descon", "inter", "ob", "pro", "re",
            "recon", "sobre",
        ]),
    ),
    // ver e derivados: viram → vir. Fora ficam prover e desprover (proveram →
    // prover), precaver, viver, mover, escrever, dever...
    (
        "ver",
        "vir",
        Prefixes::Only(&["", "ante", "circun", "entre", "inter", "pre", "re", "tele"]),
    ),
    // dar, desdar, redar: deram → der. Fora ficam andar, mandar, nadar...
    ("dar", "der", Prefixes::Only(&["", "des", "re"])),
    // ser: foram → for. Fora ficam coser e derivados.
    ("ser", "for", Prefixes::Only(&[""])),
    // ir: foram → for (a mesma forma de "ser").
    ("ir", "for", Prefixes::Only(&[""])),
];

/// Exceções que nenhuma família cobre, verbo a verbo.
const IRREGULAR_SUBJUNCTIVE: &[(&str, &str)] = &[
    // "sobresser" (sobre + ser, com o "s" dobrado entre vogais): foram → for. O
    // Verbiste dá "sobresfor", que conserva o "s" dobrado diante de consoante.
    ("sobresser", "sobrefor"),
];

/// "ver" → "vir", "compor" → "compuser", "exceder" → "exceder".
pub fn future_subjunctive(inf: &str) -> String {
    if let Some((_, f)) = IRREGULAR_SUBJUNCTIVE.iter().find(|(k, _)| *k == inf) {
        return f.to_string();
    }
    for (root, subj, prefixes) in FAMILIES {
        let Some(prefix) = inf.strip_suffix(root) else {
            continue;
        };
        let member = match prefixes {
            Prefixes::AnyExcept(except) => !except.contains(&prefix),
            Prefixes::Only(only) => only.contains(&prefix),
        };
        if member {
            return format!("{prefix}{subj}");
        }
    }
    inf.to_string()
}

#[cfg(test)]
mod tests {
    use super::future_subjunctive as f;

    #[test]
    fn regular() {
        assert_eq!(f("exceder"), "exceder");
        assert_eq!(f("falar"), "falar");
        assert_eq!(f("partir"), "partir");
        assert_eq!(f("requerer"), "requerer");
        assert_eq!(f("prover"), "prover");
        assert_eq!(f("desprover"), "desprover");
        assert_eq!(f("precaver"), "precaver");
        assert_eq!(f("bater"), "bater");
        assert_eq!(f("ouvir"), "ouvir");
        assert_eq!(f("servir"), "servir");
        assert_eq!(f("prestar"), "prestar");
        assert_eq!(f("andar"), "andar");
        assert_eq!(f("coser"), "coser");
        assert_eq!(f("jazer"), "jazer");
        assert_eq!(f("adjazer"), "adjazer");
        assert_eq!(f("viver"), "viver");
        assert_eq!(f("previver"), "previver");
    }

    #[test]
    fn roots() {
        assert_eq!(f("ser"), "for");
        assert_eq!(f("ir"), "for");
        assert_eq!(f("ter"), "tiver");
        assert_eq!(f("estar"), "estiver");
        assert_eq!(f("fazer"), "fizer");
        assert_eq!(f("poder"), "puder");
        assert_eq!(f("caber"), "couber");
        assert_eq!(f("saber"), "souber");
        assert_eq!(f("querer"), "quiser");
        assert_eq!(f("trazer"), "trouxer");
        assert_eq!(f("dizer"), "disser");
        assert_eq!(f("vir"), "vier");
        assert_eq!(f("ver"), "vir");
        assert_eq!(f("haver"), "houver");
        assert_eq!(f("dar"), "der");
        assert_eq!(f("pôr"), "puser");
        assert_eq!(f("prazer"), "prouver");
    }

    #[test]
    fn derived() {
        assert_eq!(f("compor"), "compuser");
        assert_eq!(f("pressupor"), "pressupuser");
        assert_eq!(f("satisfazer"), "satisfizer");
        assert_eq!(f("bem-fazer"), "bem-fizer");
        assert_eq!(f("contradizer"), "contradisser");
        assert_eq!(f("malquerer"), "malquiser");
        assert_eq!(f("ressaber"), "ressouber");
        assert_eq!(f("descaber"), "descouber");
        assert_eq!(f("aprazer"), "aprouver");
        assert_eq!(f("conter"), "contiver");
        assert_eq!(f("abster"), "abstiver");
        assert_eq!(f("entreter"), "entretiver");
        assert_eq!(f("sobrestar"), "sobrestiver");
        assert_eq!(f("intervir"), "intervier");
        assert_eq!(f("provir"), "provier");
        assert_eq!(f("revir"), "revier");
        assert_eq!(f("prever"), "previr");
        assert_eq!(f("rever"), "revir");
        assert_eq!(f("antever"), "antevir");
        assert_eq!(f("reaver"), "reouver");
        assert_eq!(f("desdar"), "desder");
        assert_eq!(f("sobresser"), "sobrefor");
        assert_eq!(f("entrequerer"), "entrequiser");
        assert_eq!(f("redarguir"), "redarguir");
        assert_eq!(f("devenir"), "devenir");
    }
}
