"""Gera `lavra.tmLanguage.json`, a gramática TextMate da Lavra.

As palavras vêm do léxico que o compilador exporta (`editors/lexicon.json`, gerado por
`lavrac lexicon --json`); aqui ficam só as regras que as combinam. O JSON da gramática é
gerado (não edite à mão). Rodar: `python3 -I syntaxes/build.py` dentro de `editors/vscode`.

A gramática é léxica: pinta o que a forma das palavras revela. O que depende de nomes
declarados (verbo do usuário num predicado, caso de enumeração) fica para os tokens
semânticos do servidor de linguagem.
"""

import json
from pathlib import Path

# Fronteiras de palavra com letras acentuadas (\b do Oniguruma não basta para "é").
B0 = r"(?<![\p{L}\p{N}_])"
B1 = r"(?![\p{L}\p{N}_])"
WORD = r"[\p{L}_][\p{L}\p{N}_]*(?:-[\p{L}][\p{L}\p{N}_]*)*"
SYMBOL = r"(?:[b-df-np-zB-DF-NP-Z]\d*|[aeoAEO]\d+)"


def words(*ws: str) -> str:
    """Alternativa de palavras inteiras, sem diferenciar maiúsculas."""
    return B0 + "(?i:" + "|".join(sorted(ws, key=len, reverse=True)) + ")" + B1


def phrase(*parts: str) -> str:
    """Locução com espaços flexíveis entre as palavras."""
    return r"\s+".join(parts)


LEXICON = json.loads((Path(__file__).resolve().parents[2] / "lexicon.json").read_text(encoding="utf-8"))

ARTICLES = [a["form"] for a in LEXICON["articles"] if a["definite"]]
INDEFINITE = [a["form"] for a in LEXICON["articles"] if not a["definite"]]
CONTRACTIONS = [c["form"] for c in LEXICON["contractions"]]
PREPOSITIONS = LEXICON["prepositions"]
TYPES = [form for t in LEXICON["types"] for form in (t["singular"], t["plural"])]
NOMINALS = [n["word"] for n in LEXICON["nominals"]]
ORDINAL_STEMS = [o["stem"] for o in LEXICON["ordinals"]]
MUTATIONS = [form for v in LEXICON["subjectVerbs"] if v["role"] == "mutation" for form in (v["singular"], v["plural"])]
CORE_VERBS = [v["firstPerson"] for v in LEXICON["coreVerbs"] if v["role"] == "statement"]
FLOW_VERBS = [v["firstPerson"] for v in LEXICON["coreVerbs"] if v["role"] == "flow"]
COMPARISONS = [tuple(k["form"].split(" ")) for k in LEXICON["keywords"] if k["role"] == "comparison"]
LITERALS = [k["form"] for k in LEXICON["keywords"] if k["role"] == "literal"]
ART = "(?i:" + "|".join(ARTICLES) + ")"
# Palavras que podem seguir "for" sem ser adjetivo de predicado.
ADJ_STOP = ARTICLES + INDEFINITE + CONTRACTIONS + [a for a, _ in COMPARISONS] + LITERALS + ["não"]
# Palavras que abrem linha sem ser verbo do usuário: toda palavra da língua (cada
# palavra das locuções), menos as formas recusadas com dica, que o usuário pode definir.
NOT_VERBS = sorted({part for w in LEXICON["words"] if w["role"] != "rejected" for part in w["form"].split(" ")})
CONTR = "(?i:" + "|".join(CONTRACTIONS) + ")"

name_after_article = {
    "match": B0 + "(" + ART + r")\s+(" + WORD + ")" + B1,
    "captures": {"1": {"name": "variable.other.article.lavra"}, "2": {"name": "variable.other.lavra"}},
}
name_after_contraction = {
    "match": B0 + "(" + CONTR + r")\s+(" + WORD + ")" + B1,
    "captures": {"1": {"name": "keyword.operator.word.preposition.lavra"}, "2": {"name": "variable.other.lavra"}},
}

