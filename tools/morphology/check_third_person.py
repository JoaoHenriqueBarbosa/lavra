"""Confere `src/morphology/verbs/third_person.rs` contra o Verbiste (via verbecc).

Uso (de qualquer diretório):

    tools/morphology/.venv/bin/python -I tools/morphology/check_third_person.py [opções]

Opções:
    --no-build     não recompila o harness
    --all          lista todas as divergências (por padrão, 8 exemplos por modelo)
    --collisions   lista colisões (formas iguais a palavras funcionais, ao vocabulário
                   da Lavra, e formas compartilhadas por mais de um verbo)
    --inverse      estatística da inversão (forma da 3ª pessoa para infinitivo)
    --verb V       mostra a referência e a saída da função para o verbo V

O código de saída é 1 quando há divergência fora da lista de erros conhecidos da
referência.
"""

import collections
import os
import re
import subprocess
import sys
import sysconfig
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SRC = os.path.join(ROOT, "src/morphology/verbs/third_person.rs")
HARNESS = os.path.join(HERE, "harness/third_person.rs")
BINARY = os.path.join(HERE, "target/third_person_harness")
DATA = os.path.join(sysconfig.get_paths()["purelib"], "verbecc/data/xml")
VERBS_XML = os.path.join(DATA, "verbs/verbs-pt.xml")
CONJ_XML = os.path.join(DATA, "conjugations/conjugations-pt.xml")

# Onde o Verbiste erra e a função está certa: infinitivo -> (singular, plural, motivo).
# `None` no lugar das formas significa "o verbo não tem 3ª pessoa do presente". A
# conferência aceita a forma daqui no lugar da referência.
_CIAR = "modelo de -ear aplicado a -iar regular, gera \"-iia\" (pt-PT admite também \"-eia\")"
_EAR = "verbo em -ear conjugado como regular; todo -ear faz \"-eia\""
REFERENCE_ERRORS: dict[str, tuple[str | None, str | None, str]] = {
    **{v: (v[:-2] + "a", v[:-2] + "am", _CIAR) for v in """agenciar cadenciar comerciar
        desnegociar despremiar diligenciar esborniar licenciar monarquiar negociar
        obsequiar premiar presenciar sentenciar silenciar subsecretariar""".split()},
    **{v: (v[:-2] + "ia", v[:-2] + "iam", _EAR) for v in """arquear balear mordomear
        passagear pestanear samear""".split()},
    "mancar": ("manca", "mancam", "\"mancia\" não existe; verbo regular"),
    "pasquinar": ("pasquina", "pasquinam", "\"pasquinia\" não existe; verbo regular"),
    "pleitar": ("pleita", "pleitam", "\"pleitia\" não existe; verbo regular"),
    "brotar": ("brota", "brotam", "\"brotia\" não existe; verbo regular, não impessoal"),
    "arriar": ("arria", "arriam", "arriar (baixar) é regular; \"arreia\" é de arrear"),
    "desraizar": ("desraíza", "desraízam", "hiato como em enraizar/enraíza"),
    "entrequerer": ("entrequer", "entrequerem", "derivado de querer, como requerer/requer"),
    "devenir": ("devene", "devenem", "modelo de \"avir\" aplicado por engano (\"deveném\")"),
    "remir": (None, None, "defectivo; o Verbiste preenche com formas de redimir"),
    "sobresser": (None, None, "\"sobresé/sobressão\" perde o prefixo; verbo duvidoso"),
}

# Formas abundantes: a função escolhe uma; a referência traz a outra.
ALTERNATIVES: dict[str, tuple[str, str, str]] = {
    v: (v[:-4] + "rói", v[:-4] + "roem", "abundante: Verbiste dá \"-strui/-struem\"")
    for v in "construir desconstruir reconstruir destruir".split()
}

# Palavras funcionais do português (artigos, preposições, contrações, conjunções,
# pronomes, advérbios frequentes) e o vocabulário da Lavra.
FUNCTION_WORDS = """
o a os as um uma uns umas
de da do das dos em na no nas nos num numa por pelo pela pelos pelas para pra pro
com sem sob sobre entre até após ante desde contra perante trás via
ao aos à às dum duma deste desta neste nesta disto nisto daquele naquele
e ou mas nem que se porque pois porém contudo todavia logo portanto quando como
enquanto embora caso conforme senão também ainda já só mesmo apenas
não sim mais menos muito pouco bem mal tão tanto quanto onde aonde assim então
eu tu ele ela nós vós eles elas me te lhe nos vos lhes se si mim ti comigo
meu minha teu tua seu sua nosso nossa este esta esse essa aquele aquela isto isso aquilo
todo toda todos todas cada outro outra qual quais quem algo nada tudo alguém ninguém
vez vezes seguido seguida dividido sempre nunca talvez antes depois aqui ali lá cá
é são está estão foi era será há
""".split()

