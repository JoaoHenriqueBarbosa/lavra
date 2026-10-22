#!/usr/bin/env python3
"""Confere `src/morphology/adjectives/feminine.rs` contra o Hunspell pt_BR e o Verbiste.

Rodar de dentro de `tools/morphology`:

    .venv/bin/python -I check_feminine.py            # placar e divergências agrupadas
    .venv/bin/python -I check_feminine.py --show esa # lista todas as divergências cujo
                                                     # grupo contém "esa"
    .venv/bin/python -I check_feminine.py --dump ref.tsv  # grava a referência

Monta pares (masculino, feminino) no singular a partir de seis fontes, compila o
harness `harness/feminine.rs` e confere as duas direções: `feminine(m)` precisa cair
num dos femininos de referência de `m`, e `masculine(f)` num dos masculinos de `f`.
Quando a referência tem mais de uma resposta (folgazã e folgazona; presa de preso e
de presar), qualquer uma vale, e a ambiguidade é contada à parte.

Fontes (cada par leva a sua etiqueta):

- `flexão`: radicais do .dic com as flags de gênero do VERO (D, F, E, G), expandidos
  só pelas regras que produzem feminino singular; `flexão+prefixo` repete com os
  prefixos que o radical aceita (re-, des-, anti-...), para testar casamento por sufixo.
- `particípio`: o particípio de cada verbo do Verbiste (regular ou irregular), mais o
  regular quando o Hunspell o reconhece (abundantes: aceso e acendido).
- `agente`: verbos com a flag j do VERO, X-dor e X-dora.
- `par lexical`: dois radicais do .dic ligados por alternância marcada
  (chorão/chorona, judeu/judia, cru/crua, mau/má...).
- `aumentativo`: X-ão e X-ona gerados do mesmo radical (V/W, L, j).
- `uniforme`: adjetivos sem flexão de gênero (marca adjetival J, H, X, I ou K, sem
  flag de gênero nem par), e os -ista das flags 2 e 3; ambas as direções devolvem a
  própria palavra.

Radicais com maiúscula (nomes próprios) ficam de fora. As exclusões documentadas
(ruído do VERO) estão em `EXCLUDED` e aparecem contadas no relatório.
"""

from __future__ import annotations

import argparse
import collections
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from spylls.hunspell import Dictionary

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SOURCE = ROOT / "src/morphology/adjectives/feminine.rs"
HARNESS = HERE / "harness/feminine.rs"
BINARY = HERE / "target/feminine_harness"
VERBECC = HERE / ".venv/lib/python3.13/site-packages/verbecc/data/xml"

# Regras do .aff que produzem feminino singular, por (flag, strip, add).
FEMININE_RULES = {
    ("D", "", "a"),
    ("D", "e", "a"),
    ("D", "ês", "esa"),
    ("D", "o", ""),
    ("D", "o", "a"),
    ("D", "u", "ia"),
    ("F", "", "a"),
    ("F", "ão", "ã"),
    ("F", "e", "a"),
    ("F", "o", "a"),
    ("F", "u", "ia"),
    ("E", "ão", "ã"),
    ("G", "o", ""),
}
GENDER_FLAGS = {"D", "F", "E", "G"}
ADJECTIVE_MARKERS = {"J", "H", "X", "I"}

# Alternâncias marcadas entre dois radicais do .dic (masculino, feminino). Só as que
# o .dic não polui com substantivos: -ão/-oa, -u/-ua, -au/-á, -éu/-é e -eu/-ia casam
# sobretudo pares acidentais (coroa, tabua, pá, maré, quantia) e ficam em CURATED.
LEXICAL_PATTERNS = [
    ("ão", "ã"),
    ("ão", "ona"),
    ("ês", "esa"),
    ("eu", "eia"),
    ("ol", "ola"),
    ("uz", "uza"),
    ("or", "ora"),
]

