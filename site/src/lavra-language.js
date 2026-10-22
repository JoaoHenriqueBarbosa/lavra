// A Lavra no editor do playground (CodeMirror): as palavras vêm do léxico do compilador
// (editors/lexicon.json), cada uma com a classe do papel que tem na língua; o resto é a
// sintaxe de comentário, texto e número. O sentido fino (o que é verbo do programa, o que
// é parâmetro) fica com os diagnósticos, que o próprio compilador dá.
import { HighlightStyle, StreamLanguage } from "@codemirror/language";
import { tags } from "@lezer/highlight";

/** A classe de cada papel do léxico. */
const ROLE = {
  article: "article",
  contraction: "article",
  preposition: "article",
  conjunction: "article",
  statement: "verb",
  flow: "verb",
  declaration: "linking",
  mutation: "linking",
  control: "keyword",
  copula: "keyword",
  definition: "keyword",
  signature: "keyword",
  ownership: "modifier",
  type: "type",
  nominal: "operator",
  operator: "operator",
  comparison: "operator",
  literal: "atom",
  position: "modifier",
  anaphora: "modifier",
  output: "operator",
};

/** A palavra, em minúsculas, e a classe dela. */
function classes(lexicon) {
  const map = new Map();
  const put = (word, cls) => {
    const w = word?.toLowerCase();
    if (w && cls && !map.has(w)) map.set(w, cls);
  };
  for (const v of lexicon.coreVerbs) put(v.firstPerson, "verb");
  for (const v of lexicon.subjectVerbs) {
    put(v.singular, "linking");
    put(v.plural, "linking");
  }
  for (const t of lexicon.types) {
    put(t.singular, "type");
    put(t.plural, "type");
  }
  for (const a of lexicon.articles) put(a.form, "article");
  for (const c of lexicon.contractions) put(c.form, "article");
  for (const p of lexicon.prepositions) put(p, "article");
  for (const k of lexicon.keywords) put(k.form, ROLE[k.role]);
  for (const n of lexicon.nominals) put(n.word, "operator");
  for (const p of lexicon.participles) for (const f of p.forms) put(f, "modifier");
  for (const o of lexicon.ordinals) for (const end of ["o", "a", "os", "as"]) put(o.stem + end, "modifier");
  for (const list of Object.values(lexicon.locutions)) for (const parts of list) for (const w of parts) put(w, "operator");
  return map;
}

/** A língua para o CodeMirror. */
export function lavra(lexicon) {
  const words = classes(lexicon);
  return StreamLanguage.define({
    name: "lavra",
    startState: () => ({ lineStart: true, header: false }),
    token(stream, state) {
      if (stream.sol()) {
        state.lineStart = true;
        state.header = false;
      }
      // "Nota:" no começo da linha, e "--" até o fim dela.
      if (state.lineStart && stream.match(/^\s*nota:/i)) {
        stream.skipToEnd();
        return "comment";
      }
      if (stream.eatSpace()) return null;
      const first = state.lineStart;
      state.lineStart = false;
      if (stream.match("--")) {
        stream.skipToEnd();
        return "comment";
      }
      if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) return "string";
      if (stream.match(/^'(?:[^'\\]|\\.)'?/)) return "string";
      if (stream.match(/^0x[0-9a-fA-F]+/) || stream.match(/^\d+(?: \d{3})*(?:,\d+)?/)) return "number";
      if (stream.match(/^[\p{L}_][\p{L}\p{N}_-]*/u)) {
        const word = stream.current();
        const lower = word.toLowerCase();
        // "Para render a quantia": depois do "Para" que abre a linha vem o verbo da função
        // (ou "cada" e "começar", que são da língua).
        if (state.header) {
          state.header = false;
          if (!words.has(lower)) return "verb";
        }
        if (first && lower === "para") state.header = true;
        const cls = words.get(lower);
        if (cls) return cls;
        // A palavra com maiúscula que abre a oração e a língua não conhece é um verbo do
        // programa na 1ª pessoa ("Rendo o saldo.").
        if (first && word[0] !== word[0].toLowerCase()) return "verb";
        return "name";
      }
      stream.next();
      return "punctuation";
    },
    tokenTable: {
      verb: tags.function(tags.variableName),
      linking: tags.operatorKeyword,
      keyword: tags.controlKeyword,
      type: tags.typeName,
      operator: tags.operator,
      modifier: tags.modifier,
      article: tags.meta,
      atom: tags.atom,
      name: tags.variableName,
    },
  });
}

/** As cores, pelas variáveis do CSS do site (claro e escuro). */
export const lavraHighlight = HighlightStyle.define([
  { tag: tags.comment, color: "var(--cm-comment)", fontStyle: "italic" },
  { tag: tags.string, color: "var(--cm-string)" },
  { tag: tags.number, color: "var(--cm-number)" },
  { tag: tags.controlKeyword, color: "var(--cm-keyword)" },
  { tag: tags.operatorKeyword, color: "var(--cm-linking)" },
  { tag: tags.function(tags.variableName), color: "var(--cm-verb)" },
  { tag: tags.typeName, color: "var(--cm-type)" },
  { tag: tags.operator, color: "var(--cm-operator)" },
  { tag: tags.modifier, color: "var(--cm-modifier)" },
  { tag: tags.meta, color: "var(--cm-article)" },
  { tag: tags.atom, color: "var(--cm-atom)" },
  { tag: tags.variableName, color: "var(--cm-name)" },
  { tag: tags.punctuation, color: "var(--cm-punctuation)" },
]);
