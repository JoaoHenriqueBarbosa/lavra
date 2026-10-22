"""Confere `src/morphology/nouns/plural.rs` contra o Hunspell pt_BR (VERO).

Uso (de dentro de tools/morphology):

    .venv/bin/python -I check_plural.py                # placar e divergências por terminação
    .venv/bin/python -I check_plural.py --show ão      # todas as divergências de uma terminação
    .venv/bin/python -I check_plural.py --dump out.tsv # todas as divergências num TSV
    .venv/bin/python -I check_plural.py --edge         # relatório de casos de borda

Como a referência é montada:

1. Para cada radical do `.dic` com flag de plural (A, B, C, D, E, G), aplica as
   regras de sufixo que formam o plural do masculino (ou da forma base). As regras de
   gênero dessas flags dão também pares feminino singular e feminino plural.
2. Muitos plurais aparecem no `.dic` como entradas avulsas, sem flag ("abomináveis",
   "juniores", "hifens"). Para um radical sem plural por flag, os candidatos de plural
   que existem como entrada avulsa contam como plural aceito ("lacuna"). Para um
   radical que já tem plural por flag, eles só aparecem como variante ("alt"), sem
   contar como acerto.
3. Palavras terminadas em -s ou -x, paroxítonas ou proparoxítonas, sem plural nenhum
   no dicionário, contam como invariáveis ("lápis", "tórax").
4. Compostos com hífen: o plural é a entrada do `.dic` que tem as mesmas partes, cada
   uma igual à do singular ou no plural dela ("amor-perfeito" e "amores-perfeitos").

Um plural da Lavra conta como certo se estiver entre os plurais aceitos.
"""

from __future__ import annotations

import argparse
import itertools
import os
import subprocess
import sys
from collections import Counter, defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
TARGET = os.path.join(HERE, "target")
HARNESS_SRC = os.path.join(HERE, "harness", "plural.rs")
HARNESS_BIN = os.path.join(TARGET, "plural_harness")
PLURAL_RS = os.path.join(ROOT, "src", "morphology", "nouns", "plural.rs")

PLURAL_FLAGS = set("ABCDEG")

# Regras (flag, strip, add) que formam o plural do masculino ou da forma base.
MASC_PLURAL = {
    "A": {("o", "es")},
    "B": None,  # todas
    "C": None,  # todas
    "D": {("", "es"), ("", "s"), ("ês", "eses"), ("m", "ns")},
    "E": {("o", "es")},
    "G": {("ão", "ões")},
}
# Regras que formam o feminino singular; o feminino plural é o mesmo com -s.
FEM_SINGULAR = {
    "A": {("", "a")},
    "D": {("", "a"), ("e", "a"), ("ês", "esa"), ("o", "a"), ("u", "ia"), ("o", "")},
    "E": {("ão", "ã")},
    "G": {("o", "")},
}

# Palavras funcionais que nunca contam como plural de nada.
CLOSED_CLASS = {
    "as", "os", "às", "aos", "das", "dos", "nas", "nos", "pelas", "pelos", "mais",
    "menos", "pois", "depois", "após", "atrás", "detrás", "lhes", "vos", "nós", "vós",
    "eles", "elas", "estes", "estas", "esses", "essas", "aqueles", "aquelas", "meus",
    "teus", "seus", "minhas", "tuas", "suas", "nossos", "nossas", "vossos", "vossas",
    "uns", "umas", "quais", "tais", "todos", "todas", "outros", "outras", "muitos",
    "muitas", "poucos", "poucas", "vários", "várias", "ambos", "ambas", "dois", "duas",
    "três", "quantos", "quantas", "tantos", "tantas", "cujos", "cujas", "demais",
    "jamais", "apenas", "antes", "através", "talvez", "lás", "cás",
}

# Palavras em que o VERO contradiz a gramática e o VOLP (o plural da Lavra entra como
# aceito, com o motivo). Cada uma está no relatório de casos de borda.
GRAMMAR_OVERRIDES = {
    "desvão": ("desvãos", "VOLP: desvão, pl. desvãos (o VERO dá desvões)"),
    "amilálcool": ("amilálcoois", "plural de álcool, que o VERO dá como álcoois"),
    "aminoálcool": ("aminoálcoois", "idem"),
    "hexálcool": ("hexálcoois", "idem"),
    "hidroálcool": ("hidroálcoois", "idem"),
    "hidrálcool": ("hidrálcoois", "idem"),
    "poliálcool": ("poliálcoois", "idem"),
    "tioálcool": ("tioálcoois", "idem"),
    "triálcool": ("triálcoois", "idem"),
}