# Pares adjetivos que o VERO não liga (ou liga errado), conforme as gramáticas
# (Cunha e Cintra, cap. 10; Bechara, flexão do adjetivo).
CURATED = [
    ("bom", "boa"),
    ("mau", "má"),
    ("cru", "crua"),
    ("nu", "nua"),
    ("judeu", "judia"),
    ("sandeu", "sandia"),
    ("ilhéu", "ilhoa"),
    ("tabaréu", "tabaroa"),
    ("réu", "ré"),
    ("português", "portuguesa"),
    ("chorão", "chorona"),
    ("comilão", "comilona"),
    ("patrão", "patroa"),
    ("beirão", "beiroa"),
    ("espanhol", "espanhola"),
    ("andaluz", "andaluza"),
    ("nono", "nona"),
    ("dono", "dona"),
    # Determinantes e pronomes adjetivos (o VERO marca um/algum com D, que soma -a a
    # todo -m e por isso sai da referência; ver EXCLUDED).
    ("um", "uma"),
    ("algum", "alguma"),
    ("nenhum", "nenhuma"),
    ("num", "numa"),
    ("dum", "duma"),
    ("nalgum", "nalguma"),
    ("este", "esta"),
    ("esse", "essa"),
    ("aquele", "aquela"),
    ("meu", "minha"),
    ("teu", "tua"),
    ("seu", "sua"),
]

# Adjetivos em -ote; os demais -ote com flag D no VERO são substantivos (camarote,
# caixote, glote) e saem da referência.
ADJECTIVES_OTE = {"acabadote", "atrevidote", "frescalhote", "grandote", "lindote", "malandrote"}

# Masculinos em -isto que existem (particípios de ver e compostos, misto, malquisto);
# os outros -isto com flag D no VERO (ametisto, cristo → crista) são ruído.
REAL_ISTO = {"misto", "permisto", "impermisto", "malquisto", "benquisto"}

COMPARATIVES = {
    "anterior", "posterior", "superior", "inferior", "interior", "exterior",
    "ulterior", "citerior", "maior", "menor", "melhor", "pior",
}

# Ruído do VERO excluído da referência: (fonte, predicado sobre (masc, fem), motivo).
# O relatório mostra quantos pares cada motivo tirou.
EXCLUDED: list[tuple[str, object, str]] = [
    (
        "*",
        lambda m, f: m.endswith("e") and f == m[:-1] + "a" and not m.endswith("ote"),
        "flag D/F em -e (minerais em -ite, mestre, presidente): substantivos; adjetivo em -e é uniforme",
    ),
    (
        "*",
        lambda m, f: f == m + "a" and m.endswith(("ar", "er", "ir", "ur")),
        "flag D/F em -ar/-er/-ir/-ur (jaguar, tapir): substantivos; adjetivo em -ar é uniforme",
    ),
    (
        "*",
        lambda m, f: f == m + "a" and m.endswith("m"),
        "flag D em -m soma -a (bom → boma, fim → fima); os pares de -m vêm de CURATED",
    ),
    (
        "*",
        lambda m, f: m.endswith("u") and not m.endswith("eu") and f == m[:-1] + "ia",
        "flag D (u → ia) em palavra sem -eu (guru → guria, piau → piaia)",
    ),
    (
        "flexão",
        lambda m, f: m.endswith("ote") and m not in ADJECTIVES_OTE,
        "flag D em substantivo -ote (camarote → camarota, glote → glota)",
    ),
    (
        "flexão",
        lambda m, f: m.endswith("isto") and not m.endswith("visto") and m not in REAL_ISTO,
        "flag D em -isto inexistente (ametisto, cristo → crista, abatisto)",
    ),
    (
        "*",
        lambda m, f: m.rsplit("-", 1)[-1] in COMPARATIVES and f != m,
        "comparativo uniforme (superior → superiora é o substantivo, madre superiora)",
    ),
]

def apply_suffix(rule, stem: str) -> str | None:
    if not rule.cond_regexp.search(stem):
        return None
    if rule.strip and not stem.endswith(rule.strip):
        return None
    base = stem[: len(stem) - len(rule.strip)] if rule.strip else stem
    return base + rule.add


def apply_prefix(rule, stem: str) -> str | None:
    if not rule.cond_regexp.search(stem):
        return None
    if rule.strip and not stem.startswith(rule.strip):
        return None
    return rule.add + stem[len(rule.strip) :]


def participle_templates() -> dict[str, list[str]]:
    tree = ET.parse(VERBECC / "conjugations/conjugations-pt.xml")
    out: dict[str, list[str]] = {}
    for template in tree.getroot():
        name = template.get("name")
        endings = []
        for mood in template:
            if mood.tag != "Particípio":
                continue
            for tense in mood:
                for p in tense:
                    for i in p:
                        if i.text is not None:
                            endings.append(i.text.strip())
        out[name] = endings
    return out


