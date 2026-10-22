"""Confere `src/morphology/verbs/first_person.rs` contra o Verbiste (via verbecc).

Uso (de qualquer diretório):

    tools/morphology/.venv/bin/python -I tools/morphology/check_first_person.py [opções]

Opções:
    --no-build       não recompila o harness
    --all            lista todas as divergências (por padrão, 8 exemplos por modelo)
    --edges          lista os casos de borda: defectivos, erros conhecidos da referência
                     e decisões provisórias
    --collisions     lista as colisões: formas iguais a palavras funcionais, a números
                     por extenso ou aos verbos-núcleo da Lavra, e formas compartilhadas
                     por mais de um infinitivo
    --verb V         mostra a referência e a saída da função para o verbo V
    --api-sample N   confere N verbos sorteados da leitura direta do XML contra a API
                     do verbecc (`CompleteConjugator`), para provar que as duas batem

A referência é lida direto dos XML do verbecc (`verbs-pt.xml` dá o modelo de cada
verbo, `conjugations-pt.xml` dá a terminação de cada pessoa no modelo), que é o que a
API faz por baixo, sem a predição por aprendizado de máquina para verbos ausentes.

O código de saída é 1 quando há divergência fora das exceções documentadas aqui.
"""

import collections
import os
import random
import subprocess
import sys
import sysconfig
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SRC = os.path.join(ROOT, "src/morphology/verbs/first_person.rs")
HARNESS = os.path.join(HERE, "harness/first_person.rs")
BINARY = os.path.join(HERE, "target/first_person_harness")
DATA = os.path.join(sysconfig.get_paths()["purelib"], "verbecc/data/xml")
VERBS_XML = os.path.join(DATA, "verbs/verbs-pt.xml")
CONJ_XML = os.path.join(DATA, "conjugations/conjugations-pt.xml")

# Modelos do Verbiste com a terminação da 1ª pessoa corrompida no XML. A chave é o
# modelo, o valor é a terminação certa (o que se cola ao radical do modelo).
TEMPLATE_FIXES: dict[str, tuple[str, str]] = {
    "perdo:ar": ("o", "vazio no XML; perdoar faz perdoo"),
    "abacalho:ar-se": ("o-me", "vazio no XML; perdoar-se faz perdoo-me"),
    "co:ar": ("o", "\"r)\" no XML; voar faz voo"),
    "co:ar-se": ("o-me", "\"r)\" no XML; doar-se faz doo-me"),
    "ato:ar": ("o", "\")\" no XML; ecoar faz ecoo"),
    "ato:ar-se": ("o-me", "\")\" no XML"),
    "cond:oer": ("oo", "vazio no XML; corroer faz corroo"),
    "cond:oer-se": ("oo-me", "vazio no XML"),
    "esm:oer": ("oo", "\")\" no XML; remoer faz remoo"),
    "rem:oer-se": ("oo-me", "\")\" no XML"),
    "d:oer-se": ("oo-me", "\"Br)\" no XML"),
    "m:oer-se": ("oo-me", "\"Br)\" no XML"),
    "m:oer": ("oo", "\"o\" no XML dá \"mo\"; moer faz moo"),
    "d:oer": ("oo", "\"o\" no XML dá \"do\"; doer faz doo"),
    # Grafia anterior ao Acordo de 1990, que tirou o acento do ditongo "oi" nas
    # paroxítonas (eu apoio, a jiboia).
    "ab:oiar": ("oio", "\"óio\" é grafia anterior a 1990"),
    "ap:oiar-se": ("oio-me", "\"óio\" é grafia anterior a 1990"),
    "b:oiar": ("oio", "\"óio\" é grafia anterior a 1990"),
    "comb:oiar": ("oio", "\"óio\" é grafia anterior a 1990"),
    "eng:oiar-se": ("oio-me", "\"óio\" é grafia anterior a 1990"),
    "jib:oiar": ("oio", "\"óio\" é grafia anterior a 1990"),
}

# Modelos de -ear que o Verbiste aplica também a verbos que não terminam em -ear
# (agenciar dá "agenciio", mancar dá "mancio"). Para esses verbos a referência passa a
# ser a forma regular (agencio, manco).
EAR_TEMPLATES = {"relampague:ar": "o", "abande:ar-se": "o-me"}