STRESS_MARKS = "áéíóúâêô"


def strip_stress(s: str) -> str:
    """Tira só acento agudo e circunflexo (o til é nasalidade, fica)."""
    table = str.maketrans("áâéêíóôú", "aaeeioou")
    return s.translate(table)


def is_oxytone(w: str) -> bool:
    """Tonicidade pela regra de acentuação gráfica (vale para a grafia padrão)."""
    if any(c in STRESS_MARKS for c in w):
        # A sílaba do acento é a tônica: oxítona se não há vogal depois dela,
        # tirando um ditongo final ("herói", "chapéu") e o -m/-ns/-s.
        i = max(i for i, c in enumerate(w) if c in STRESS_MARKS)
        rest = w[i + 1:]
        return all(c not in "aeiouãõ" for c in rest) or rest in ("i", "u", "is", "us", "m", "ns")
    if w.endswith(("a", "e", "o", "as", "es", "os", "am", "em", "ens")):
        return w.endswith(("ão", "ãos", "ãe", "ães", "õe", "ões", "ã", "ãs"))
    return True


def load_dictionary():
    from spylls.hunspell import Dictionary

    return Dictionary.from_files(os.path.join(HERE, "data", "pt_BR"))


def apply(stem, suffix):
    if not suffix.cond_regexp.search(stem):
        return None
    if suffix.strip and not stem.endswith(suffix.strip):
        return None
    base = stem[: len(stem) - len(suffix.strip)] if suffix.strip else stem
    return base + suffix.add


def candidates(s: str) -> set[str]:
    """Candidatos generosos de plural, só para achar entradas avulsas no `.dic`."""
    out = {s + "s", s + "es", strip_stress(s) + "es"}
    if s.endswith("en"):
        out.add(strip_stress(s) + "s")
    if s.endswith("ão"):
        out |= {s[:-2] + x for x in ("ões", "ães", "ãos")}
    if s.endswith("l"):
        out |= {s[:-1] + "is", strip_stress(s[:-1]) + "is"}
        if s.endswith("el"):
            out |= {s[:-2] + "éis", s[:-2] + "eis"}
        if s.endswith("ol"):
            out |= {s[:-2] + "óis", strip_stress(s[:-2]) + "óis"}
        if s.endswith("il"):
            out |= {s[:-1] + "s", s[:-2] + "eis"}
    if s.endswith("m"):
        out.add(s[:-1] + "ns")
    return {c for c in out if c != s}