def verbiste_participles() -> list[tuple[str, str]]:
    templates = participle_templates()
    tree = ET.parse(VERBECC / "verbs/verbs-pt.xml")
    out = []
    for v in tree.getroot():
        infinitive = v.findtext("i")
        template = v.findtext("t")
        if not infinitive or not template or infinitive.endswith("-se"):
            continue
        termination = template.split(":", 1)[1]
        if not infinitive.endswith(termination):
            continue
        radical = infinitive[: len(infinitive) - len(termination)]
        for ending in templates.get(template, []):
            out.append((infinitive, radical + ending))
    return out


def verbiste_infinitives() -> set[str]:
    tree = ET.parse(VERBECC / "verbs/verbs-pt.xml")
    return {
        v.findtext("i")
        for v in tree.getroot()
        if v.findtext("i") and not v.findtext("i").endswith("-se")
    }


def regular_participle(infinitive: str) -> str | None:
    if infinitive.endswith("ar"):
        return infinitive[:-2] + "ado"
    if infinitive.endswith(("er", "ir")):
        return infinitive[:-2] + "ido"
    return None


def feminine_of_participle(masc: str) -> str:
    return masc[:-1] + "a" if masc.endswith("o") else masc


class Reference:
    def __init__(self) -> None:
        # (masc, fem) -> conjunto de fontes
        self.pairs: dict[tuple[str, str], set[str]] = collections.defaultdict(set)
        self.excluded: collections.Counter[str] = collections.Counter()
        self.excluded_examples: dict[str, list[str]] = collections.defaultdict(list)

    def add(self, masc: str, fem: str, source: str) -> None:
        for src, predicate, reason in EXCLUDED:
            if (src == "*" or source.startswith(src)) and predicate(masc, fem):
                self.excluded[reason] += 1
                if len(self.excluded_examples[reason]) < 8:
                    self.excluded_examples[reason].append(f"{masc}/{fem}")
                return
        self.pairs[(masc, fem)].add(source)


def build_reference() -> Reference:
    d = Dictionary.from_files(str(HERE / "data/pt_BR"))
    sfx, pfx = d.aff.SFX, d.aff.PFX
    ref = Reference()
    words = [w for w in d.dic.words if w.stem and not w.stem[0].isupper()]
    stems = {w.stem for w in words}
    flags_of: dict[str, set[str]] = collections.defaultdict(set)
    for w in words:
        flags_of[w.stem] |= set(w.flags)

    # flexão (+ prefixo)
    for w in words:
        for flag in sorted(set(w.flags) & GENDER_FLAGS):
            for rule in sfx.get(flag, []):
                if (flag, rule.strip, rule.add) not in FEMININE_RULES:
                    continue
                fem = apply_suffix(rule, w.stem)
                if fem is None:
                    continue
                ref.add(w.stem, fem, "flexão")
                if not rule.crossproduct:
                    continue
                for pflag in w.flags:
                    for prule in pfx.get(pflag, []):
                        if not prule.crossproduct:
                            continue
                        pm = apply_prefix(prule, w.stem)
                        if pm is None:
                            continue
                        ref.add(pm, apply_prefix(prule, fem) or prule.add + fem, "flexão+prefixo")

    # particípio
    with_participle = set()
    for infinitive, part in verbiste_participles():
        with_participle.add(infinitive)
        ref.add(part, feminine_of_participle(part), "particípio")
        regular = regular_participle(infinitive)
        if regular and regular != part and d.lookup(regular):
            ref.add(regular, feminine_of_participle(regular), "particípio")
    # Modelos do Verbiste com o particípio vazio (aceit:ar): vale o regular.
    for infinitive in verbiste_infinitives() - with_participle:
        regular = regular_participle(infinitive)
        if regular:
            ref.add(regular, feminine_of_participle(regular), "particípio")

    # agente
    for w in words:
        if "j" in w.flags and w.stem.endswith("r"):
            ref.add(w.stem[:-1] + "dor", w.stem[:-1] + "dora", "agente")

    # par lexical
    for s in stems:
        for ms, fs in LEXICAL_PATTERNS:
            if s.endswith(ms):
                f = s[: len(s) - len(ms)] + fs
                if f in stems and f != s:
                    ref.add(s, f, "par lexical")
    for m, f in CURATED:
        ref.add(m, f, "par curado")

    # aumentativo
    for w in words:
        forms = set()
        for flag in w.flags:
            for rule in sfx.get(flag, []):
                form = apply_suffix(rule, w.stem)
                if form:
                    forms.add(form)
        for form in forms:
            if form.endswith("ão") and form[:-2] + "ona" in forms:
                ref.add(form, form[:-2] + "ona", "aumentativo")

    # uniforme
    paired = {m for m, _ in ref.pairs} | {f for _, f in ref.pairs}
    for s in sorted(stems):
        fl = flags_of[s]
        if fl & GENDER_FLAGS or not fl & ADJECTIVE_MARKERS:
            continue
        if s in paired or s.endswith("o") or " " in s:
            continue
        ref.add(s, s, "uniforme")
    for w in words:
        for flag in {"2", "3"} & set(w.flags):
            for rule in sfx[flag]:
                form = apply_suffix(rule, w.stem)
                if form and form.endswith("ista") and form not in paired:
                    ref.add(form, form, "uniforme -ista")
    return ref