# Verbos em que o Verbiste erra (modelo errado para aquele verbo) e a função está
# certa. Infinitivo -> (forma certa, ou None para defectivo; motivo).
# A entrada vale também para o pronominal (`-se` vira `-me` na forma).
REFERENCE_ERRORS: dict[str, tuple[str | None, str]] = {
    # Modelo compr:ar (regular) aplicado a verbos em -ear.
    "arquear": ("arqueio", "verbo em -ear com modelo regular"),
    "balear": ("baleio", "verbo em -ear com modelo regular"),
    "mordomear": ("mordomeio", "verbo em -ear com modelo regular"),
    "passagear": ("passageio", "verbo em -ear com modelo regular"),
    "pestanear": ("pestaneio", "verbo em -ear com modelo regular"),
    "samear": ("sameio", "verbo em -ear com modelo regular"),
    # Hiato em -izar sem acento, ao contrário de enraizar/enraízo no mesmo XML.
    "desraizar": ("desraízo", "hiato a-i como em enraízo"),
    "emboizar": ("emboízo", "hiato o-i como em enraízo"),
    # "arreio" é de arrear (pôr arreios); arriar (baixar) é regular.
    "arriar": ("arrio", "confusão com arrear"),
    # Composto de tecer: c vira ç como em entretecer/entreteço.
    "intertecer": ("interteço", "composto de tecer sem a cedilha"),
    # cauda tem ditongo (cau-da), não hiato.
    "descaudar": ("descaudo", "ditongo au, sem hiato"),
    # Modelo de vir aplicado a devenir; o verbo se conjuga como prevenir.
    "devenir": ("devino", "modelo de vir dá \"devenenho\""),
    # Irregulares que o XML conjuga como regulares.
    "parir": ("pairo", "irregular como caber/caibo"),
    "mobiliar": ("mobílio", "tônica no i do radical, com acento"),
    "raer": ("raio", "conjuga-se como cair/caio"),
    "desmobiliar": ("desmobílio", "como mobiliar"),
    "malaparir": ("malapairo", "como parir"),
    # pupilar não tem 1ª pessoa no XML, mas pupilar-se tem.
    "pupilar-se": (None, "unipessoal como pupilar"),
}

# Palavras funcionais do português (artigos, preposições, contrações, conjunções,
# pronomes, advérbios frequentes).
FUNCTION_WORDS = """
o a os as um uma uns umas
de da do das dos em na no nas nos num numa por pelo pela pelos pelas para pra pro
com sem sob sobre entre até após ante desde contra perante trás via segundo conforme
ao aos à às dum duma deste desta neste nesta disto nisto daquele naquele
e ou mas nem que se porque pois porém contudo todavia logo portanto quando como
enquanto embora caso senão também ainda já só mesmo apenas salvo exceto menos
não sim mais muito pouco bem mal tão tanto quanto onde aonde assim então
eu tu ele ela nós vós eles elas me te lhe nos vos lhes si mim ti comigo
meu minha teu tua seu sua nosso nossa este esta esse essa aquele aquela isto isso aquilo
todo toda todos todas cada outro outra qual quais quem algo nada tudo alguém ninguém
vez vezes sempre nunca talvez antes depois aqui ali lá cá
""".split()

NUMBER_WORDS = """
zero um dois três quatro cinco seis sete oito nove dez onze doze treze catorze quatorze
quinze dezesseis dezessete dezoito dezenove vinte trinta quarenta cinquenta sessenta
setenta oitenta noventa cem cento duzentos trezentos quatrocentos quinhentos seiscentos
setecentos oitocentos novecentos mil milhão milhões bilhão meio meia dobro triplo
primeiro segundo terceiro quarto quinto sexto sétimo oitavo nono décimo último
""".split()

# Vocabulário da Lavra que uma 1ª pessoa pode imitar: verbos-núcleo (notes/language.md)
# e palavras reservadas.
LAVRA_CORE_VERBS = """
seja sejam guardo acrescento subtraio multiplico divido escrevo leio devolvo repito
paro pulo reservo libero desisto tomo
""".split()

