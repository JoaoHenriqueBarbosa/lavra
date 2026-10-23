/**
 * @file Gramática tree-sitter da Lavra.
 *
 * Rasa de propósito: orações, blocos, cabeçalhos, definições de tipo, comentários, textos,
 * números, palavras por papel e nomes com artigo. O detalhe fino (o que é verbo do
 * programa, parâmetro emprestado, campo) fica com os tokens semânticos do servidor de
 * linguagem (`lavrac lsp`).
 *
 * As palavras vêm do léxico que o compilador exporta (`editors/lexicon.json`): aqui ficam só
 * as regras que as combinam. O recuo, o fim da linha lógica (a linha que termina em ".",
 * ":" ou ";"), os comentários e o conteúdo dos textos ficam com o scanner externo
 * (`src/scanner.c`), que segue as regras do léxico do compilador.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const lexicon = require('../lexicon.json');

// A ordem dos papéis decide o papel de uma palavra que tem mais de um no léxico: cada
// palavra é um token só, com um papel só ("a" é artigo antes de ser preposição; "para" é
// preposição no meio da oração, e as construções que começam por "Para" usam a palavra
// diretamente).
const ROLE_ORDER = [
  'article',
  'contraction',
  'conjunction',
  'statement',
  'flow',
  'declaration',
  'mutation',
  'copula',
  'control',
  'signature',
  'ownership',
  'type',
  'nominal',
  'operator',
  'comparison',
  'literal',
  'position',
  'anaphora',
  'output',
  'preposition',
  'definition',
];

// O nó de cada papel. O artigo se divide em definido (o que marca o nome) e indefinido
// (o do tipo e do caso), pelo próprio léxico.
const ROLE_NODES = {
  contraction: 'contraction',
  preposition: 'preposition',
  conjunction: 'conjunction',
  statement: 'core_verb',
  flow: 'flow_verb',
  declaration: 'declaration_verb',
  mutation: 'mutation_verb',
  control: 'control_keyword',
  copula: 'copula',
  definition: 'definition_keyword',
  signature: 'signature_keyword',
  ownership: 'ownership_keyword',
  type: 'primitive_type',
  nominal: 'nominal',
  operator: 'operator',
  comparison: 'comparison',
  literal: 'literal',
  position: 'position',
  anaphora: 'anaphora',
  output: 'output',
};

// Papéis que não reservam a palavra: "nota" (comentário só no começo da linha) e as formas
// que a língua recusa com dica podem ser nomes do programa ("a nota", "guardo").
const NOT_RESERVED = new Set(['comment', 'rejected', 'reserved']);

// Os papéis de que uma oração comum não começa: as construções que começam por essas
// palavras ("Se", "Para", "Uso", "Uma") têm regra própria.
const NOT_SENTENCE_START = new Set(['control_keyword', 'preposition', 'definition_keyword', 'indefinite_article']);

/** A forma em regex, sem distinguir maiúsculas (o léxico do compilador não distingue). */
function kw(form) {
  const escaped = form.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/ /g, '\\s+');
  return new RegExp(escaped, 'i');
}

/** O papel de cada palavra do léxico, e as palavras de cada nó. */
function wordsByNode() {
  const locutions = [...lexicon.locutions.contractible, ...lexicon.locutions.protected].map((parts) => parts.join(' '));
  const roles = new Map();
  const addRole = (form, role) => {
    if (!roles.has(form)) roles.set(form, new Set());
    roles.get(form).add(role);
  };
  for (const w of lexicon.words) {
    if (NOT_RESERVED.has(w.role)) continue;
    if (locutions.includes(w.form)) {
      // A locução se lê palavra por palavra ("igual" e "a", ou "igual" e "ao"): cada
      // palavra que a língua não tem em outro papel fica com o papel da locução.
      for (const part of w.form.split(' ')) addRole(part, `${w.role}:part`);
      continue;
    }
    addRole(w.form, w.role);
  }
  const definite = new Set(lexicon.articles.filter((a) => a.definite).map((a) => a.form));
  const nodes = {};
  for (const [form, set] of roles) {
    const own = ROLE_ORDER.filter((r) => set.has(r));
    const role = own.length > 0 ? own[0] : ROLE_ORDER.find((r) => set.has(`${r}:part`));
    if (!role) continue;
    const node = role === 'article' ? (definite.has(form) ? 'article' : 'indefinite_article') : ROLE_NODES[role];
    (nodes[node] = nodes[node] || []).push(form);
  }
  for (const forms of Object.values(nodes)) forms.sort();
  return nodes;
}

