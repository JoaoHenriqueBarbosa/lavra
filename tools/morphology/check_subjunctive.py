"""Confere `src/morphology/verbs/subjunctive.rs` contra o Verbiste (via verbecc).

Rodar a partir de qualquer pasta:

    tools/morphology/.venv/bin/python -I tools/morphology/check_subjunctive.py
    tools/morphology/.venv/bin/python -I tools/morphology/check_subjunctive.py --collisions

Compila o harness `harness/subjunctive.rs` (que inclui o arquivo Rust por `#[path]`)
para `target/subjunctive_harness`, passa todos os infinitivos do dicionário por ele
e lista as divergências agrupadas pelo modelo de conjugação do Verbiste. Confere o
singular (a função) e o plural que o crate monta (singular + "em").

Verbos pronominais ("abater-se") entram sem o "-se": quando o verbo simples também
está no dicionário, só se confere que os dois concordam; quando não está, o
pronominal sem o "-se" vira mais um verbo testado.

Com `--collisions`, lista também as ambiguidades: subjuntivo de um verbo igual ao
subjuntivo ou ao infinitivo (pessoal, singular ou plural) de outro, e formas do
subjuntivo que coincidem com palavras do vocabulário da Lavra.
"""

import collections
import pathlib
import subprocess
import sys
import xml.etree.ElementTree as ET

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SOURCE = ROOT / "src/morphology/verbs/subjunctive.rs"
HARNESS = HERE / "harness/subjunctive.rs"
BINARY = HERE / "target/subjunctive_harness"
DATA = HERE / ".venv/lib/python3.13/site-packages/verbecc/data/xml"

# Divergências deliberadas: o Verbiste está errado aqui (ou não tem a forma), e o
# arquivo Rust segue a gramática. Cada uma vai para o relatório como caso de borda,
# não como divergência.
KNOWN_VERBISTE_ERRORS = {
    # Modelo "pr:azer" (aprazer): o Verbiste dá "adjouver", mas "adjazer" vem de
    # "jazer", que é regular (jazeram → jazer).
    "adjazer": "adjazer",
    # Modelo "av:ir" (advir): o Verbiste dá "devenier"; "devenir" não é derivado de
    # "vir" (é galicismo, regular em -ir).
    "devenir": "devenir",
    # Modelo ":ser": o Verbiste dá "sobresfor", conservando o "s" dobrado (que só
    # existe entre vogais) diante de consoante. Sobre + ser → "sobrefor".
    "sobresser": "sobrefor",
    # Modelo "redarg:uir": o Verbiste não tem forma nenhuma; conjuga-se como
    # "arguir" (arguiram → arguir), logo "redarguir".
    "redarguir": "redarguir",
    # Só existe no dicionário como "entrequerer-se", no modelo regular
    # "abat:er-se"; como derivado de "querer", é quiseram → "entrequiser".
    "entrequerer": "entrequiser",
}

# Vocabulário da Lavra que não é verbo (ou que é forma fixa), conferido contra as
# formas do subjuntivo. Extraído das palavras-chave do parser e da semântica.
LAVRA_VOCABULARY = {
    "a", "o", "as", "os", "um", "uma", "uns", "umas", "e", "é", "ou", "não", "se",
    "senão", "que", "de", "da", "do", "das", "dos", "em", "no", "na", "nos", "nas",
    "por", "pelo", "pela", "para", "até", "sem", "cada", "vez", "vezes", "mais",
    "menos", "maior", "menor", "igual", "diferente", "máximo", "mínimo", "for",
    "forem", "fosse", "fossem", "seja", "sejam", "sendo", "está", "estão", "são",
    "ser", "serem", "enquanto", "sempre", "talvez", "nada", "fato", "resultado",
    "verdadeiro", "falso", "inteiro", "inteiros", "natural", "naturais", "real",
    "reais", "lógico", "lógicos", "texto", "textos", "caractere", "caracteres",
    "byte", "bytes", "bits", "fixo", "emprestado", "confiado", "seguido",
    "seguida", "dividido", "módulo", "linha", "início", "começo", "fim", "salvo",
    "ambos", "ambas", "esquerda", "direita", "vira", "falha", "conta",
}

# Verbos frequentes, para destacar as colisões que importam na prática.
COMMON_VERBS = {
    "ser", "ir", "ter", "estar", "ver", "vir", "rever", "revir", "prever",
    "provir", "intervir", "interver", "fazer", "dar", "dizer", "pôr", "poder",
    "querer", "saber", "haver", "trazer", "caber", "manter", "conter", "obter",
    "deter", "reter", "compor", "propor", "supor", "dispor", "expor", "impor",
    "convir", "advir", "antever", "entrever", "sobrevir",
}


def load_reference():
    """Devolve {infinitivo: (singular, plural, modelo)}; formas ausentes são None."""
    verbs = ET.parse(DATA / "verbs/verbs-pt.xml").getroot()
    conjugations = ET.parse(DATA / "conjugations/conjugations-pt.xml").getroot()
    templates = {}
    for t in conjugations.findall("template"):
        persons = t.find("Subjuntivo/futuro").findall("p")
        templates[t.get("name")] = [p.find("i").text for p in persons]
    raw = {}
    for v in verbs.findall("v"):
        inf, name = v.find("i").text, v.find("t").text
        ending = name.split(":")[1]
        stem = inf[: len(inf) - len(ending)]
        forms = templates[name]

        def form(i):
            x = forms[i]
            return None if x in (None, "-") else stem + x

        raw.setdefault(inf, (form(2), form(5), name))
    return raw


