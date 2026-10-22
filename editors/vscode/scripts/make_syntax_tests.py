"""Gera os testes de gramática (formato do vscode-tmgrammar-test) a partir de casos
escritos como (linha, [(trecho, escopo), ...]). O gerador alinha os ^ nas colunas certas.

Rodar: `python3 -I scripts/make_syntax_tests.py` dentro de `editors/vscode`.
"""

from pathlib import Path

L = ".lavra"


def assertions(line: str, checks: list[tuple[str, str]]) -> list[str]:
    out = []
    pos = 0
    for text, scope in checks:
        col = line.index(text, pos)
        pos = col + len(text)
        if col < 3:
            # Começo da linha: "<-" afirma a partir da coluna 0, "~" pula colunas.
            out.append("--" + "<" + "~" * col + "-" * len(text) + " " + scope + L)
        else:
            out.append("--" + " " * (col - 2) + "^" * len(text) + " " + scope + L)
    return out


CASES = {
    "declaracoes": [
        ("A gravidade é 9,81.", [("A", "variable.other.article"), ("gravidade", "variable.other"), ("é", "storage.type.declaration"), ("9,81", "constant.numeric.decimal")]),
        ("O saldo começa em 1 000 000.", [("saldo", "variable.other"), ("começa", "storage.modifier.mutable"), ("em", "keyword.operator.word.preposition"), ("1 000 000", "constant.numeric.decimal")]),
        ("O limite, um inteiro de 32 bits, é 100.", [("um", "keyword.other.article"), ("inteiro", "storage.type.primitive"), ("32", "constant.numeric.decimal"), ("bits", "storage.type.primitive")]),
        ("As compras começam vazias.", [("compras", "variable.other"), ("começam", "storage.modifier.mutable"), ("vazias", "constant.language.empty")]),
        ("O apelido, talvez um texto, começa em nada.", [("talvez", "storage.modifier.maybe"), ("texto", "storage.type.primitive"), ("nada", "constant.language.nothing")]),
        ("A sobra começa numa cópia da herança.", [("cópia", "support.function.copy"), ("da", "keyword.operator.word.preposition"), ("herança", "variable.other")]),
    ],
    "mutacao": [
        ("    O saldo cresce o saldo vezes a taxa.", [("cresce", "keyword.other.mutation"), ("vezes", "keyword.operator.arithmetic"), ("taxa", "variable.other")]),
        ("    As notas ganham 7, e a mensagem vira \"pago\".", [("ganham", "keyword.other.mutation"), ("7", "constant.numeric.decimal"), ("vira", "keyword.other.mutation"), ("\"", "string.quoted.double")]),
        ("    Tomo a última das notas e escrevo o resultado.", [("Tomo", "support.function.builtin"), ("última", "keyword.other.position"), ("escrevo", "support.function.builtin"), ("resultado", "variable.language.result")]),
        ("    O item 2 das notas vira 9.", [("item", "keyword.other.position"), ("notas", "variable.other")]),
    ],
    "controle": [
        ("    Se o saldo for maior que o teto: paro.", [("Se", "keyword.control.conditional"), ("for", "keyword.operator.copula"), ("maior que", "keyword.operator.comparison"), ("paro", "keyword.control.flow")]),
        ("    Senão, se a nota for igual ao teto: pulo.", [("Senão", "keyword.control.conditional"), ("se", "keyword.control.conditional"), ("igual", "keyword.operator.comparison"), ("ao", "keyword.operator.word.preposition"), ("teto", "variable.other")]),
        ("    Enquanto o senado for paciente e n for no máximo 4:", [("Enquanto", "keyword.control.loop"), ("paciente", "entity.name.function.adjective"), ("n", "variable.other.symbol"), ("no máximo", "keyword.operator.comparison")]),
        ("    Repito sempre:", [("Repito", "keyword.control.loop"), ("sempre", "keyword.control.loop")]),
        ("    Para cada nota das notas:", [("Para", "keyword.control.loop"), ("cada", "keyword.control.loop"), ("nota", "variable.other"), ("notas", "variable.other")]),
        ("    Para cada i de 1 até 10:", [("i", "variable.other.symbol"), ("até", "keyword.control.loop")]),
        ("    Se não houver apelido: escrevo \"sem\".", [("não", "keyword.operator.logical"), ("houver", "keyword.operator.copula")]),
        ("    Conforme a figura:", [("Conforme", "keyword.control.conditional"), ("figura", "variable.other")]),
        ("        Sendo um círculo: devolvo o raio.", [("Sendo", "keyword.control.conditional"), ("um", "keyword.other.article"), ("círculo", "entity.name.type"), ("devolvo", "keyword.control.flow")]),
    ],
    "funcoes": [
        ("Para começar:", [("Para", "keyword.other.definition"), ("começar", "entity.name.function.main")]),
        ("Para render o saldo, sendo o saldo um real confiado:", [("Para", "keyword.other.definition"), ("render", "entity.name.function"), ("saldo", "variable.parameter"), ("sendo", "keyword.other.signature"), ("real", "storage.type.primitive"), ("confiado", "storage.modifier.lent")]),
        ("Para medir a carta, sendo a carta um texto emprestado, devolvendo um inteiro:", [("medir", "entity.name.function"), ("emprestado", "storage.modifier.borrowed"), ("devolvendo", "keyword.other.signature")]),
        ("Para o valor exceder o limite, sendo ambos inteiros, devolvendo um lógico:", [("valor", "variable.parameter"), ("exceder", "entity.name.function"), ("limite", "variable.parameter"), ("ambos", "keyword.other.both"), ("inteiros", "storage.type.primitive")]),
        ("Para o terreno ser quadrado, sendo o terreno um terreno emprestado, devolvendo um lógico:", [("ser", "entity.name.function"), ("quadrado", "entity.name.function.adjective"), ("terreno", "variable.parameter")]),
        ("Para trovejar (trovejo) a nuvem, sendo a nuvem um texto:", [("trovejar", "entity.name.function"), ("trovejo", "entity.name.function")]),
        ("Para dividir a conta entre as pessoas, sendo a conta e as pessoas inteiros, devolvendo um inteiro, salvo uma falha:", [("dividir", "entity.name.function"), ("entre", "keyword.operator.word.preposition"), ("salvo uma falha", "keyword.other.fallible")]),
        ("    Transfiro 300 da conta para a poupança.", [("Transfiro", "entity.name.function.call"), ("da", "keyword.operator.word.preposition"), ("para", "keyword.operator.word.preposition")]),
        ("    Ameaço a cidade pelo réu e deixo a cidade com o réu.", [("Ameaço", "entity.name.function.call"), ("pelo", "keyword.operator.word.preposition"), ("deixo", "entity.name.function.call")]),
        ("    A parte é o resultado de dividir a conta entre 3, ou desisto.", [("resultado", "variable.language.result"), ("dividir", "entity.name.function.call"), (", ou desisto", "keyword.control.flow")]),
        ("    Desisto com \"ninguém\".", [("Desisto", "keyword.control.flow")]),
        ("    Devolvo o fato de o valor ser maior que o limite.", [("Devolvo", "keyword.control.flow"), ("fato", "keyword.other.fact")]),
    ],
    "tipos": [
        ("Uma conta é um registro de:", [("Uma", "keyword.other.article"), ("conta", "entity.name.type"), ("um registro de", "keyword.other.definition")]),
        ("    o titular, um texto;", [("titular", "variable.other"), ("texto", "storage.type.primitive"), (";", "punctuation.separator")]),
        ("    as movimentações, uma lista de reais.", [("lista", "storage.type.primitive"), ("reais", "storage.type.primitive")]),
        ("Uma forma é uma destas:", [("forma", "entity.name.type"), ("uma destas", "keyword.other.definition")]),
        ("A carteira é uma conta com o titular \"Ana\" e o saldo 0,5.", [("conta", "entity.name.type"), ("titular", "variable.other")]),
    ],
    "expressoes": [
        ("    Escrevo (a diferença de 10 e 3) mais 1.", [("diferença", "support.function.nominal"), ("mais", "keyword.operator.arithmetic")]),
        ("    Escrevo o tamanho das notas seguido de \" itens\".", [("tamanho", "support.function.nominal"), ("seguido de", "keyword.operator.concatenation")]),
        ("    Escrevo o total como real dividido pelo lado.", [("como", "keyword.operator.conversion"), ("real", "storage.type.primitive"), ("dividido", "keyword.operator.arithmetic"), ("lado", "variable.other")]),
        ("    Escrevo o deslocamento de 1 por 4 à esquerda.", [("deslocamento", "support.function.nominal"), ("à esquerda", "keyword.operator.direction")]),
        ("    Escrevo 'ç' seguido de \"a\\nb\" seguido de verdadeiro.", [("'ç'", "constant.character"), ("\\n", "constant.character.escape"), ("verdadeiro", "constant.language.boolean")]),
        ("    Escrevo 0xFF sem quebrar a linha.", [("0xFF", "constant.numeric.hex"), ("sem quebrar a linha", "keyword.other.newline")]),
    ],
    "comentarios": [
        ("Nota: comentário que ocupa a linha.", [("Nota: comentário que ocupa a linha.", "comment.line.nota")]),
        ("    Escrevo \"a -- b\". -- comentário no fim", [("\"a -- b\"", "string.quoted.double"), ("-- comentário no fim", "comment.line.double-dash")]),
    ],
}

out_dir = Path(__file__).resolve().parent.parent / "tests" / "syntax"
out_dir.mkdir(parents=True, exist_ok=True)
for name, cases in CASES.items():
    lines = [f'-- SYNTAX TEST "source.lavra" "{name}"']
    for line, checks in cases:
        lines.append(line)
        lines.extend(assertions(line, checks))
    (out_dir / f"{name}.lav").write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"{len(CASES)} arquivos de teste em {out_dir}")