const WORDS = wordsByNode();
const ROLE_RULES = Object.keys(WORDS).sort();

/** As regras de cada papel: um nó por papel, com as palavras do léxico. */
function roleRules() {
  const rules = {};
  for (const node of ROLE_RULES) {
    const forms = WORDS[node].map(kw);
    rules[node] = (_) => (forms.length === 1 ? forms[0] : choice(...forms));
  }
  return rules;
}

module.exports = grammar({
  name: 'lavra',

  word: ($) => $.identifier,

  externals: ($) => [$._newline, $._indent, $._dedent, $.string_content, $.comment, $._error_sentinel],

  // O recuo e o fim da linha lógica são tokens do scanner; a quebra de uma linha que
  // continua na de baixo (enjambement) é espaço.
  extras: ($) => [/[ \t\r\n]/, $.comment],

  supertypes: ($) => [$._statement],

  rules: {
    source_file: ($) => repeat($._statement),

    _statement: ($) =>
      choice(
        $.sentence,
        $.import,
        $.if_statement,
        $.else_statement,
        $.while_statement,
        $.repeat_statement,
        $.for_statement,
        $.match_statement,
        $.match_arm,
        $.function_definition,
        $.main_definition,
        $.type_definition,
        $.variant,
      ),

    // ------------------------------------------------------------------ orações

    // Um período: palavras até o ponto final (ou o ponto e vírgula, entre os campos de um
    // registro), possivelmente em várias linhas.
    sentence: ($) => seq($._sentence_start, optional($._items), $._terminator, optional($._newline)),

    _terminator: (_) => choice('.', ';'),

    _items: ($) => repeat1($._item),
    _item: ($) => choice($.name, $._other_item),

    // Nos cabeçalhos de função, os nomes são os parâmetros.
    _header_items: ($) => repeat1(choice(alias($.name, $.parameter), $._other_item)),

    _other_item: ($) =>
      choice($.identifier, $.symbol, $.number, $.string, $.character, $.parenthesized, ',', ...ROLE_RULES.map((r) => $[r])),

    _sentence_start: ($) =>
      choice(
        $.name,
        $.identifier,
        $.symbol,
        $.number,
        $.string,
        $.character,
        $.parenthesized,
        ...ROLE_RULES.filter((r) => !NOT_SENTENCE_START.has(r)).map((r) => $[r]),
      ),

    parenthesized: ($) => seq('(', optional($._items), ')'),

    // O nome de um valor: o artigo (ou a contração com o artigo) e a palavra.
    name: ($) => prec(1, seq(choice($.article, $.contraction), $.identifier)),

    // Qualquer palavra, também da língua: o adjetivo de um predicado ("ser vazio"), o nome
    // de uma biblioteca ("textos").
    _word: ($) => choice($.identifier, $.symbol, ...ROLE_RULES.map((r) => $[r])),

    // ------------------------------------------------------------------- blocos

    // Depois dos dois-pontos: um bloco recuado nas linhas de baixo, ou um período na mesma
    // linha.
    _body: ($) => choice(seq($._newline, $.block), $._statement),

    block: ($) => seq($._indent, repeat1($._statement), $._dedent),

    if_statement: ($) => seq(alias(kw('se'), $.conditional_keyword), optional($._items), ':', $._body),
    // "Senão:" e "Senão, se ...:", depois de um "Se" ou dos casos de um "Conforme".
    else_statement: ($) => seq(alias(kw('senão'), $.conditional_keyword), optional($._items), ':', $._body),
    while_statement: ($) => seq(alias(kw('enquanto'), $.repeat_keyword), optional($._items), ':', $._body),
    repeat_statement: ($) => seq(alias(kw('repito'), $.repeat_keyword), optional($._items), ':', $._body),
    for_statement: ($) =>
      seq(alias(kw('para'), $.repeat_keyword), alias(kw('cada'), $.repeat_keyword), optional($._items), ':', $._body),
    match_statement: ($) => seq(alias(kw('conforme'), $.conditional_keyword), optional($._items), ':', $._body),
    match_arm: ($) => seq(alias(kw('sendo'), $.conditional_keyword), optional($._items), ':', $._body),

    // ----------------------------------------------------------------- definições

    // "Para render a quantia, sendo ...:", "Para o valor exceder o limite, ...:",
    // "Para o número ser par, ...:".
    function_definition: ($) =>
      seq(
        alias(kw('para'), $.function_keyword),
        choice(
          field('name', $.identifier),
          seq(
            field('subject', choice(alias($.name, $.parameter), $.symbol)),
            choice(field('name', $.identifier), seq(alias(kw('ser'), $.declaration_verb), field('name', $._word))),
          ),
        ),
        optional($._header_items),
        ':',
        $._body,
      ),

    main_definition: ($) =>
      seq(alias(kw('para'), $.function_keyword), field('name', alias(kw('começar'), $.identifier)), ':', $._body),

    // "Uso a biblioteca de textos."
    import: ($) =>
      seq(
        alias(kw('uso'), $.import_keyword),
        alias(kw('a'), $.article),
        alias(kw('biblioteca'), $.import_keyword),
        alias(kw('de'), $.preposition),
        field('library', $._word),
        '.',
        optional($._newline),
      ),

    // "Uma conta é um registro de:", "Uma forma é uma destas:", com os campos ou os casos
    // recuados.
    type_definition: ($) => seq($._type_head, ':', $._body),
    // Um caso de enumeração: "um círculo, com o raio, um real;".
    variant: ($) => seq($._type_head, $._terminator, optional($._newline)),
    _type_head: ($) => seq($.indefinite_article, field('name', $.identifier), optional($._items)),

    // ------------------------------------------------------------------- tokens

    // Inteiro, real com vírgula decimal e milhar com espaço ("1 000 000"), hexadecimal.
    number: (_) => token(choice(/0[xX][0-9a-fA-F]+/, /-?[0-9]+( [0-9]{3})*(,[0-9]+)?/)),

    string: ($) =>
      seq(
        alias(/["“”]/, '"'),
        repeat(choice($.string_content, $.escape_sequence)),
        alias(token.immediate(/["“”]/), '"'),
      ),

    escape_sequence: (_) => token.immediate(/\\./),

    character: (_) => token(/['‘’]([^'‘’\\\n]|\\.)['‘’]/),

    // As palavras da língua, um nó por papel.
    ...roleRules(),

    // O símbolo e a palavra comum vêm por último: diante do mesmo texto, o gerador prefere o
    // token definido antes, e as palavras da língua e o símbolo ganham da palavra comum.

    // Símbolo: uma letra com algarismos opcionais, sem artigo ("x", "n", "p1"); "a", "e" e
    // "o" sozinhos são artigo e conjunção.
    symbol: (_) => /[b-df-np-zB-DF-NP-Z][0-9]*|[aeoAEO][0-9]+/,

    identifier: (_) => /[\p{L}_][\p{L}\p{M}0-9_]*(-\p{L}[\p{L}\p{M}0-9_]*)*/,
  },
});