LAVRA_RESERVED = """
deslocamento resultado fato item apontado verdadeiro falso nada registro
inteiro inteiros natural naturais real reais lógico texto caractere byte módulo
máximo mínimo resto soma produto valendo sendo devolvendo
""".split()


def load_templates():
    """Modelo -> lista de terminações da 1ª pessoa do presente do indicativo."""
    templates = {}
    for t in ET.parse(CONJ_XML).getroot():
        p = t.find("Indicativo/presente/p")
        templates[t.get("name")] = [i.text or "" for i in p.findall("i")]
    return templates


def load_reference():
    """Infinitivo -> (modelo, forma esperada ou None, forma crua do Verbiste)."""
    templates = load_templates()
    ref = {}
    for v in ET.parse(VERBS_XML).getroot():
        inf = v.find("i").text
        tname = v.find("t").text
        term = tname.split(":", 1)[1]
        if not inf.endswith(term):
            raise SystemExit(f"{inf}: não termina como o modelo {tname}")
        stem = inf[: len(inf) - len(term)]
        endings = templates[tname]
        raw = [stem + e if e != "-" else None for e in endings]
        if tname in TEMPLATE_FIXES:
            endings = [TEMPLATE_FIXES[tname][0]]
        if tname in EAR_TEMPLATES and not inf.removesuffix("-se").endswith("ear"):
            stem = inf.removesuffix("-se")[:-2]
            endings = [EAR_TEMPLATES[tname]]
        expected = [stem + e if e != "-" else None for e in endings]
        base = inf.removesuffix("-se")
        if inf in REFERENCE_ERRORS:
            expected = [REFERENCE_ERRORS[inf][0]]
        elif base in REFERENCE_ERRORS:
            form = REFERENCE_ERRORS[base][0]
            expected = [None if form is None else form + "-me"]
        ref[inf] = (tname, expected, raw)
    return ref


def build():
    os.makedirs(os.path.dirname(BINARY), exist_ok=True)
    if os.path.exists(BINARY):
        mtime = os.path.getmtime(BINARY)
        if mtime > os.path.getmtime(SRC) and mtime > os.path.getmtime(HARNESS):
            return
    subprocess.run(
        ["rustc", "--edition", "2024", "-O", "-o", BINARY, HARNESS], check=True
    )


def run(infinitives):
    r = subprocess.run(
        [BINARY],
        input="\n".join(infinitives) + "\n",
        capture_output=True,
        text=True,
        check=True,
    )
    out = {}
    for line in r.stdout.splitlines():
        inf, form = line.split("\t")
        out[inf] = None if form == "-" else form
    return out


def show(form):
    return "None" if form is None else form


def main(argv):
    if "--api-sample" in argv:
        return api_sample(int(argv[argv.index("--api-sample") + 1]))
    if "--no-build" not in argv:
        build()
    ref = load_reference()
    got = run(sorted(ref))

    if "--verb" in argv:
        v = argv[argv.index("--verb") + 1]
        print(v, ref.get(v), show(got.get(v)) if v in got else run([v]).get(v))
        return 0

    by_template = collections.defaultdict(list)
    for inf, (tname, expected, _) in sorted(ref.items()):
        if got[inf] not in expected:
            by_template[tname].append((inf, expected, got[inf]))

    total = sum(len(v) for v in by_template.values())
    show_all = "--all" in argv
    for tname, items in sorted(by_template.items(), key=lambda kv: -len(kv[1])):
        print(f"\n[{tname}] {len(items)} divergência(s)")
        for inf, expected, g in items if show_all else items[:8]:
            exp = "/".join(show(e) for e in expected)
            print(f"  {inf}: referência {exp}, função {show(g)}")

    pronominal = sum(1 for i in ref if i.endswith("-se"))
    defective = [i for i, (_, e, _) in ref.items() if e == [None]]
    fixed = [i for i, (t, _, _) in ref.items() if t in TEMPLATE_FIXES]
    print(f"\nverbos testados: {len(ref)} ({len(ref) - pronominal} sem pronome,"
          f" {pronominal} pronominais em -se)")
    print(f"divergências: {total} em {len(by_template)} modelo(s)")
    print(f"defectivos (sem 1ª pessoa, função deve devolver None): {len(defective)}")
    print(f"verbos em modelos corrompidos no XML (corrigidos aqui): {len(fixed)}")
    print(f"erros de verbo na referência (corrigidos aqui): {len(REFERENCE_ERRORS)}")

    if "--edges" in argv:
        edges(ref, got)
    if "--collisions" in argv:
        collisions(ref, got)
    return 1 if total else 0