def hunspell_fix(stem: str, plural: str) -> tuple[str, str | None]:
    """Corrige os plurais que as flags do VERO geram contra a ortografia de 1990.

    Devolve o plural corrigido e o nome da correção (None se não mexeu). Cada caso é
    uma flag aplicada a um radical que ela não cobre:

    - oxítona-el: a flag B tira o acento do ditongo aberto da oxítona, que o Acordo
      mantém (Base VIII): "pixeis" → "pixéis", "sorbitois" → "sorbitóis".
    - paroxítona-el: a flag C acentua o ditongo da paroxítona: "níquéis" → "níqueis".
    - oxítona-s: a flag B soma -es sem tirar o acento: "benoxafóses" → "benoxafoses".
    - paroxítona-s: a flag B soma -es a paroxítona em -s, que é invariável:
      "ábises" → "ábis", "forônidases" → "forônidas".
    - paroxítona-en: a flag B mantém o acento no -ens: "abdômens" → "abdomens"
      (Base IX: paroxítona em -ens não se acentua).
    - paroxítona-il: a flag B dá -is à paroxítona: "hipocontrátis" → "hipocontráteis".
    - paroxítona-ior: a tônica passa ao o: "júniores" → "juniores".
    - caráter: "caráteres" → "caracteres".
    - paroxítona-ão: a flag B dá -ões à paroxítona em -ão, que faz -ãos: "zângões" →
      "zângãos", "claviórgões" → "claviórgãos".
    - consoante-l: a flag B troca -l por -is depois de consoante: "argylis" →
      "argylls", "horsebalis" → "horseballs".
    - consoante-m: a flag B troca -m por -ns depois de consoante, grafia que o
      português não tem: "napalns" → "napalms", "megaohns" → "megaohms".

    Em composto com hífen a flag só mexe na última parte, e a correção também.
    """
    if "-" in stem:
        head, tail = stem.rsplit("-", 1)
        if not plural.startswith(head + "-"):
            return plural, None
        new, kind = hunspell_fix(tail, plural[len(head) + 1:])
        return head + "-" + new, kind
    ox = is_oxytone(stem)
    if stem.endswith("ão") and not ox and plural == stem[:-2] + "ões":
        return stem + "s", "paroxítona-ão"
    if stem.endswith("l") and len(stem) > 1 and stem[-2] not in "aeiouáéíóúâêô" and plural == stem[:-1] + "is":
        return stem + "s", "consoante-l"
    if stem.endswith("m") and len(stem) > 1 and stem[-2] not in "aeiouáéíóúâêôãõü" and plural == stem[:-1] + "ns":
        return stem + "s", "consoante-m"
    if stem.endswith(("el", "ol")) and plural == stem[:-1] + "is" and ox:
        return stem[:-2] + ("éis" if stem.endswith("el") else "óis"), "oxítona-el"
    if stem.endswith(("el", "ol")) and plural in (stem[:-2] + "éis", stem[:-2] + "óis") and not ox:
        return plural[:-3] + plural[-3:].replace("é", "e").replace("ó", "o"), "paroxítona-el"
    if stem.endswith("s") and plural == stem + "es":
        if ox and stem[-2] in "áéóâêô":
            return stem[:-2] + strip_stress(stem[-2]) + "ses", "oxítona-s"
        if not ox:
            return stem, "paroxítona-s"
    if stem.endswith("en") and not ox and plural == stem + "s":
        return strip_stress(stem) + "s", "paroxítona-en"
    if stem.endswith("il") and not ox and plural == stem[:-1] + "s":
        return stem[:-2] + "eis", "paroxítona-il"
    if stem.endswith("ior") and not ox and plural == stem + "es":
        return strip_stress(stem) + "es", "paroxítona-ior"
    if stem == "caráter" and plural == "caráteres":
        return "caracteres", "caráter"
    return plural, None