repository = {
    "comments": {
        "patterns": [
            {"name": "comment.line.nota.lavra", "match": r"^\s*(?i:nota):.*$"},
            {"name": "comment.line.double-dash.lavra", "match": r"--.*$"},
        ]
    },
    "strings": {
        "patterns": [
            {
                "name": "string.quoted.double.lavra",
                "begin": "[\"“”]",
                "end": "[\"“”]",
                "beginCaptures": {"0": {"name": "punctuation.definition.string.begin.lavra"}},
                "endCaptures": {"0": {"name": "punctuation.definition.string.end.lavra"}},
                "patterns": [{"name": "constant.character.escape.lavra", "match": r"\\[nt0\\\"']"}],
            },
            {
                "name": "constant.character.lavra",
                "match": r"['‘’](?:\\[nt0\\\"']|[^'‘’\\])['‘’]",
            },
        ]
    },
    "numbers": {
        "patterns": [
            {"name": "constant.numeric.hex.lavra", "match": B0 + r"0[xX][0-9A-Fa-f]+" + B1},
            # Milhar com espaço ("1 000 000") e vírgula decimal ("8,5").
            {
                "name": "constant.numeric.decimal.lavra",
                "match": r"(?<![\p{L}\p{N}_,])-?\d{1,3}(?: \d{3})+(?:,\d+)?(?![\p{L}\p{N}_])",
            },
            {"name": "constant.numeric.decimal.lavra", "match": r"(?<![\p{L}\p{N}_])-?\d+(?:,\d+)?(?![\p{L}\p{N}_])"},
        ]
    },
    "definitions": {
        "patterns": [
            # "Para começar:" é a entrada do programa.
            {
                "match": r"^\s*(?i:(para))\s+(começar)\s*(:)",
                "captures": {
                    "1": {"name": "keyword.other.definition.lavra"},
                    "2": {"name": "entity.name.function.main.lavra"},
                    "3": {"name": "punctuation.section.block.lavra"},
                },
            },
            # "Uma conta é um registro de:", "Uma forma é uma destas:".
            {
                "match": r"^\s*(?i:(um|uma))\s+(" + WORD + r")\s+(é)\s+((?:um\s+registro\s+de)|(?:uma\s+destas)|(?:um\s+destes))\s*(:)",
                "captures": {
                    "1": {"name": "keyword.other.article.lavra"},
                    "2": {"name": "entity.name.type.lavra"},
                    "3": {"name": "storage.type.declaration.lavra"},
                    "4": {"name": "keyword.other.definition.lavra"},
                    "5": {"name": "punctuation.section.block.lavra"},
                },
            },
            {"include": "#header"},
        ]
    },
    # Cabeçalho de função: "Para transferir a quantia da origem para o destino, sendo ...:".
    # Pode ocupar várias linhas; termina nos dois-pontos do fim da linha.
    "header": {
        "begin": r"^\s*(?i:(para))(?!\s+(?i:cada|começar)" + B1 + r")\s+(?:(" + ART + r")\s+(" + WORD + r")\s+|(" + SYMBOL + r")\s+)?"
        + r"(?:(ser)\s+(" + WORD + r")|(" + WORD + r"))(?:\s+(\()(" + WORD + r")(\)))?",
        "beginCaptures": {
            "1": {"name": "keyword.other.definition.lavra"},
            "2": {"name": "variable.other.article.lavra"},
            "3": {"name": "variable.parameter.lavra"},
            "4": {"name": "variable.parameter.lavra"},
            "5": {"name": "entity.name.function.lavra"},
            "6": {"name": "entity.name.function.adjective.lavra"},
            "7": {"name": "entity.name.function.lavra"},
            "8": {"name": "punctuation.definition.parameters.begin.lavra"},
            "9": {"name": "entity.name.function.lavra"},
            "10": {"name": "punctuation.definition.parameters.end.lavra"},
        },
        "end": r":\s*$",
        "endCaptures": {"0": {"name": "punctuation.section.block.lavra"}},
        "name": "meta.function.header.lavra",
        "patterns": [
            {"include": "#comments"},
            {"name": "keyword.other.signature.lavra", "match": words("sendo", "devolvendo")},
            {
                "match": phrase("(?i:salvo)", "(uma)", "(falha)"),
                "name": "keyword.other.fallible.lavra",
            },
            {"name": "keyword.other.both.lavra", "match": words("ambos", "ambas")},
            {"name": "keyword.operator.word.conjunction.lavra", "match": words("e")},
            {"include": "#ownership"},
            {"include": "#types"},
            # Predicado adjetivo: "Para o número ser par".
            {
                "match": B0 + r"(ser)\s+(" + WORD + ")" + B1,
                "captures": {"1": {"name": "entity.name.function.lavra"}, "2": {"name": "entity.name.function.adjective.lavra"}},
            },
            {
                "match": B0 + "(" + ART + r")\s+(" + WORD + ")" + B1,
                "captures": {"1": {"name": "variable.other.article.lavra"}, "2": {"name": "variable.parameter.lavra"}},
            },
            {
                "match": B0 + "(" + CONTR + r")\s+(" + WORD + ")" + B1,
                "captures": {"1": {"name": "keyword.operator.word.preposition.lavra"}, "2": {"name": "variable.parameter.lavra"}},
            },
            {"name": "keyword.operator.word.preposition.lavra", "match": words(*PREPOSITIONS)},
            {"name": "keyword.other.article.lavra", "match": words(*INDEFINITE)},
            {"name": "variable.parameter.lavra", "match": B0 + SYMBOL + B1},
            {"name": "entity.name.type.lavra", "match": B0 + WORD + B1},
            {"include": "#punctuation"},
        ],
    },
    "control": {
        "patterns": [
            {
                "match": r"^\s*(?i:(para))\s+(cada)\s+(?:(" + SYMBOL + ")|(" + WORD + "))" + B1,
                "captures": {
                    "1": {"name": "keyword.control.loop.lavra"},
                    "2": {"name": "keyword.control.loop.lavra"},
                    "3": {"name": "variable.other.symbol.lavra"},
                    "4": {"name": "variable.other.lavra"},
                },
            },
            {"match": r"^\s*(" + words("se", "senão", "conforme", "sendo") + ")", "captures": {"1": {"name": "keyword.control.conditional.lavra"}}},
            {"name": "keyword.control.conditional.lavra", "match": r"(?<=:\s|,\s)" + words("se", "senão")},
            {"match": r"^\s*(" + words("enquanto", "repito") + ")", "captures": {"1": {"name": "keyword.control.loop.lavra"}}},
            {"name": "keyword.control.loop.lavra", "match": words("sempre", "até")},
            {"name": "keyword.control.flow.lavra", "match": words(*FLOW_VERBS)},
            {"name": "keyword.control.flow.lavra", "match": phrase(",", "(?i:ou)", "(?i:desisto)") + B1},
            # Depois de "for", palavra que não é comparação, artigo nem literal é o
            # adjetivo de um predicado ("Se o senado for paciente").
            {
                "match": B0 + r"(?i:(for|forem))\s+(?!(?i:" + "|".join(ADJ_STOP) + ")" + B1 + r")(" + WORD + ")" + B1,
                "captures": {"1": {"name": "keyword.operator.copula.lavra"}, "2": {"name": "entity.name.function.adjective.lavra"}},
            },
            {"name": "keyword.operator.copula.lavra", "match": words("for", "forem", "houver")},
        ]
    },
    "declarations": {
        "patterns": [
            # "começa em", "começam com", "começa vazio": declaração de quem muda.
            {
                "match": B0 + r"(?i:(começa|começam))(?:\s+(em|com|na|no|nas|nos)|\s+(vazi[oa]s?))?" + B1,
                "captures": {
                    "1": {"name": "storage.modifier.mutable.lavra"},
                    "2": {"name": "keyword.operator.word.preposition.lavra"},
                    "3": {"name": "constant.language.empty.lavra"},
                },
            },
            {"name": "storage.type.declaration.lavra", "match": words("é", "são")},
            {"name": "constant.language.empty.lavra", "match": words("vazio", "vazia", "vazios", "vazias")},
            {"name": "keyword.other.mutation.lavra", "match": words(*MUTATIONS)},
        ]
    },
    "ownership": {
        "patterns": [
            {"name": "storage.modifier.borrowed.lavra", "match": words("emprestado", "emprestada", "emprestados", "emprestadas")},
            {"name": "storage.modifier.lent.lavra", "match": words("confiado", "confiada", "confiados", "confiadas")},
            {"name": "storage.modifier.maybe.lavra", "match": words("talvez")},
            {
                "match": B0 + r"(?i:(uma|numa))\s+(cópia)" + B1,
                "captures": {"1": {"name": "keyword.other.article.lavra"}, "2": {"name": "support.function.copy.lavra"}},
            },
        ]
    },
    "types": {
        "patterns": [
            {
                "match": r"(?:" + B0 + r"(?i:(um|uma))\s+)?(" + words(*TYPES) + r")(?:\s+(de)\s+(\d+)\s+(bits))?",
                "captures": {
                    "1": {"name": "keyword.other.article.lavra"},
                    "2": {"name": "storage.type.primitive.lavra"},
                    "3": {"name": "keyword.operator.word.preposition.lavra"},
                    "4": {"name": "constant.numeric.decimal.lavra"},
                    "5": {"name": "storage.type.primitive.lavra"},
                },
            },
        ]
    },
    "builtins": {
        "patterns": [
            {"name": "support.function.builtin.lavra", "match": words(*CORE_VERBS)},
            {"name": "keyword.other.newline.lavra", "match": phrase("(?i:sem)", "quebrar", "a", "linha") + B1},
            # O artigo acompanha a operação ("o tamanho"); a contração é preposição
            # ("seguido do tamanho").
            {
                "match": B0 + "(?:(" + ART + ")|(" + CONTR + r"))\s+(" + "|".join(NOMINALS) + r")(?=\s+(?:de|do|da|dos|das|entre)" + B1 + ")",
                "captures": {
                    "1": {"name": "support.function.nominal.lavra"},
                    "2": {"name": "keyword.operator.word.preposition.lavra"},
                    "3": {"name": "support.function.nominal.lavra"},
                },
            },
            # "o resultado de flexionar ...": depois de "o resultado de" só vem verbo.
            {
                "match": B0 + "(?:(" + ART + ")|(" + CONTR + r"))\s+(resultado)(?:\s+(de)\s+(" + WORD + "))?" + B1,
                "captures": {
                    "1": {"name": "variable.language.article.lavra"},
                    "2": {"name": "keyword.operator.word.preposition.lavra"},
                    "3": {"name": "variable.language.result.lavra"},
                    "4": {"name": "keyword.operator.word.preposition.lavra"},
                    "5": {"name": "entity.name.function.call.lavra"},
                },
            },
            {
                "match": B0 + "(?:(" + ART + ")|(" + CONTR + r"))\s+(fato)\s+(de)" + B1,
                "captures": {
                    "1": {"name": "keyword.other.fact.lavra"},
                    "2": {"name": "keyword.operator.word.preposition.lavra"},
                    "3": {"name": "keyword.other.fact.lavra"},
                    "4": {"name": "keyword.other.fact.lavra"},
                },
            },
            {
                "match": B0 + "(?:(" + ART + ")|(" + CONTR + r"))\s+((?:" + "|".join(ORDINAL_STEMS) + r")(?:o|a|os|as)|item)" + B1,
                "captures": {
                    "1": {"name": "keyword.other.position.lavra"},
                    "2": {"name": "keyword.operator.word.preposition.lavra"},
                    "3": {"name": "keyword.other.position.lavra"},
                },
            },
        ]
    },
    "operators": {
        "patterns": [
            {"name": "keyword.operator.comparison.lavra", "match": "|".join(B0 + phrase(f"(?i:{a})", b) + B1 for a, b in COMPARISONS)},
            # Com a preposição contraída ("igual ao teto"), o operador é só a primeira
            # palavra; "ao teto" fica com a regra do nome.
            {"name": "keyword.operator.comparison.lavra", "match": B0 + r"(?i:igual|iguais)(?=\s+" + CONTR + B1 + ")"},
            {"name": "keyword.operator.comparison.lavra", "match": B0 + r"(?i:diferentes?)(?=\s+" + CONTR + B1 + ")"},
            {"name": "keyword.operator.arithmetic.lavra", "match": words("mais", "menos", "vezes", "módulo")},
            {"name": "keyword.operator.arithmetic.lavra", "match": B0 + r"dividido\s+por" + B1},
            {"name": "keyword.operator.arithmetic.lavra", "match": B0 + r"dividido(?=\s+" + CONTR + B1 + ")"},
            {"name": "keyword.operator.concatenation.lavra", "match": B0 + r"seguid[oa]\s+de" + B1},
            {"name": "keyword.operator.concatenation.lavra", "match": B0 + r"seguid[oa](?=\s+" + CONTR + B1 + ")"},
            {"name": "keyword.operator.conversion.lavra", "match": words("como")},
            {"name": "keyword.operator.logical.lavra", "match": words("não", "ou")},
            {"name": "keyword.operator.direction.lavra", "match": B0 + r"(?:à|a)\s+(?:esquerda|direita)" + B1},
        ]
    },
    "literals": {
        "patterns": [
            {"name": "constant.language.boolean.lavra", "match": words("verdadeiro", "falso")},
            {"name": "constant.language.nothing.lavra", "match": words("nada")},
        ]
    },
    "names": {
        "patterns": [
            name_after_article,
            name_after_contraction,
            # "um círculo", "uma conta": tipo ou caso nomeado pelo usuário.
            {
                "match": B0 + r"(?i:(um|uma))\s+(?!" + words(*TYPES, "cópia") + ")(" + WORD + ")" + B1,
                "captures": {"1": {"name": "keyword.other.article.lavra"}, "2": {"name": "entity.name.type.lavra"}},
            },
            {"name": "variable.other.symbol.lavra", "match": B0 + SYMBOL + B1},
            {"name": "keyword.operator.word.preposition.lavra", "match": words(*PREPOSITIONS)},
        ]
    },
    # Verbo do usuário no começo da oração: "Transfiro 300 ...", "e deixo a cidade".
    # Começa na margem, então precisa excluir tudo o que a língua já pinta (o TextMate
    # escolhe o casamento que começa antes) e as continuações de linha.
    "calls": {
        "patterns": [
            {
                "match": r"(?:^|(?<=:\s)|(?<=,\s)|(?<=\se\s))(\s*)(?!(?i:" + "|".join(sorted(NOT_VERBS, key=len, reverse=True)) + ")" + B1 + r")(?!" + SYMBOL + B1 + r")(\p{L}[\p{L}\p{N}_-]*)(?=\s|\.|$)",
                "captures": {"2": {"name": "entity.name.function.call.lavra"}},
            }
        ]
    },
    "punctuation": {
        "patterns": [
            {"name": "punctuation.terminator.period.lavra", "match": r"\."},
            {"name": "punctuation.section.block.lavra", "match": r":"},
            {"name": "punctuation.separator.lavra", "match": r"[,;]"},
            {"name": "punctuation.section.parens.lavra", "match": r"[()]"},
        ]
    },
}

grammar = {
    "$schema": "https://raw.githubusercontent.com/martinring/tmlanguage/master/tmlanguage.json",
    "name": "Lavra",
    "scopeName": "source.lavra",
    "fileTypes": ["lav"],
    "patterns": [
        {"include": "#comments"},
        {"include": "#strings"},
        {"include": "#definitions"},
        {"include": "#control"},
        {"include": "#builtins"},
        {"include": "#ownership"},
        {"include": "#types"},
        {"include": "#declarations"},
        {"include": "#operators"},
        {"include": "#literals"},
        {"include": "#numbers"},
        {"include": "#names"},
        {"include": "#calls"},
        {"include": "#punctuation"},
    ],
    "repository": repository,
}

out = Path(__file__).with_name("lavra.tmLanguage.json")
out.write_text(json.dumps(grammar, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(f"escrevi {out}")