LAVRA_WORDS = """
inteiro inteiros natural naturais real reais lógico lógicos texto textos caractere
caracteres byte bytes bits verdadeiro falso resultado módulo máximo mínimo teto piso
saldo linha início fixo fato falha esquerda direita igual diferente maior menor
ambos ambas confiado emprestado salvo sendo devolvendo uma lista conta nota notas
""".split()


# Formas que o Verbiste grafa como antes do Acordo Ortográfico de 1990. A conferência
# compara com a grafia atual; cada troca entra no relatório (`--all`).
MODERNIZED: list[tuple[str, str, str]] = []


def modernize(form: str) -> str:
    new = form.replace("êem", "eem")  # leem, veem, creem, proveem (Base IX, 8)
    new = re.sub(r"([óé])i(am?)$", lambda m: {"ó": "o", "é": "e"}[m[1]] + "i" + m[2], new)  # apoia, boia (Base IX, 3)
    new = new.replace("ü", "u")  # trema (Base XIV)
    new = re.sub(r"([gq])ú([ie])", r"\1u\2", new)  # argui, arguem (Base X, 7)
    new = re.sub(r"([aeo][iu])([íú])(?=.)", lambda m: m[1] + {"í": "i", "ú": "u"}[m[2]], new)  # baiuca (Base IX, 4)
    return new


def load_reference():
    """Infinitivo (sem "-se") -> (modelo, formas do singular, formas do plural)."""
    templates = {}
    for t in ET.parse(CONJ_XML).getroot():
        pres = t.find("Indicativo/presente")
        ps = pres.findall("p")

        def forms(p):
            return [i.text or "" for i in p.findall("i")]

        templates[t.get("name")] = (forms(ps[2]), forms(ps[5]))

    ref = {}
    for v in ET.parse(VERBS_XML).getroot():
        inf = v.find("i").text
        tname = v.find("t").text
        term = tname.split(":", 1)[1]
        stem = inf[: len(inf) - len(term)]
        sing, plur = templates[tname]

        stem = modernize(stem)

        def build(endings, reflexive):
            out = []
            for e in endings:
                if reflexive:
                    e = e.removesuffix("-se")
                if not e or e.endswith("-"):
                    continue  # o Verbiste marca forma inexistente com "-"
                form = stem + e
                new = modernize(form)
                if new != form:
                    MODERNIZED.append((inf, form, new))
                out.append(new)
            return out

        reflexive = inf.endswith("-se")
        base = modernize(inf.removesuffix("-se"))  # "tranqüilizar" -> "tranquilizar"
        if base in ref and reflexive:
            continue  # a entrada sem "-se" prevalece
        s, p = build(sing, reflexive), build(plur, reflexive)
        if base in ref:
            continue
        ref[base] = (tname, s, p)
    return ref


def build():
    os.makedirs(os.path.dirname(BINARY), exist_ok=True)
    subprocess.run(
        ["rustc", "--edition", "2024", "-O", "-o", BINARY, HARNESS], check=True
    )


def run(infinitives):
    r = subprocess.run(
        [BINARY], input="\n".join(infinitives) + "\n", capture_output=True,
        text=True, check=True,
    )
    out = {}
    for line in r.stdout.splitlines():
        inf, s, p = line.split("\t")
        out[inf] = (s, p) if s else None
    return out