class Reference:
    def __init__(self):
        d = load_dictionary()
        self.dic = d
        sfx = d.aff.SFX
        self.flagged: dict[str, set[str]] = defaultdict(set)  # plurais por flag
        self.source: dict[str, str] = {}  # de onde veio o singular
        self.entry_flags: dict[str, set[str]] = defaultdict(set)
        generated_plurals: set[str] = set()
        fem_pairs: dict[str, set[str]] = defaultdict(set)
        for w in d.dic.words:
            self.entry_flags[w.stem] |= set(w.flags)
        for w in d.dic.words:
            stem = w.stem
            if not stem[:1].islower() or "." in stem or len(stem) < 2:
                continue
            flags = set(w.flags) & PLURAL_FLAGS
            for fl in flags:
                fem_sg, fem_pl = set(), set()
                for s in sfx[fl]:
                    form = apply(stem, s)
                    if form is None:
                        continue
                    allowed = MASC_PLURAL[fl]
                    if allowed is None or (s.strip, s.add) in allowed:
                        if allowed is not None and fl == "D" and s.add == "s" and not stem.endswith(("o", "e", "u")):
                            continue
                        self.flagged[stem].add(form)
                        generated_plurals.add(form)
                    if (s.strip, s.add) in FEM_SINGULAR.get(fl, ()):
                        fem_sg.add(form)
                    elif s.add.endswith("s"):
                        fem_pl.add(form)
                for f in fem_sg:
                    if f + "s" in fem_pl and f != stem:
                        fem_pairs[f].add(f + "s")
        # Correções de flag mal aplicada (ver hunspell_fix).
        self.fixes: Counter = Counter()
        self.fix_examples: dict[str, list] = defaultdict(list)
        for stem, pls in self.flagged.items():
            fixed = set()
            for pl in pls:
                new, kind = hunspell_fix(stem, pl)
                if kind:
                    self.fixes[kind] += 1
                    if len(self.fix_examples[kind]) < 4:
                        self.fix_examples[kind].append(f"{stem}: {pl} → {new}")
                fixed.add(new)
            self.flagged[stem] = fixed
        self.generated_plurals = generated_plurals
        for f, pl in fem_pairs.items():
            if f not in self.flagged:
                self.flagged[f] = set(pl)
                self.source[f] = "fem"
        for s in self.flagged:
            self.source.setdefault(s, "flag")

        # Entradas avulsas: radicais sem flag de plural e sem flag de verbo.
        self.standalone = {
            s
            for s, fl in self.entry_flags.items()
            if s[:1].islower() and not fl & PLURAL_FLAGS and s not in CLOSED_CLASS
        }
        all_singulars = {
            s for s in self.entry_flags if s[:1].islower() and "-" not in s and "." not in s and len(s) > 1
        }
        self.accepted: dict[str, set[str]] = {s: set(p) for s, p in self.flagged.items() if "-" not in s}
        self.alt: dict[str, set[str]] = {}
        self.invariable: set[str] = set()
        for s in all_singulars:
            cands = candidates(s) & self.standalone
            if s in self.accepted:
                extra = cands - self.accepted[s]
                if extra:
                    self.alt[s] = extra
                continue
            if cands:
                self.accepted[s] = cands
                self.source[s] = "gap"
        for w, (pl, _why) in GRAMMAR_OVERRIDES.items():
            if w in self.accepted:
                self.accepted[w] = self.accepted[w] | {pl}
        # Invariáveis: -s/-x paroxítonas sem plural conhecido e que não são plural de nada.
        known_plurals = generated_plurals.union(*self.accepted.values())
        for s in all_singulars:
            if s in self.accepted or s in known_plurals or s in CLOSED_CLASS:
                continue
            if s.endswith(("s", "x")) and not is_oxytone(s):
                self.accepted[s] = {s}
                self.invariable.add(s)
                self.source[s] = "inv"

        # Compostos.
        compounds = {s for s in self.entry_flags if s[:1].islower() and "-" in s and "." not in s}
        self.compound_accepted: dict[str, set[str]] = {}
        self.compound_unknown = 0
        plural_of = self.accepted
        for c in compounds:
            if c in generated_plurals:
                continue
            parts = c.split("-")
            options = [[p] + sorted(plural_of.get(p, ())) for p in parts]
            found = set(self.flagged.get(c, ()))
            n = 1
            for o in options:
                n *= len(o)
            if n <= 4096:
                for combo in itertools.product(*options):
                    cand = "-".join(combo)
                    if cand != c and cand in compounds:
                        found.add(cand)
            # um singular que é plural de outro composto não entra
            if found:
                self.compound_accepted[c] = found
            else:
                self.compound_unknown += 1
        # tira compostos que são o plural achado de outro
        plurals_found = set().union(*self.compound_accepted.values()) if self.compound_accepted else set()
        for c in list(self.compound_accepted):
            if c in plurals_found:
                del self.compound_accepted[c]


def build_harness():
    os.makedirs(TARGET, exist_ok=True)
    if (
        not os.path.exists(HARNESS_BIN)
        or os.path.getmtime(HARNESS_BIN) < os.path.getmtime(PLURAL_RS)
        or os.path.getmtime(HARNESS_BIN) < os.path.getmtime(HARNESS_SRC)
    ):
        subprocess.run(
            ["rustc", "--edition", "2024", "-O", HARNESS_SRC, "-o", HARNESS_BIN],
            check=True,
        )


def run_lavra(words) -> dict[str, str]:
    build_harness()
    data = "\n".join(words) + "\n"
    res = subprocess.run([HARNESS_BIN], input=data, capture_output=True, text=True, check=True)
    out = {}
    for line in res.stdout.splitlines():
        w, p = line.split("\t")
        out[w] = p
    return out


def ending_class(w: str) -> str:
    for e in ("ão", "ês", "ás", "ís", "ós", "és", "us", "is", "es", "as", "os", "x", "al", "el",
              "il", "ol", "ul", "m", "n", "r", "z"):
        if w.endswith(e):
            return "-" + e
    return "-" + w[-1:]


PREPOSITIONS = {
    "a", "à", "ao", "aos", "às", "com", "da", "das", "de", "do", "dos", "e", "em", "na",
    "nas", "no", "nos", "ou", "para", "pela", "pelas", "pelo", "pelos", "por", "sem", "sob",
    "sobre",
}


def compound_mask(sg: str, pl: str) -> str:
    a, b = sg.split("-"), pl.split("-")
    if len(a) != len(b):
        return "?"
    return "".join(
        "P" if x != y else ("·" if x in PREPOSITIONS or x.startswith("d'") else "_") for x, y in zip(a, b)
    )