def build_harness() -> None:
    BINARY.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        ["rustc", "--edition", "2024", "-O", str(HARNESS), "-o", str(BINARY)],
        check=True,
    )


def run_harness(queries: list[tuple[str, str]]) -> list[str]:
    data = "".join(f"{d}\t{w}\n" for d, w in queries)
    out = subprocess.run([str(BINARY)], input=data, capture_output=True, text=True, check=True)
    lines = out.stdout.split("\n")[:-1]
    assert len(lines) == len(queries), (len(lines), len(queries))
    return lines


def change(word: str, target: str) -> str:
    """Descreve a troca de terminação, como "-esa → -eso"."""
    i = 0
    while i < min(len(word), len(target)) and word[i] == target[i]:
        i += 1
    i = min(i, max(0, len(word) - 3), max(0, len(target) - 3))
    return f"-{word[i:]} → -{target[i:]}"


KNOWN_FILE = HERE / "known_feminine.tsv"

# Motivos das divergências aceitas em `known_feminine.tsv` (direção, palavra, motivo).
REASONS = {
    "ao-ona": "-ão: a regra dá -ã e o VERO liga a -ona (aumentativo de substantivo: balão/balona, vagão/vagona)",
    "ao-a": "-ão: a regra de aumentativo dá -ona e o VERO dá -ã pela flag G (ferrão, cantão, ramalhão)",
    "ona-ono": "-ona: a regra dá -ão (chorona) e o VERO tem substantivo em -ono (mamona, trombona, acetona)",
    "eia-eio": "-eia: a regra dá -eu (europeia) e o VERO tem substantivo em -eio (correia, teia, centeia)",
    "esa-eso": "-esa: a regra dá -ês e o VERO tem -eso raro ou substantivo (mesa, teresa, equeso)",
    "ora-oro": "-ora: o VERO tem substantivo em -oro ou -or que a regra não alcança (pandora, flora de flor)",
    "es-uniforme": "-ês uniforme pelas gramáticas (montês, pedrês); o VERO flexiona",
    "a-uniforme": "uniforme em -a pela regra ou tabela (poliglota, -ícola, -crata, cores); o VERO tem -o",
    "color": "-color uniforme (incolor, bicolor); o VERO flexiona albicolor",
    "vero": "ruído isolado do VERO",
}


def load_known() -> dict[tuple[str, str], str]:
    known = {}
    if KNOWN_FILE.exists():
        for line in KNOWN_FILE.read_text().splitlines():
            if not line or line.startswith("#"):
                continue
            direction, word, reason = line.split("\t")
            assert reason in REASONS, reason
            known[(direction, word)] = reason
    return known