def strip_se(x):
    return x[:-3] if x is not None and x.endswith("-se") else x


def split_pronominal(raw):
    """Separa os pronominais. Devolve (referência, conflitos entre pronominal e simples)."""
    ref = {k: v for k, v in raw.items() if not k.endswith("-se")}
    conflicts = []
    for inf, (s, p, name) in raw.items():
        if not inf.endswith("-se"):
            continue
        base = inf[:-3]
        s, p = strip_se(s), strip_se(p)
        if base in ref:
            if (s, p) != ref[base][:2]:
                conflicts.append((inf, (s, p, name), ref[base]))
        else:
            ref[base] = (s, p, name)
    return ref, conflicts


def build():
    BINARY.parent.mkdir(parents=True, exist_ok=True)
    stale = not BINARY.exists() or any(
        f.stat().st_mtime > BINARY.stat().st_mtime for f in (SOURCE, HARNESS)
    )
    if stale:
        subprocess.run(
            ["rustc", "--edition", "2024", "-O", str(HARNESS), "-o", str(BINARY)],
            check=True,
        )


def run(infinitives):
    out = subprocess.run(
        [str(BINARY)],
        input="\n".join(infinitives) + "\n",
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return dict(line.split("\t") for line in out.splitlines())


def report_collisions(ref, got):
    subj = {inf: got[inf] for inf in ref}
    by_form = collections.defaultdict(list)
    for inf, s in subj.items():
        by_form[s].append(inf)
        by_form[s + "em"].append(inf)
    infinitives = set(ref)

    print("\n== Colisões: subjuntivo de A igual ao subjuntivo de B ==")
    same = sorted({tuple(sorted(v)) for v in by_form.values() if len(v) > 1})
    for group in same:
        forms = sorted({subj[i] for i in group})
        mark = " *" if any(i in COMMON_VERBS for i in group) else ""
        print(f"  {'/'.join(forms)}: {', '.join(group)}{mark}")
    print(f"  total: {len(same)}")

    print("\n== Colisões: subjuntivo de A igual ao infinitivo de B (singular e plural) ==")
    pairs = []
    for a, s in subj.items():
        if s == a:
            continue
        if s in infinitives and s != a:
            pairs.append((a, s, s))
    for a, s, b in sorted(pairs):
        mark = " *" if a in COMMON_VERBS or b in COMMON_VERBS else ""
        print(f"  {a} → {s} (= infinitivo de {b}; plural {s}em = infinitivo pessoal plural de {b}){mark}")
    print(f"  total: {len(pairs)}")

    print("\n== Formas do subjuntivo iguais a palavras do vocabulário da Lavra ==")
    hits = []
    for form, verbs in sorted(by_form.items()):
        if form in LAVRA_VOCABULARY:
            hits.append((form, verbs))
            print(f"  {form}: {', '.join(sorted(set(verbs)))}")
    print(f"  total: {len(hits)}")
    print("  (* = envolve verbo frequente)")


def main():
    raw = load_reference()
    ref, pronominal_conflicts = split_pronominal(raw)
    build()
    got = run(sorted(ref))

    divergences = collections.defaultdict(list)
    plural_failures = collections.defaultdict(list)
    defective = []
    known = []
    for inf, (s, p, name) in sorted(ref.items()):
        g = got[inf]
        if inf in KNOWN_VERBISTE_ERRORS:
            expected = KNOWN_VERBISTE_ERRORS[inf]
            if g != expected:
                divergences[name].append(f"{inf}: deu {g}, esperado {expected} (erro conhecido do Verbiste)")
            known.append((inf, name, s, g))
            continue
        if s is None:
            defective.append((inf, name, s, p))
            continue
        if g != s:
            divergences[name].append(f"{inf}: deu {g}, Verbiste {s}")
            continue
        if p is None:
            defective.append((inf, name, s, p))
        elif g + "em" != p:
            plural_failures[name].append(f"{inf}: {g}em, Verbiste {p}")

    total = len(ref)
    n_div = sum(len(v) for v in divergences.values())
    n_plural = sum(len(v) for v in plural_failures.values())
    print(f"Verbos testados: {total} ({len(raw)} entradas no Verbiste, pronominais sem o '-se' fundidos)")
    print(f"Divergências no singular: {n_div}")
    for name, items in sorted(divergences.items(), key=lambda kv: -len(kv[1])):
        print(f"  [{name}] {len(items)}")
        for item in items:
            print(f"    {item}")

    print(f"\nPlural '+em' diferente do Verbiste: {n_plural}")
    for name, items in sorted(plural_failures.items(), key=lambda kv: -len(kv[1])):
        print(f"  [{name}] {len(items)}: {'; '.join(items)}")

    print(f"\nErros conhecidos do Verbiste (o Rust segue a gramática): {len(known)}")
    for inf, name, s, g in known:
        print(f"  [{name}] {inf}: Verbiste {s}, Rust {g}")

    print(f"\nDefectivos no Verbiste (singular ou plural ausente): {len(defective)}")
    for inf, name, s, p in defective:
        print(f"  [{name}] {inf}: singular {s}, plural {p}")

    print(f"\nPronominal discordando do verbo simples: {len(pronominal_conflicts)}")
    for inf, mine, base in pronominal_conflicts:
        print(f"  {inf}: {mine} contra {base}")

    if "--collisions" in sys.argv:
        report_collisions(ref, got)

    sys.exit(1 if n_div else 0)


if __name__ == "__main__":
    main()