def edges(ref, got):
    print("\n== modelos corrompidos no XML ==")
    for t, (e, why) in TEMPLATE_FIXES.items():
        vs = [i for i, (tn, _, _) in ref.items() if tn == t]
        print(f"  {t} -> -{e} ({why}): {len(vs)} verbo(s), ex.: {', '.join(vs[:6])}")
    print("\n== modelos de -ear aplicados a verbos sem -ear (referência vira regular) ==")
    misapplied = [i for i, (t, _, _) in ref.items()
                  if t in EAR_TEMPLATES and not i.removesuffix("-se").endswith("ear")]
    print(f"  {len(misapplied)} verbo(s): {', '.join(misapplied)}")
    print("\n== erros de verbo na referência ==")
    for inf, (form, why) in sorted(REFERENCE_ERRORS.items()):
        print(f"  {inf}: Verbiste {'/'.join(show(r) for r in ref[inf][2])},"
              f" aqui {show(form)} ({why})")
    print("\n== defectivos (sem 1ª pessoa) ==")
    by_t = collections.defaultdict(list)
    for inf, (t, e, _) in ref.items():
        if e == [None]:
            by_t[t].append(inf)
    for t, vs in sorted(by_t.items(), key=lambda kv: -len(kv[1])):
        print(f"  [{t}] {len(vs)}: {', '.join(vs)}")


def collisions(ref, got):
    forms = collections.defaultdict(set)
    for inf, g in got.items():
        if g:
            forms[g].add(inf)
    vocab = {}
    for kind, words in [
        ("núcleo da Lavra", LAVRA_CORE_VERBS),
        ("reservada da Lavra", LAVRA_RESERVED),
        ("número/ordinal", NUMBER_WORDS),
        ("funcional", FUNCTION_WORDS),
    ]:
        for w in words:
            vocab.setdefault(w, kind)
    print("\n== 1ª pessoa igual a palavra funcional, número ou vocabulário da Lavra ==")
    for w, kind in sorted(vocab.items(), key=lambda kv: (kv[1], kv[0])):
        if w in forms:
            print(f"  {w} [{kind}]: {', '.join(sorted(forms[w]))}")

    # Infinitivos com e sem "-se" do mesmo verbo não contam como colisão entre si.
    shared = {}
    for f, infs in forms.items():
        bases = {i.removesuffix("-se") for i in infs}
        if len(bases) > 1:
            shared[f] = sorted(infs)
    print(f"\n== formas compartilhadas por mais de um infinitivo: {len(shared)} ==")
    for f, infs in sorted(shared.items()):
        print(f"  {f}: {', '.join(infs)}")


def api_sample(n):
    """Prova que ler o XML direto dá o mesmo que a API do verbecc."""
    from verbecc import CompleteConjugator, LangCodeISO639_1

    import logging

    logging.disable(logging.CRITICAL)
    conj = CompleteConjugator(LangCodeISO639_1("pt"))
    templates = load_templates()
    verbs = [(v.find("i").text, v.find("t").text)
             for v in ET.parse(VERBS_XML).getroot()]
    random.seed(1)
    bad = 0
    for inf, tname in random.sample(verbs, n):
        term = tname.split(":", 1)[1]
        stem = inf[: len(inf) - len(term)]
        xml = [stem + e if e != "-" else "-" for e in templates[tname]]
        tc = conj.conjugate_mood_tense(inf, "indicativo", "presente",
                                       conjugate_pronouns=False)
        api = list(tc[0])
        if api != xml:
            bad += 1
            print(f"  {inf} [{tname}]: XML {xml}, API {api}")
    print(f"amostra de {n}: {bad} diferença(s) entre XML e API")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