def ambiguity_table(mascs: dict[str, set[str]]) -> None:
    """Mede, para cada terminação feminina ambígua, de que masculinos ela vem."""
    endings = {
        "esa": ["ês", "eso"],
        "eia": ["eu", "eio"],
        "ia": ["io", "eu"],
        "ua": ["uo", "u"],
        "ora": ["or", "oro"],
        "ona": ["ão", "ono"],
        "ota": ["oto", "ote"],
        "oa": ["oo", "om", "ão", "éu"],
        "ã": ["ão"],
        "ista": ["ista", "isto"],
    }
    print("\nterminações femininas ambíguas (quantos femininos vêm de cada masculino):")
    for fe, classes in endings.items():
        c = collections.Counter()
        for f, ms in mascs.items():
            if not f.endswith(fe) or (fe == "ia" and f.endswith("eia")):
                continue
            for m in ms:
                k = next((mc for mc in classes if m.endswith(mc)), "outro")
                c[k] += 1
        print(f"  -{fe:5} " + ", ".join(f"{k}: {n}" for k, n in c.most_common()))
    c = collections.Counter()
    for f, ms in mascs.items():
        if f.endswith("a") and not f.endswith(("ista", "esa", "eia", "ia", "ua", "ora", "ona", "ota", "oa")):
            for m in ms:
                c["uniforme" if m == f else "-o" if m.endswith("o") else "outro"] += 1
    print("  -a (demais) " + ", ".join(f"{k}: {n}" for k, n in c.most_common()))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--show", help="lista todas as divergências cujo grupo contém o texto")
    ap.add_argument("--dump", help="grava a referência em TSV (masc, fem, fontes)")
    ap.add_argument("--known", action="store_true", help="lista também as divergências documentadas")
    ap.add_argument("--examples", type=int, default=6)
    args = ap.parse_args()

    ref = build_reference()
    if args.dump:
        with open(args.dump, "w") as fh:
            for (m, f), srcs in sorted(ref.pairs.items()):
                fh.write(f"{m}\t{f}\t{','.join(sorted(srcs))}\n")

    fems: dict[str, set[str]] = collections.defaultdict(set)
    mascs: dict[str, set[str]] = collections.defaultdict(set)
    sources: dict[tuple[str, str], set[str]] = collections.defaultdict(set)
    for (m, f), srcs in ref.pairs.items():
        fems[m].add(f)
        mascs[f].add(m)
        sources[("feminine", m)] |= srcs
        sources[("masculine", f)] |= srcs

    build_harness()
    m_words, f_words = sorted(fems), sorted(mascs)
    answers = run_harness([("m", w) for w in m_words] + [("f", w) for w in f_words])
    queries = [("feminine", w, fems[w]) for w in m_words] + [("masculine", w, mascs[w]) for w in f_words]

    known = load_known()
    seen_known = set()
    groups: dict[tuple[str, str], list[str]] = collections.defaultdict(list)
    documented: dict[str, list[str]] = collections.defaultdict(list)
    per_source = collections.Counter()
    for (direction, word, expected), got in zip(queries, answers):
        if got in expected:
            continue
        line = f"{word} → {got} (esperado {'/'.join(sorted(expected))}; {','.join(sorted(sources[(direction, word)]))})"
        if (direction, word) in known:
            seen_known.add((direction, word))
            documented[known[(direction, word)]].append(f"[{direction}] {line}")
            continue
        exp = sorted(expected)[0]
        groups[(direction, f"{change(word, exp)}   (deu {change(word, got)})")].append(line)
        for s in sources[(direction, word)]:
            per_source[(direction, s)] += 1

    by_source = collections.Counter()
    for srcs in ref.pairs.values():
        for s in srcs:
            by_source[s] += 1
    print(f"pares de referência: {len(ref.pairs)}")
    for s, n in by_source.most_common():
        print(f"  {s:16} {n:7}  divergências: feminine {per_source[('feminine', s)]}, masculine {per_source[('masculine', s)]}")
    print(f"consultas: feminine {len(m_words)}, masculine {len(f_words)}")
    amb_m = sum(1 for f in f_words if len(mascs[f]) > 1)
    amb_f = sum(1 for m in m_words if len(fems[m]) > 1)
    print(f"femininos com mais de um masculino: {amb_m}; masculinos com mais de um feminino: {amb_f}")
    if ref.excluded:
        print("\nexcluídos da referência (ruído do VERO):")
        for reason, n in ref.excluded.most_common():
            print(f"  {n:6}  {reason}\n          ex.: {', '.join(ref.excluded_examples[reason])}")
    ambiguity_table(mascs)

    print("\ndivergências documentadas (known_feminine.tsv):")
    for reason, items in sorted(documented.items(), key=lambda kv: -len(kv[1])):
        print(f"  {len(items):4}  {REASONS[reason]}")
        if args.known:
            for item in items:
                print(f"          {item}")
    stale = sorted(set(known) - seen_known)
    if stale:
        print(f"\nentradas de known_feminine.tsv que já não divergem ({len(stale)}): "
              + ", ".join(f"{d}:{w}" for d, w in stale))

    total = sum(len(v) for v in groups.values())
    print(f"\ndivergências não documentadas: {total}")
    for (direction, key), items in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        if args.show and args.show not in key:
            continue
        print(f"\n[{direction}] {key}: {len(items)}")
        limit = None if args.show else args.examples
        for item in items[:limit]:
            print(f"    {item}")
    return 0 if total == 0 and not stale else 1


if __name__ == "__main__":
    sys.exit(main())