def main(argv):
    if "--no-build" not in argv:
        build()
    ref = load_reference()
    got = run(sorted(ref))

    if "--verb" in argv:
        v = argv[argv.index("--verb") + 1]
        print(v, ref.get(v), got.get(v))
        return 0

    by_template = collections.defaultdict(list)
    defective = []
    known = []
    alternatives = []
    for inf, (tname, s, p) in sorted(ref.items()):
        g = got[inf]
        if inf in REFERENCE_ERRORS:
            fs, fp, why = REFERENCE_ERRORS[inf]
            known.append((inf, s, p, fs, fp, why))
            expected = (fs, fp) if fs else None
            if g != expected:
                by_template[tname].append((inf, s, p, g))
            continue
        if inf in ALTERNATIVES:
            fs, fp, why = ALTERNATIVES[inf]
            alternatives.append((inf, s, p, fs, fp, why))
            if g != (fs, fp):
                by_template[tname].append((inf, s, p, g))
            continue
        if not s or not p:
            # Defectivo na referência: confere só a parte que existe.
            defective.append((inf, tname, s, p, g))
            if s and (g is None or g[0] not in s) or p and (g is None or g[1] not in p):
                by_template[tname].append((inf, s, p, g))
            continue
        if g is None or g[0] not in s or g[1] not in p:
            by_template[tname].append((inf, s, p, g))

    total = sum(len(v) for v in by_template.values())
    show_all = "--all" in argv
    for tname, items in sorted(by_template.items(), key=lambda kv: -len(kv[1])):
        print(f"\n[{tname}] {len(items)} divergência(s)")
        for inf, s, p, g in items if show_all else items[:8]:
            print(f"  {inf}: referência {'/'.join(s)} {'/'.join(p)}, função {g}")

    print(f"\nverbos testados: {len(ref)}")
    print(f"divergências: {total} em {len(by_template)} modelo(s)")
    print(f"defectivos na referência (sem 3ª pessoa): {len(defective)}")
    print(f"erros conhecidos da referência: {len(known)}")
    print(f"abundantes (a função escolhe a outra forma): {len(alternatives)}")
    print(f"formas da referência trazidas para a grafia do Acordo: {len(MODERNIZED)}")
    if show_all:
        for d in defective:
            print("  defectivo:", d)
        for k in known:
            print("  erro da referência:", k)
        for a in alternatives:
            print("  abundante:", a)
        for m in MODERNIZED:
            print("  grafia do Acordo:", m)

    if "--collisions" in argv:
        collisions(ref, got)
    if "--inverse" in argv:
        inverse(ref, got)
    return 1 if total else 0


DETERMINERS = r"o|a|os|as|um|uma|uns|umas|do|da|dos|das|no|na|nos|nas|ao|à|aos|às|pelo|pela|cada"


def lavra_nouns():
    """Palavras depois de artigo ou contração nos programas .lav do repositório."""
    words = set()
    for base, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in ("target", ".git", ".venv")]
        for f in files:
            if f.endswith(".lav"):
                text = open(os.path.join(base, f), encoding="utf-8").read().lower()
                for m in re.finditer(rf"\b(?:{DETERMINERS})\s+([a-zà-úç]+)", text):
                    words.add(m[1])
    return words


def collisions(ref, got):
    forms = collections.defaultdict(set)
    for inf, g in got.items():
        if g:
            forms[g[0]].add((inf, "sg"))
            forms[g[1]].add((inf, "pl"))
    vocab = {w: "funcional" for w in FUNCTION_WORDS}
    vocab.update({w: "Lavra" for w in LAVRA_WORDS if w not in vocab})
    vocab.update({w: "substantivo nos .lav" for w in lavra_nouns() if w not in vocab})
    print("\n== colisões com palavras funcionais e vocabulário da Lavra ==")
    for w, kind in sorted(vocab.items()):
        if w in forms:
            vs = ", ".join(f"{i} ({n})" for i, n in sorted(forms[w]))
            print(f"  {w} [{kind}]: {vs}")
    shared = {f: v for f, v in forms.items() if len({i for i, _ in v}) > 1}
    print(f"\n== formas compartilhadas por mais de um verbo: {len(shared)} ==")
    for f, v in sorted(shared.items()):
        print(f"  {f}: " + ", ".join(f"{i} ({n})" for i, n in sorted(v)))


def inverse(ref, got):
    sg = collections.defaultdict(set)
    pl = collections.defaultdict(set)
    for inf, g in got.items():
        if g:
            sg[g[0]].add(inf)
            pl[g[1]].add(inf)
    amb_s = {f: v for f, v in sg.items() if len(v) > 1}
    amb_p = {f: v for f, v in pl.items() if len(v) > 1}
    cross = set(sg) & set(pl)
    print(f"\n== inversão ==")
    print(f"formas do singular: {len(sg)}, ambíguas: {len(amb_s)}")
    print(f"formas do plural: {len(pl)}, ambíguas: {len(amb_p)}")
    print(f"formas que são singular de um verbo e plural de outro: {len(cross)}")
    for f, v in sorted(amb_s.items()):
        print(f"  sg {f}: {', '.join(sorted(v))}")
    for f, v in sorted(amb_p.items()):
        print(f"  pl {f}: {', '.join(sorted(v))}")
    for f in sorted(cross):
        print(f"  sg/pl {f}: sg de {', '.join(sorted(sg[f]))}; pl de {', '.join(sorted(pl[f]))}")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