def report(ref: Reference, args):
    simple = ref.accepted
    lavra = run_lavra(sorted(simple) + sorted(ref.compound_accepted))
    groups: dict[str, list] = defaultdict(list)
    by_source = Counter()
    bad_by_source = Counter()
    for s, acc in simple.items():
        by_source[ref.source[s]] += 1
        p = lavra[s]
        if p not in acc:
            bad_by_source[ref.source[s]] += 1
            groups[ending_class(s)].append((s, p, acc))
    comp_bad = []
    for c, acc in ref.compound_accepted.items():
        if lavra[c] not in acc:
            comp_bad.append((c, lavra[c], acc))

    if args.dump:
        with open(args.dump, "w") as f:
            for g, items in sorted(groups.items()):
                for s, p, acc in sorted(items):
                    f.write(f"{g}\t{s}\t{p}\t{','.join(sorted(acc))}\t{ref.source[s]}\n")
            for s, p, acc in sorted(comp_bad):
                f.write(f"composto\t{s}\t{p}\t{','.join(sorted(acc))}\tcomp\n")
        print(f"divergências gravadas em {args.dump}")

    if args.show:
        key = args.show if args.show.startswith("-") or args.show == "composto" else "-" + args.show
        items = comp_bad if key == "composto" else groups.get(key, [])
        for s, p, acc in sorted(items):
            print(f"{s}\t{p}\t[{', '.join(sorted(acc))}]\t{ref.source.get(s, 'comp')}")
        return

    total = len(simple)
    bad = sum(len(v) for v in groups.values())
    print(f"palavras simples: {total}  divergências: {bad}")
    for src, label in (("flag", "plural por flag"), ("fem", "feminino"), ("gap", "entrada avulsa"),
                       ("inv", "invariável")):
        print(f"  {label:>16}: {by_source[src]:7d}  divergências: {bad_by_source[src]}")
    print(f"correções de flag do VERO (ver hunspell_fix): {sum(ref.fixes.values())}")
    for kind, n in ref.fixes.most_common():
        print(f"  {kind:>16}: {n:6d}  {'; '.join(ref.fix_examples[kind])}")
    print(f"compostos com plural conhecido: {len(ref.compound_accepted)}  divergências: {len(comp_bad)}"
          f"  (sem plural no dicionário: {ref.compound_unknown})")
    print()
    for g, items in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        ex = ", ".join(f"{s}>{p} [{'/'.join(sorted(acc))}]" for s, p, acc in sorted(items)[:4])
        print(f"{g:>6} {len(items):6d}  {ex}")
    if comp_bad:
        print()
        print("compostos: padrão esperado (P = parte no plural, · = preposição) e o da Lavra")
        cg: dict[tuple, list] = defaultdict(list)
        for c, p, acc in comp_bad:
            want = "/".join(sorted({compound_mask(c, a) for a in acc}))
            cg[(want, compound_mask(c, p))].append(c)
        for (want, got), items in sorted(cg.items(), key=lambda kv: -len(kv[1]))[:25]:
            print(f"  {want:>12} lavra {got:<8} {len(items):6d}  {', '.join(sorted(items)[:6])}")


def edge(ref: Reference):
    simple = ref.accepted
    lavra = run_lavra(sorted(simple))
    # 1. Mais de um plural aceito.
    multi = {s: acc for s, acc in simple.items() if len(acc) > 1}
    print(f"== singulares com mais de um plural aceito: {len(multi)}")
    pat = Counter()
    for s, acc in multi.items():
        pat[(ending_class(s), len(acc))] += 1
    for (g, n), c in pat.most_common(20):
        ex = [f"{s}:{'/'.join(sorted(a))}" for s, a in sorted(multi.items()) if ending_class(s) == g and len(a) == n][:3]
        print(f"  {g} x{n}: {c}  {'; '.join(ex)}")
    print(f"== singulares com variante avulsa no dicionário (não contada): {len(ref.alt)}")
    print("  ", "; ".join(f"{s}: {'/'.join(sorted(a))}" for s, a in sorted(ref.alt.items())[:30]))
    print("== aceitos contra o VERO (GRAMMAR_OVERRIDES):")
    for w, (pl, why) in GRAMMAR_OVERRIDES.items():
        print(f"  {w} → {pl}: {why}")

    # 2. Colisões: plural da Lavra igual a uma palavra que o dicionário tem como singular.
    coll = [(s, p) for s, p in lavra.items() if p != s and p in ref.accepted and p not in ref.invariable]
    print(f"== colisões (plural da Lavra = singular de outra palavra): {len(coll)}")
    print("  ", ", ".join(f"{s}>{p}" for s, p in sorted(coll)[:80]))

    # 3. Inverso.
    inverse(ref, lavra)


LAVRA_WORDS = [
    "as", "os", "às", "aos", "das", "dos", "nas", "nos", "pelas", "pelos", "mais", "menos",
    "vezes", "bits", "bytes", "ambos", "ambas", "antes", "depois", "uns", "umas", "dois",
    "duas", "três", "eis", "pois", "aspas", "dias",
]


def unstressed_variants(stem: str) -> set[str]:
    """O radical e as versões com acento na última vogal ("mes" → "mês")."""
    out = {stem}
    marks = {"a": "áâ", "e": "éê", "i": "í", "o": "óô", "u": "ú"}
    for i in range(len(stem) - 1, -1, -1):
        if stem[i] in marks:
            for m in marks[stem[i]]:
                out.add(stem[:i] + m + stem[i + 1:])
            break
    return out


def inverse_candidates(pl: str) -> set[str]:
    """Singulares possíveis de um plural, desfazendo cada regra (sem dicionário)."""
    out = {pl}
    if pl.endswith("s"):
        out.add(pl[:-1])
    if pl.endswith("es"):
        out |= unstressed_variants(pl[:-2])
    for a, b in (("ões", "ão"), ("ães", "ão"), ("ãos", "ão"), ("ns", "m"), ("ais", "al"),
                 ("éis", "el"), ("eis", "el"), ("eis", "il"), ("óis", "ol"), ("ois", "ol"),
                 ("uis", "ul"), ("is", "il")):
        if pl.endswith(a):
            out.add(pl[: -len(a)] + b)
    if pl.endswith("eis"):
        out |= {v + "il" for v in unstressed_variants(pl[:-3])}
        out |= {v + "el" for v in unstressed_variants(pl[:-3])}
    if pl.endswith("ens"):
        out |= {v + "en" for v in unstressed_variants(pl[:-3])}
    return out


def inverse(ref: Reference, lavra: dict[str, str]):
    """Mede se o singular sai do plural sem ambiguidade."""
    produced = set(lavra.values())
    plurals = sorted(produced | set(LAVRA_WORDS))
    cands = {p: inverse_candidates(p) for p in plurals}
    every = sorted({c for cs in cands.values() for c in cs})
    fwd = run_lavra(every)
    known = set(ref.accepted) | set(ref.entry_flags)
    amb_rules = 0
    amb_lexicon = []
    kinds = Counter()
    for p in plurals:
        if p not in produced:
            continue
        sg = {c for c in cands[p] if fwd.get(c) == p and not c.endswith("õe")}
        if len(sg) > 1:
            sg.discard(p)  # o próprio plural como invariável só vale se não houver outro
        if len(sg) > 1:
            amb_rules += 1
            key = "/".join(sorted(p[len(os.path.commonprefix(sorted(sg))):] and
                                  c[len(os.path.commonprefix(sorted(sg))):] or "∅" for c in sg))
            kinds[key] += 1
            in_dic = sg & known
            if len(in_dic) > 1:
                amb_lexicon.append((p, sorted(in_dic)))
    n = len(produced)
    print(f"== inverso só por regra: {amb_rules} de {n} plurais têm mais de um singular que volta a ele")
    for k, c in kinds.most_common(12):
        print(f"  singulares {k}: {c}")
    print(f"== inverso com o dicionário como léxico: {len(amb_lexicon)} plurais com mais de um singular no dicionário")
    print("  ", "; ".join(f"{p} < {'|'.join(ss)}" for p, ss in amb_lexicon[:40]))
    print("== palavras da Lavra que são plural de algum singular possível:")
    for w in LAVRA_WORDS:
        sg = sorted(c for c in cands[w] if fwd.get(c) == w and c != w)
        if sg:
            print(f"  {w}: {', '.join(sg)}{'  (no dicionário: ' + ', '.join(x for x in sg if x in known) + ')' if any(x in known for x in sg) else ''}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--show", help="lista as divergências de uma terminação (ex.: ão, -al, composto)")
    ap.add_argument("--dump", help="grava todas as divergências num TSV")
    ap.add_argument("--edge", action="store_true", help="relatório de casos de borda")
    args = ap.parse_args()
    ref = Reference()
    if args.edge:
        edge(ref)
    else:
        report(ref, args)


if __name__ == "__main__":
    sys.exit(main())
