/* Lavra — linguagem de programação de baixo nível escrita em português gramatical.
   Este arquivo contém: léxico, morfologia, análise sintática com verificação de
   concordância, regência, modo verbal e tipos, e uma máquina que executa o programa
   sobre uma memória de células (pilha + monte). */
(function (raiz) {
'use strict';

// ============================================================ ERROS
class ErroLavra extends Error {
  constructor(tipo, msg, tok, dica) {
    super(msg);
    this.tipo = tipo;
    this.linha = tok && tok.line ? tok.line : null;
    this.coluna = tok && tok.col ? tok.col : null;
    this.dica = dica || null;
  }
}
function falha(tipo, msg, tok, dica) { throw new ErroLavra(tipo, msg, tok, dica); }
const ROTULO_ERRO = {
  'léxico': 'Erro léxico', 'morfologia': 'Erro de morfologia', 'concordância': 'Erro de concordância',
  'regência': 'Erro de regência', 'sintaxe': 'Erro de sintaxe', 'escopo': 'Erro de escopo',
  'tipo': 'Erro de tipo', 'execução': 'Erro de execução'
};

// ============================================================ MORFOLOGIA
const ARTIGOS = {
  o: { def: true, g: 'm', n: 's' }, a: { def: true, g: 'f', n: 's' },
  os: { def: true, g: 'm', n: 'p' }, as: { def: true, g: 'f', n: 'p' },
  um: { def: false, g: 'm', n: 's' }, uma: { def: false, g: 'f', n: 's' },
  uns: { def: false, g: 'm', n: 'p' }, umas: { def: false, g: 'f', n: 'p' }
};
const artDef = (g, n) => g === 'f' ? (n === 'p' ? 'as' : 'a') : (n === 'p' ? 'os' : 'o');
const artIndef = (g, n) => g === 'f' ? (n === 'p' ? 'umas' : 'uma') : (n === 'p' ? 'uns' : 'um');
const descGN = (g, n) => (g === 'f' ? 'feminino' : 'masculino') + ' ' + (n === 'p' ? 'plural' : 'singular');

const CONTRACOES = {
  do: ['de', 'o'], da: ['de', 'a'], dos: ['de', 'os'], das: ['de', 'as'],
  no: ['em', 'o'], na: ['em', 'a'], nos: ['em', 'os'], nas: ['em', 'as'],
  ao: ['a', 'o'], 'à': ['a', 'a'], aos: ['a', 'os'], 'às': ['a', 'as'],
  pelo: ['por', 'o'], pela: ['por', 'a'], pelos: ['por', 'os'], pelas: ['por', 'as'],
  num: ['em', 'um'], numa: ['em', 'uma'], dum: ['de', 'um'], duma: ['de', 'uma']
};
const CONTRAI = {};
for (const [c, [p, a]] of Object.entries(CONTRACOES)) CONTRAI[p + ' ' + a] = c;
const contrair = (p, a) => CONTRAI[p + ' ' + a] || (p + ' ' + a);

const LOC_PROTEGIDAS = [['pelo', 'menos'], ['no', 'máximo'], ['à', 'esquerda'], ['à', 'direita']];
const LOCUCOES = [['maior', 'que'], ['menor', 'que'], ['igual', 'a'], ['diferente', 'de'],
  ['dividido', 'por'], ['seguido', 'de'], ['seguida', 'de']];
const PREPOSICOES = new Set(['a', 'de', 'em', 'por', 'com', 'para', 'sobre', 'contra', 'entre', 'até', 'sem', 'desde']);

const NUCLEO = {
  seja: 'ser', sejam: 'ser', guardo: 'guardar', acrescento: 'acrescentar', subtraio: 'subtrair',
  multiplico: 'multiplicar', divido: 'dividir', escrevo: 'escrever', leio: 'ler', devolvo: 'devolver',
  repito: 'repetir', paro: 'parar', pulo: 'pular', reservo: 'reservar', libero: 'liberar'
};
const INF_NUCLEO = {};
for (const [f, l] of Object.entries(NUCLEO)) if (!INF_NUCLEO[l]) INF_NUCLEO[l] = f;

const IRREG_1PS = {
  ser: 'sou', estar: 'estou', ter: 'tenho', ir: 'vou', vir: 'venho', fazer: 'faço', desfazer: 'desfaço',
  refazer: 'refaço', satisfazer: 'satisfaço', dizer: 'digo', trazer: 'trago', ler: 'leio', reler: 'releio',
  crer: 'creio', ver: 'vejo', rever: 'revejo', prever: 'prevejo', prover: 'provejo', medir: 'meço',
  pedir: 'peço', impedir: 'impeço', despedir: 'despeço', expedir: 'expeço', ouvir: 'ouço', perder: 'perco',
  poder: 'posso', saber: 'sei', querer: 'quero', requerer: 'requeiro', caber: 'caibo', valer: 'valho',
  dar: 'dou', haver: 'hei', conter: 'contenho', manter: 'mantenho', obter: 'obtenho', deter: 'detenho',
  reter: 'retenho', entreter: 'entretenho', intervir: 'intervenho', convir: 'convenho', provir: 'provenho',
  odiar: 'odeio', ansiar: 'anseio', remediar: 'remedeio', incendiar: 'incendeio', mediar: 'medeio',
  intermediar: 'intermedeio', rir: 'rio', sorrir: 'sorrio'
};
const IRREG_SUBJ = {
  ser: 'for', ir: 'for', ter: 'tiver', estar: 'estiver', fazer: 'fizer', desfazer: 'desfizer',
  refazer: 'refizer', satisfazer: 'satisfizer', poder: 'puder', caber: 'couber', saber: 'souber',
  querer: 'quiser', trazer: 'trouxer', dizer: 'disser', vir: 'vier', ver: 'vir', rever: 'revir',
  prever: 'previr', haver: 'houver', dar: 'der', conter: 'contiver', manter: 'mantiver', obter: 'obtiver',
  deter: 'detiver', reter: 'retiver', intervir: 'intervier', convir: 'convier', provir: 'provier'
};

function conjugar1ps(inf) {
  if (IRREG_1PS[inf]) return IRREG_1PS[inf];
  if (/p[oô]r$/.test(inf)) return inf.replace(/p[oô]r$/, 'ponho');
  if (/ear$/.test(inf)) return inf.slice(0, -3) + 'eio';
  if (/air$/.test(inf)) return inf.slice(0, -3) + 'aio';
  const m = inf.match(/^(.+)(ar|er|ir)$/);
  if (!m) return null;
  let r = m[1];
  const t = m[2];
  if (t === 'ir') {
    r = r.replace(/e(rt|rv|rn|st|nt|gu|r|t|d)$/, 'i$1');
    r = r.replace(/o(rm|br|ss|l)$/, 'u$1');
  }
  if (t !== 'ar') {
    if (/gu$/.test(r)) r = r.slice(0, -2) + 'g';
    else if (/c$/.test(r)) r = r.slice(0, -1) + 'ç';
    else if (/g$/.test(r)) r = r.slice(0, -1) + 'j';
  }
  return r + 'o';
}
function futuroSubj(inf) {
  if (IRREG_SUBJ[inf]) return IRREG_SUBJ[inf];
  if (/p[oô]r$/.test(inf)) return inf.replace(/p[oô]r$/, 'puser');
  return inf;
}
function terceiraPessoa(inf) {
  const m = inf.match(/^(.+)(ar|er|ir)$/);
  if (!m) return null;
  return m[1] + (m[2] === 'ar' ? 'a' : 'e');
}
const ACENTOS = /[áéíóúâêôàãõ]/;
const semAcento = s => s.normalize('NFD').replace(/[̀-ͯ]/g, '');
function pluralizar(p) {
  if (p.includes('-')) return p.split('-').map(pluralizar).join('-');
  if (/ão$/.test(p)) return p.slice(0, -2) + 'ões';
  if (/[aeiouáéíóúâêôã]$/.test(p)) return p + 's';
  if (/m$/.test(p)) return p.slice(0, -1) + 'ns';
  if (/al$/.test(p)) return p.slice(0, -1) + 'is';
  if (/el$/.test(p)) return p.slice(0, -2) + (ACENTOS.test(p.slice(0, -2)) ? 'eis' : 'éis');
  if (/ol$/.test(p)) return p.slice(0, -2) + (ACENTOS.test(p.slice(0, -2)) ? 'ois' : 'óis');
  if (/ul$/.test(p)) return p.slice(0, -1) + 'is';
  if (/il$/.test(p)) return ACENTOS.test(p.slice(0, -2)) ? p.slice(0, -2) + 'eis' : p.slice(0, -1) + 's';
  if (/[rz]$/.test(p)) return p + 'es';
  if (/[êéáó]s$/.test(p)) return p.slice(0, -2) + semAcento(p.slice(-2, -1)) + 'ses';
  if (/[sx]$/.test(p)) return p;
  return p + 's';
}
// flexão do adjetivo: base escrita no gênero g0 do sujeito da definição
function flexAdj(base, g0, g, n) {
  let f = base;
  if (g !== g0) {
    if (g === 'f' && /o$/.test(base)) f = base.slice(0, -1) + 'a';
    else if (g === 'f' && /ês$/.test(base)) f = base.slice(0, -2) + 'esa';
    else if (g === 'm' && /a$/.test(base)) f = base.slice(0, -1) + 'o';
  }
  return n === 'p' ? pluralizar(f) : f;
}
const ORDINAIS = { primeir: 1, segund: 2, terceir: 3, quart: 4, quint: 5, sext: 6, 'sétim': 7, oitav: 8, non: 9, 'décim': 10, 'últim': -1 };
function analisarOrdinal(w) {
  const m = w.match(/^(.+?)(o|a|os|as)$/);
  if (!m || !(m[1] in ORDINAIS)) return null;
  return { valor: ORDINAIS[m[1]], g: m[2][0] === 'a' ? 'f' : 'm', n: m[2].endsWith('s') ? 'p' : 's' };
}

// ============================================================ TIPOS
const T = {
  int: (b = 64) => ({ k: 'int', bits: b, sinal: true }),
  nat: (b = 64) => ({ k: 'int', bits: b, sinal: false }),
  byte: { k: 'int', bits: 8, sinal: false, byte: true },
  real: (b = 64) => ({ k: 'real', bits: b }),
  bool: { k: 'bool' }, car: { k: 'car' }, texto: { k: 'texto' }, nada: { k: 'nada' },
  ptr: to => ({ k: 'ptr', to }), seq: (n, of) => ({ k: 'seq', n, of }), rec: r => ({ k: 'rec', r })
};
const TIPO_SING = { inteiro: 'm', natural: 'm', real: 'm', byte: 'm', 'lógico': 'm', caractere: 'm', texto: 'm', 'endereço': 'm', 'sequência': 'f' };
const TIPO_PLUR = { inteiros: 'inteiro', naturais: 'natural', reais: 'real', bytes: 'byte', 'lógicos': 'lógico', caracteres: 'caractere', textos: 'texto', 'endereços': 'endereço', 'sequências': 'sequência' };
function descTipo(t, pl) {
  const p = (s, x) => pl ? x : s;
  switch (t.k) {
    case 'int':
      if (t.byte) return p('byte', 'bytes');
      return (t.sinal ? p('inteiro', 'inteiros') : p('natural', 'naturais')) + (t.bits !== 64 ? ` de ${t.bits} bits` : '');
    case 'real': return p('real', 'reais') + (t.bits !== 64 ? ' de 32 bits' : '');
    case 'bool': return p('lógico', 'lógicos');
    case 'car': return p('caractere', 'caracteres');
    case 'texto': return p('texto', 'textos');
    case 'nada': return 'nada';
    case 'ptr': return p('endereço', 'endereços') + ' de ' + descTipo(t.to);
    case 'seq': return p('sequência', 'sequências') + ` de ${t.n} ${descTipo(t.of, true)}`;
    case 'rec': return pl ? t.r.plural : t.r.nome;
  }
  return '?';
}
const generoTipo = t => t.k === 'seq' ? 'f' : t.k === 'rec' ? t.r.g : 'm';
function tam(t) { return t.k === 'seq' ? t.n * tam(t.of) : t.k === 'rec' ? t.r.tamanho : 1; }
function igualTipo(a, b) {
  if (a.k !== b.k) return false;
  switch (a.k) {
    case 'int': return a.bits === b.bits && a.sinal === b.sinal;
    case 'real': return a.bits === b.bits;
    case 'ptr': return igualTipo(a.to, b.to);
    case 'seq': return a.n === b.n && igualTipo(a.of, b.of);
    case 'rec': return a.r === b.r;
    default: return true;
  }
}
const ehNum = t => t.k === 'int' || t.k === 'real';
const ehByte = t => t.k === 'int' && t.bits === 8 && !t.sinal;
function compativel(a, f) {
  if (igualTipo(a, f)) return null;
  if (a.k === 'int' && f.k === 'int') return null;
  if (a.k === 'real' && ehNum(f)) return null;
  if (a.k === 'int' && f.k === 'real') return `um real não cabe direto em ${descTipo(a)}: converta com "como ${descTipo(a)}".`;
  if (a.k === 'ptr' && f.k === 'nada') return null;
  if (a.k === 'ptr' && f.k === 'ptr' && (igualTipo(a.to, f.to) || ehByte(a.to) || ehByte(f.to))) return null;
  if ((a.k === 'car' && f.k === 'int') || (a.k === 'int' && f.k === 'car'))
    return `caractere e número são tipos diferentes: converta com "como ${descTipo(a)}".`;
  return `esperava ${descTipo(a)}, mas o valor é ${descTipo(f)}.`;
}
const ehSimbolo = v => /^[b-df-np-z][0-9]*$|^[aeo][0-9]+$/.test(v);
const OPS_BIN = {
  soma: { g: 'f', op: '+', comut: true }, 'diferença': { g: 'f', op: '-' }, produto: { g: 'm', op: '*', comut: true },
  quociente: { g: 'm', op: '/' }, resto: { g: 'm', op: '%' }, 'potência': { g: 'f', op: '**' },
  'interseção': { g: 'f', op: '&', comut: true }, 'união': { g: 'f', op: '|', comut: true }, 'disjunção': { g: 'f', op: '^', comut: true }
};
const OPS_UN = { negativo: { g: 'm', op: 'neg' }, inverso: { g: 'm', op: '~' }, raiz: { g: 'f', op: 'raiz' }, tamanho: { g: 'm', op: 'tam' } };
const NOMES_OP = { '+': 'soma', '-': 'diferença', '*': 'produto', '/': 'quociente', '%': 'resto', '**': 'potência', '&': 'interseção', '|': 'união', '^': 'disjunção', '<<': 'deslocamento à esquerda', '>>': 'deslocamento à direita' };
const RESERVADOS = new Set([...Object.keys(OPS_BIN), ...Object.keys(OPS_UN), ...Object.keys(TIPO_SING), ...Object.keys(TIPO_PLUR),
  'deslocamento', 'resultado', 'fato', 'item', 'apontado', 'verdadeiro', 'falso', 'nada', 'registro']);
const PALAVRAS_ACENTO = ['senão', 'sequência', 'sequências', 'lógico', 'lógicos', 'endereço', 'endereços', 'potência', 'interseção',
  'união', 'disjunção', 'diferença', 'até', 'máximo', 'último', 'última', 'décimo', 'sétimo', 'não', 'módulo', 'no máximo', 'à esquerda', 'à direita'];

// ============================================================ LÉXICO
const LETRA = 'A-Za-zÀ-ÖØ-öø-ÿ';
const RE_PALAVRA = new RegExp(`^[${LETRA}_][${LETRA}0-9_]*(?:-[${LETRA}0-9_]+)*`);

function tokenizarLinha(linha, nl) {
  const out = [];
  let i = 0;
  while (i < linha.length) {
    const c = linha[i];
    if (c === ' ') { i++; continue; }
    const col = i + 1;
    if (c === '-' && linha[i + 1] === '-') break;
    if (c === '"') {
      let j = i + 1, s = '';
      while (j < linha.length && linha[j] !== '"') {
        if (linha[j] === '\\' && j + 1 < linha.length) { const e = linha[j + 1]; s += e === 'n' ? '\n' : e === 't' ? '\t' : e; j += 2; }
        else s += linha[j++];
      }
      if (j >= linha.length) falha('léxico', 'Texto aberto com aspas e nunca fechado.', { line: nl, col });
      out.push({ t: 's', v: s, o: linha.slice(i, j + 1), line: nl, col });
      i = j + 1; continue;
    }
    if (c === "'") {
      let j = i + 1, ch;
      if (linha[j] === '\\') { const e = linha[j + 1]; ch = e === 'n' ? '\n' : e === 't' ? '\t' : e === '0' ? '\0' : e; j += 2; }
      else {
        const cp = linha.codePointAt(j);
        if (cp === undefined || linha[j] === "'") falha('léxico', "Caractere vazio: escreva um símbolo entre as aspas simples, como 'a'.", { line: nl, col });
        ch = String.fromCodePoint(cp); j += ch.length;
      }
      if (linha[j] !== "'") falha('léxico', "Entre aspas simples vai um único caractere: 'a'. Para textos use aspas duplas.", { line: nl, col });
      out.push({ t: 'c', v: ch.codePointAt(0), o: linha.slice(i, j + 1), line: nl, col });
      i = j + 1; continue;
    }
    const resto = linha.slice(i);
    let m;
    if ((m = resto.match(/^-?0x[0-9a-fA-F]+/))) {
      const neg = m[0][0] === '-';
      const v = BigInt(m[0].replace('-', ''));
      out.push({ t: 'n', real: false, val: neg ? -v : v, o: m[0], line: nl, col });
      i += m[0].length; continue;
    }
    if ((m = resto.match(/^-?\d+(?: \d{3}(?![\d,]\d))*(?:,\d+)?/))) {
      const txt = m[0], limpo = txt.replace(/ /g, '');
      const real = limpo.includes(',');
      out.push({ t: 'n', real, val: real ? parseFloat(limpo.replace(',', '.')) : BigInt(limpo), o: txt, line: nl, col });
      i += txt.length; continue;
    }
    if ((m = resto.match(RE_PALAVRA))) {
      out.push({ t: 'w', v: m[0].toLowerCase(), o: m[0], line: nl, col });
      i += m[0].length; continue;
    }
    if ('.,:;()'.includes(c)) { out.push({ t: 'p', v: c, o: c, line: nl, col }); i++; continue; }
    falha('léxico', `Caractere inesperado: "${c}".`, { line: nl, col },
      c === '?' || c === '!' ? 'Lavra não usa "?" nem "!". Perguntas viram condições com "Se".' :
      '+-*/=<>'.includes(c) ? 'Operações se escrevem por extenso: "mais", "menos", "vezes", "dividido por", "a soma de X e Y".' : null);
  }
  return out;
}

function fundir(toks, pares) {
  const out = [];
  for (let i = 0; i < toks.length; i++) {
    const a = toks[i], b = toks[i + 1];
    let feito = false;
    if (a.t === 'w' && b && b.t === 'w' && a.line === b.line) {
      for (const [x, y] of pares) {
        if (a.v === x && b.v === y && !(a.contr && a.parte === 2)) {
          out.push({ t: 'w', v: x + ' ' + y, o: a.o + ' ' + b.o, loc: true, contr: b.contr || null, line: a.line, col: a.col, col2: b.col, parte2: b.parte, bol: a.bol });
          i++; feito = true; break;
        }
      }
    }
    if (!feito) out.push(a);
  }
  return out;
}
function desfazerContracoes(toks) {
  const out = [];
  for (const tk of toks) {
    if (tk.t === 'w' && !tk.loc && CONTRACOES[tk.v]) {
      const [p, a] = CONTRACOES[tk.v];
      out.push({ t: 'w', v: p, o: tk.o, contr: tk.v, parte: 1, line: tk.line, col: tk.col, bol: tk.bol });
      out.push({ t: 'w', v: a, o: '', contr: tk.v, parte: 2, line: tk.line, col: tk.col });
    } else out.push(tk);
  }
  return out;
}

function lexar(fonte) {
  fonte = fonte.replace(/\r\n?/g, '\n').replace(/\t/g, '    ')
    .replace(/[“”«»„]/g, '"').replace(/[‘’]/g, "'").replace(/ /g, ' ');
  const linhas = fonte.split('\n');
  const brutos = [];
  const pilha = [0];
  let cont = false, ultimo = null;
  for (let k = 0; k < linhas.length; k++) {
    const linha = linhas[k], nl = k + 1, aparada = linha.trim();
    if (!aparada || /^nota\s*:/i.test(aparada) || aparada.startsWith('--')) continue;
    const lt = tokenizarLinha(linha, nl);
    if (!lt.length) continue;
    lt[0].bol = true;
    if (!cont) {
      const recuo = linha.length - linha.trimStart().length;
      const topo = pilha[pilha.length - 1];
      if (recuo > topo) { pilha.push(recuo); brutos.push({ t: 'INDENT', line: nl, col: 1, o: '' }); }
      else if (recuo < topo) {
        while (recuo < pilha[pilha.length - 1]) { pilha.pop(); brutos.push({ t: 'DEDENT', line: nl, col: 1, o: '' }); }
        if (recuo !== pilha[pilha.length - 1])
          falha('léxico', 'Este recuo não bate com nenhum nível anterior.', { line: nl, col: 1 }, 'Use sempre o mesmo número de espaços por nível.');
      }
    }
    brutos.push(...lt);
    ultimo = lt[lt.length - 1];
    if (ultimo.t === 'p' && '.:;'.includes(ultimo.v)) { brutos.push({ t: 'NL', line: nl, col: ultimo.col + 1, o: '' }); cont = false; }
    else cont = true;
  }
  if (cont) falha('sintaxe', 'O texto termina no meio de um período: falta o ponto final.', ultimo);
  const fim = ultimo ? ultimo.line + 1 : 1;
  while (pilha.length > 1) { pilha.pop(); brutos.push({ t: 'DEDENT', line: fim, col: 1, o: '' }); }
  brutos.push({ t: 'EOF', line: fim, col: 1, o: '' });
  return fundir(desfazerContracoes(fundir(brutos, LOC_PROTEGIDAS)), LOCUCOES);
}

// ============================================================ ANÁLISE
const LVALUES = new Set(['var', 'campo', 'indice', 'apontado']);
const refNome = d => d.simbolo ? d.nome : `${artDef(d.g, d.n)} ${d.nome}`;
const cap = s => s ? s[0].toUpperCase() + s.slice(1) : s;
function descTok(tk) {
  switch (tk.t) {
    case 'EOF': return 'o fim do texto';
    case 'NL': return 'o fim da linha';
    case 'INDENT': return 'um recuo';
    case 'DEDENT': return 'o fim do bloco';
    case 'n': return `o número ${tk.o}`;
    case 's': return `o texto ${tk.o}`;
    case 'c': return `o caractere ${tk.o}`;
    case 'p': return `"${tk.v}"`;
    default: return `"${tk.parte ? tk.contr : tk.o}"`;
  }
}

class Analisador {
  constructor(toks) {
    this.t = toks; this.i = 0; this.nid = 0;
    this.registros = new Map(); this.registrosPl = new Map();
    this.funcs = []; this.porForma = new Map(); this.porInf = new Map();
    this.predSubj = new Map(); this.predInf = new Map(); this.predAdj = new Map();
    this.funcInicio = new Map(); this.regInicio = new Map(); this.campos = new Set();
    this.escopo = null; this.funcAtual = null; this.laco = 0; this.res = null; this.ultimaOracao = null;
  }

  // ---------- navegação
  peek(k = 0) { return this.t[Math.min(this.i + k, this.t.length - 1)]; }
  next() { const tk = this.t[this.i]; if (this.i < this.t.length - 1) this.i++; return tk; }
  isW(v, k = 0) { const tk = this.peek(k); return tk.t === 'w' && tk.v === v; }
  isP(v, k = 0) { const tk = this.peek(k); return tk.t === 'p' && tk.v === v; }
  accW(v) { return this.isW(v) ? this.next() : null; }
  expW(v, ctx) { if (this.isW(v)) return this.next(); this.esperado(`"${v}"`, ctx); }
  expP(v, ctx) {
    if (this.isP(v)) return this.anotar(this.next(), 'pontuação');
    this.esperado(v === '.' ? 'o ponto final' : v === ':' ? 'dois-pontos' : `"${v}"`, ctx);
  }
  anotar(tk, cl, info) { if (tk && tk.t !== 'EOF') tk.ana = Object.assign({ cl }, info || {}); return tk; }
  esperado(oque, ctx) {
    const tk = this.peek(), ant = this.t[this.i - 1];
    if (tk.bol && ant && ['w', 'n', 's', 'c', 'p'].includes(ant.t) && !(ant.t === 'p' && '.:;'.includes(ant.v)) && ant.line < tk.line)
      falha('sintaxe', `Faltou o ponto final no fim da linha ${ant.line}.`, ant, 'Todo período termina em ponto; blocos abrem com dois-pontos.');
    falha('sintaxe', `Esperava ${oque}${ctx ? ' ' + ctx : ''}, mas encontrei ${descTok(tk)}.`, tk, this.dicaAcento(tk));
  }
  dicaAcento(tk) {
    if (tk.t !== 'w') return null;
    const s = semAcento(tk.v);
    for (const kw of PALAVRAS_ACENTO) if (semAcento(kw) === s && kw !== tk.v) return `Faltou acento? Escreva "${kw}".`;
    return null;
  }
  expPrep(p, ctx) {
    const tk = this.peek();
    if (!(tk.t === 'w' && tk.v === p)) {
      if (tk.t === 'w' && PREPOSICOES.has(tk.v))
        falha('regência', `Esperava a preposição "${p}"${ctx ? ' ' + ctx : ''}, mas veio "${tk.parte ? tk.contr : tk.o}".`, tk);
      this.esperado(`a preposição "${p}"`, ctx);
    }
    this.next();
    this.anotar(tk, 'preposição', tk.contr ? { contr: tk.contr } : {});
    this.checaContracao(tk);
    return tk;
  }
  checaContracao(prepTk) {
    const p = prepTk.v.split(' ').pop();
    if (!['de', 'em', 'a', 'por'].includes(p) || prepTk.contr) return;
    const at = this.peek();
    if (at.t === 'w' && ARTIGOS[at.v] && ARTIGOS[at.v].def && !at.contr) {
      const certo = contrair(p, at.v);
      falha('morfologia', `Contração obrigatória: "${p} ${at.o}" se escreve "${certo}".`, prepTk,
        `Preposição seguida de artigo definido se contrai, como no português escrito: de + o = do, em + a = na, a + o = ao, por + a = pela.`);
    }
  }

  // ---------- escopo
  abreEscopo(tipo) { this.escopo = { vars: new Map(), pai: this.escopo, tipo }; }
  fechaEscopo() { this.escopo = this.escopo.pai; }
  buscar(nome) { for (let e = this.escopo; e; e = e.pai) if (e.vars.has(nome)) return e.vars.get(nome); return null; }
  declarar(info) {
    const ex = this.escopo.vars.get(info.nome);
    if (ex) falha('escopo', `${cap(refNome(ex))} já foi declarad${ex.g === 'f' ? 'a' : 'o'} neste bloco (linha ${ex.tok.line}).`, info.tok);
    const d = Object.assign({ id: ++this.nid, func: this.funcAtual }, info);
    this.escopo.vars.set(info.nome, d);
    return d;
  }
  concordaArtigo(at, g, n, w) {
    const a = ARTIGOS[at.v];
    if (a.g === g && a.n === n) return;
    const certo = artDef(g, n);
    let escrito, sug;
    if (at.parte === 2) { const prep = CONTRACOES[at.contr][0]; escrito = `${at.contr} ${w.o}`; sug = `${contrair(prep, certo)} ${w.o}`; }
    else { escrito = `${at.o} ${w.o}`; sug = `${certo} ${w.o}`; }
    falha('concordância', `"${escrito}" não concorda: "${w.o}" é ${descGN(g, n)}. Escreva "${sug}".`, at);
  }
  ehVerbo(tk) {
    if (tk.t !== 'w') return false;
    if (NUCLEO[tk.v] && tk.v !== 'repito') return true;
    return this.porForma.has(tk.v);
  }
  podeIniciarValor(tk) {
    if (['n', 's', 'c'].includes(tk.t)) return true;
    if (tk.t === 'p') return tk.v === '(';
    if (tk.t !== 'w') return false;
    return (ARTIGOS[tk.v] && ARTIGOS[tk.v].def) || ehSimbolo(tk.v) || ['verdadeiro', 'falso', 'nada'].includes(tk.v);
  }

  // ============================================================ PROGRAMA
  programa() {
    this.preVarredura();
    this.abreEscopo('global');
    const corpo = [];
    while (this.peek().t !== 'EOF') {
      const tk = this.peek();
      if (tk.t === 'NL') { this.next(); continue; }
      if (tk.t === 'INDENT') falha('sintaxe', 'Recuo inesperado: esta linha não pertence a nenhum bloco.', tk, 'Só se recua depois de uma linha terminada em dois-pontos.');
      if (tk.t === 'DEDENT') { this.next(); continue; }
      if (this.regInicio.has(this.i)) { this.i = this.regInicio.get(this.i); continue; }
      if (this.funcInicio.has(this.i)) { this.corpoFuncao(this.funcInicio.get(this.i)); continue; }
      corpo.push(this.periodo());
    }
    const comecar = this.porInf.get('começar') || null;
    if (comecar && (comecar.params.length || comecar.retorno))
      falha('sintaxe', '"Para começar" é a entrada do programa e não recebe parâmetros nem devolve valor.', comecar.tok);
    return { k: 'texto', corpo, funcs: this.funcs, registros: [...this.registros.values()], comecar };
  }

  // Encontra registros e cabeçalhos de funções antes da análise principal,
  // para que chamadas possam aparecer antes da definição.
  preVarredura() {
    const inicios = [];
    let prof = 0, inicioPeriodo = true;
    for (let k = 0; k < this.t.length; k++) {
      const tk = this.t[k];
      if (tk.t === 'INDENT') { prof++; continue; }
      if (tk.t === 'DEDENT') { prof--; continue; }
      if (tk.t === 'NL') { inicioPeriodo = true; continue; }
      if (inicioPeriodo && prof === 0) inicios.push(k);
      inicioPeriodo = false;
    }
    const regs = [], funcs = [];
    for (const k of inicios) {
      const a = this.t[k], b = this.t[k + 1], c = this.t[k + 2];
      if (a.t === 'w' && ARTIGOS[a.v] && !ARTIGOS[a.v].def && b.t === 'w' && c.t === 'w' && c.v === 'é') regs.push(k);
      else if (a.t === 'w' && a.v === 'para' && !(b.t === 'w' && b.v === 'cada')) funcs.push(k);
    }
    // 1) nomes dos registros
    for (const k of regs) {
      const at = this.t[k], w = this.t[k + 1];
      if (RESERVADOS.has(w.v) || ehSimbolo(w.v)) falha('morfologia', `"${w.o}" não pode ser nome de tipo.`, w);
      if (this.registros.has(w.v)) falha('escopo', `O tipo "${w.o}" já foi definido.`, w);
      const r = { nome: w.v, g: ARTIGOS[at.v].g, plural: pluralizar(w.v), campos: [], tamanho: null, tok: w, inicio: k };
      if (ARTIGOS[at.v].n !== 's') falha('concordância', 'Um tipo de registro se define no singular: "Um nó é um registro de:".', at);
      this.anotar(at, 'artigo', { def: false, g: r.g, n: 's' });
      this.anotar(w, 'nome', { g: r.g, n: 's', papel: 'tipo de registro' });
      this.registros.set(r.nome, r);
      this.registrosPl.set(r.plural, r);
    }
    // 2) campos
    for (const r of this.registros.values()) {
      this.i = r.inicio + 2;
      this.anotar(this.expW('é'), 'verbo', { lema: 'ser', forma: 'presente do indicativo, 3ª p. sing. (definição de tipo)' });
      const um = this.next();
      if (!(um.t === 'w' && um.v === 'um') || !this.isW('registro')) { this.i--; this.esperado('"um registro de"', `na definição de "${r.nome}"`); }
      this.anotar(um, 'artigo', { def: false, g: 'm', n: 's' });
      this.anotar(this.next(), 'tipo', {});
      this.expPrep('de');
      this.expP(':', 'depois de "um registro de"');
      if (this.peek().t !== 'NL') this.esperado('o fim da linha', 'depois de "registro de:"');
      this.next();
      if (this.peek().t !== 'INDENT') falha('sintaxe', 'Os campos do registro vêm nas linhas de baixo, recuados.', this.peek());
      this.next();
      let off = 0;
      for (;;) {
        const tk = this.peek();
        let nome, g = 'm', n = 's', simbolo = false;
        if (tk.t === 'w' && ehSimbolo(tk.v)) { this.next(); nome = tk.v; simbolo = true; this.anotar(tk, 'símbolo', { papel: 'campo' }); }
        else if (tk.t === 'w' && ARTIGOS[tk.v] && ARTIGOS[tk.v].def) {
          this.next(); const w = this.next();
          if (w.t !== 'w') this.esperado('o nome do campo');
          if (RESERVADOS.has(w.v)) falha('morfologia', `"${w.o}" é palavra da linguagem e não pode nomear campo.`, w);
          nome = w.v; g = ARTIGOS[tk.v].g; n = ARTIGOS[tk.v].n;
          this.anotar(tk, 'artigo', ARTIGOS[tk.v]);
          this.anotar(w, 'nome', { g, n, papel: 'campo' });
        } else this.esperado('um campo ("o saldo, um real;")');
        if (r.campos.some(c => c.nome === nome)) falha('escopo', `O campo "${nome}" aparece duas vezes em "${r.nome}".`, tk);
        this.expP(',', 'entre o nome do campo e o seu tipo');
        const art = this.next();
        if (!(art.t === 'w' && ARTIGOS[art.v] && !ARTIGOS[art.v].def && ARTIGOS[art.v].n === 's')) { this.i--; this.esperado('o tipo com artigo indefinido ("um real")'); }
        const tipo = this.tipoSing(art);
        this.checaNumeroNome(n, tipo, tk, simbolo);
        r.campos.push({ nome, g, n, simbolo, tipo, tok: tk });
        this.campos.add(nome);
        if (this.isP(';')) { this.next(); if (this.peek().t === 'NL') this.next(); continue; }
        if (this.isP('.')) { this.next(); if (this.peek().t === 'NL') this.next(); break; }
        this.esperado('";" (há mais campos) ou "." (último campo)');
      }
      if (this.peek().t !== 'DEDENT') falha('sintaxe', 'O registro termina no campo com ponto final; nada mais vai nesse recuo.', this.peek());
      this.next();
      this.regInicio.set(r.inicio, this.i);
    }
    // tamanhos
    const calc = (r, pilha) => {
      if (r.tamanho !== null) return r.tamanho;
      if (pilha.includes(r)) falha('tipo', `O registro "${r.nome}" contém a si mesmo e teria tamanho infinito. Use "um endereço de ${r.nome}".`, r.tok);
      let off = 0;
      for (const c of r.campos) {
        c.off = off;
        const sz = t => t.k === 'seq' ? t.n * sz(t.of) : t.k === 'rec' ? calc(t.r, pilha.concat([r])) : 1;
        off += sz(c.tipo);
      }
      r.tamanho = off;
      return off;
    };
    for (const r of this.registros.values()) calc(r, []);
    // 3) cabeçalhos de funções
    for (const k of funcs) {
      this.i = k;
      const f = this.cabecalho();
      f.inicio = k; f.fimCab = this.i;
      this.funcInicio.set(k, f);
    }
    this.i = 0;
  }

  ehParamInicio(k) {
    const tk = this.peek(k);
    if (tk.t !== 'w') return false;
    if (ehSimbolo(tk.v)) return true;
    if (ARTIGOS[tk.v] && ARTIGOS[tk.v].def && tk.parte !== 1) {
      const w = this.peek(k + 1);
      return w.t === 'w' && !ehSimbolo(w.v) && !RESERVADOS.has(w.v);
    }
    return false;
  }
  paramDecl() {
    const tk = this.next();
    if (ehSimbolo(tk.v)) { this.anotar(tk, 'símbolo', { papel: 'parâmetro' }); return { nome: tk.v, simbolo: true, g: 'm', n: 's', tok: tk }; }
    const w = this.next();
    this.anotar(tk, 'artigo', ARTIGOS[tk.v]);
    this.anotar(w, 'nome', { g: ARTIGOS[tk.v].g, n: ARTIGOS[tk.v].n, papel: 'parâmetro' });
    return { nome: w.v, g: ARTIGOS[tk.v].g, n: ARTIGOS[tk.v].n, art: tk.v, tok: w };
  }
  cabecalho() {
    const para = this.expW('para');
    this.anotar(para, 'controle', { papel: 'definição de função' });
    let sujeito = null;
    if (this.ehParamInicio(0)) sujeito = this.paramDecl();
    const vt = this.next();
    if (vt.t !== 'w' || !/(ar|er|ir|or|ôr)$/.test(vt.v) || RESERVADOS.has(vt.v))
      falha('morfologia', `Depois de "Para" vem o verbo no infinitivo (terminado em -ar, -er ou -ir), mas encontrei ${descTok(vt)}.`, vt);
    // predicado adjetivo: "Para o número ser par"
    let adj = null;
    if (sujeito && vt.v === 'ser') {
      const at = this.peek();
      if (at.t === 'w' && !ARTIGOS[at.v] && !PREPOSICOES.has(at.v) && !ehSimbolo(at.v) && !RESERVADOS.has(at.v)) {
        this.next(); adj = at.v;
        this.anotar(vt, 'verbo', { lema: 'ser', forma: 'infinitivo (definição de predicado)', subjSempre: true, subj: 'for' });
        this.anotar(at, 'adjetivo', { g: sujeito.g, n: sujeito.n, papel: 'predicativo do sujeito (definição)' });
      }
    }
    if (INF_NUCLEO[vt.v] && !adj) falha('morfologia', `"${vt.o}" é verbo da própria linguagem; escolha outro verbo.`, vt,
      vt.v === 'ser' && sujeito ? 'Para um predicado com adjetivo: "Para o número ser par, sendo o número um inteiro, devolvendo um lógico:".' : null);
    let forma = adj ? null : conjugar1ps(vt.v), formaDeclarada = false;
    if (this.isP('(')) {
      this.next(); const ft = this.next();
      if (ft.t !== 'w') this.esperado('a forma da 1ª pessoa entre parênteses');
      forma = ft.v; formaDeclarada = true; this.expP(')');
    }
    if (forma && NUCLEO[forma]) falha('morfologia', `A 1ª pessoa de "${vt.v}" é "${forma}", que já é verbo da linguagem. Escolha outro verbo.`, vt);
    if (this.porInf.has(vt.v) && !sujeito) falha('escopo', `O verbo "${vt.v}" já foi definido (linha ${this.porInf.get(vt.v).tok.line}).`, vt);
    if (!adj) this.anotar(vt, 'verbo', sujeito
      ? { lema: vt.v, forma: 'infinitivo (definição de predicado)', subjSempre: true, subj: futuroSubj(vt.v) }
      : { lema: vt.v, forma: 'infinitivo (definição)', conj: forma, declarada: formaDeclarada });
    const objetos = [], preps = [];
    if (this.ehParamInicio(0)) {
      objetos.push(this.paramDecl());
      while ((this.isW('e') || this.isP(',')) && this.ehParamInicio(1)) { this.anotar(this.next(), 'conjunção'); objetos.push(this.paramDecl()); }
    }
    while (this.peek().t === 'w' && PREPOSICOES.has(this.peek().v) && this.ehParamInicio(1)) {
      const pt = this.expPrep(this.peek().v);
      pt.ana.papel = `regência de ${adj ? 'ser ' + adj : vt.v}`;
      preps.push([pt.v, this.paramDecl()]);
      while (this.isW('e') && this.ehParamInicio(1)) {
        this.anotar(this.next(), 'conjunção', { papel: `segundo termo de "${pt.v}"` });
        preps.push(['e', this.paramDecl()]);
      }
    }
    let params = [...(sujeito ? [sujeito] : []), ...objetos, ...preps.map(p => p[1])];
    const nomes = new Set();
    for (const p of params) { if (nomes.has(p.nome)) falha('escopo', `O parâmetro "${p.nome}" aparece duas vezes.`, p.tok); nomes.add(p.nome); }
    const candidatosFixos = new Set(objetos.filter(p => !p.simbolo));
    const obrigatorios = params.filter(p => !candidatosFixos.has(p));
    const temSendo = this.isP(',') && this.isW('sendo', 1);
    if (obrigatorios.length && !temSendo)
      falha('tipo', 'Faltam os tipos dos parâmetros. Depois deles vem ", sendo ...": "Para somar x e y, sendo ambos reais".', this.peek());
    if (temSendo) { this.next(); this.anotar(this.next(), 'gerúndio', { lema: 'ser', papel: 'tipos dos parâmetros' }); this.tipagens(params, candidatosFixos); }
    // objeto nominal sem tipo = complemento fixo do verbo ("medir a distância entre p e q")
    for (const p of params) {
      if (p.tipo) continue;
      if (candidatosFixos.has(p)) { p.fixo = true; p.tok.ana = { cl: 'nome', g: p.g, n: p.n, papel: 'complemento fixo do verbo' }; continue; }
      falha('tipo', `Falta o tipo do parâmetro "${p.nome}".`, p.tok);
    }
    params = params.filter(p => !p.fixo);
    let retorno = null;
    if (this.isP(',') && this.isW('devolvendo', 1)) {
      this.next(); this.anotar(this.next(), 'gerúndio', { lema: 'devolver', papel: 'tipo de retorno' });
      const at = this.next();
      if (!(at.t === 'w' && ARTIGOS[at.v] && !ARTIGOS[at.v].def)) { this.i--; this.esperado('o tipo devolvido com artigo indefinido ("um real")'); }
      retorno = this.tipoSing(at);
    }
    this.expP(':', 'no fim do cabeçalho da função');
    const f = { inf: adj ? 'ser ' + adj : vt.v, forma, sujeito, objetos, preps, params, retorno, tok: vt, corpo: null };
    if (adj) {
      if (!retorno || retorno.k !== 'bool')
        falha('tipo', `"ser ${adj}" é um predicado e precisa devolver um lógico (", devolvendo um lógico").`, vt);
      if (this.predAdj.has(adj)) falha('escopo', `O predicado "ser ${adj}" já foi definido.`, vt);
      f.adj = adj; f.adjG = sujeito.g; f.adjN = sujeito.n; f.subj = 'for ' + adj;
      this.predAdj.set(adj, f);
    } else if (sujeito) {
      if (!retorno || retorno.k !== 'bool')
        falha('tipo', `"${vt.v}" tem sujeito antes do verbo, então é um predicado e precisa devolver um lógico (", devolvendo um lógico").`, vt);
      f.subj = futuroSubj(vt.v);
      if (this.predInf.has(vt.v)) falha('escopo', `O predicado "${vt.v}" já foi definido.`, vt);
      this.predSubj.set(f.subj, f); this.predInf.set(vt.v, f);
    } else {
      this.porForma.set(forma, f); this.porInf.set(vt.v, f);
    }
    this.funcs.push(f);
    return f;
  }
  tipagens(params, fixos) {
    const tipados = new Set();
    for (;;) {
      let grupo = [];
      const t0 = this.peek();
      if (this.isW('ambos') || this.isW('ambas')) {
        this.next();
        let restantes = params.filter(p => !tipados.has(p) && !(fixos && fixos.has(p)));
        if (restantes.length !== 2) restantes = params.filter(p => !tipados.has(p));
        if (restantes.length !== 2) falha('concordância', `"${t0.o}" pede exatamente dois parâmetros sem tipo; aqui há ${restantes.length}.`, t0);
        const fem = restantes.every(p => p.g === 'f' && !p.simbolo);
        if (fem !== (t0.v === 'ambas')) falha('concordância', `"${t0.o}" não concorda com os parâmetros: use "${fem ? 'ambas' : 'ambos'}".`, t0);
        this.anotar(t0, 'pronome', { papel: 'retoma os dois parâmetros' });
        grupo = restantes;
      } else {
        grupo.push(this.paramRef(params));
        while ((this.isW('e') || this.isP(',')) && this.ehParamInicio(1) && !this.isPredicadoAdiante(1)) { this.anotar(this.next(), 'conjunção'); grupo.push(this.paramRef(params)); }
      }
      const tipo = this.predicadoTipo(grupo.length > 1, grupo, t0);
      for (const p of grupo) {
        if (tipados.has(p)) falha('tipo', `O tipo de "${p.nome}" foi dado duas vezes.`, t0);
        this.checaNumeroNome(p.n, tipo, p.tok, p.simbolo);
        p.tipo = tipo; tipados.add(p);
      }
      if ((this.isW('e') || this.isP(',')) && (this.ehParamInicio(1) || this.isW('ambos', 1) || this.isW('ambas', 1))) { this.anotar(this.next(), 'conjunção'); continue; }
      break;
    }
  }
  isPredicadoAdiante() { return false; }
  paramRef(params) {
    const tk = this.peek();
    let nome;
    if (ehSimbolo(tk.v)) { this.next(); nome = tk.v; this.anotar(tk, 'símbolo', { papel: 'parâmetro' }); }
    else if (ARTIGOS[tk.v] && ARTIGOS[tk.v].def) {
      this.next(); const w = this.next(); nome = w.v;
      const p = params.find(x => x.nome === nome);
      if (p) { this.concordaArtigo(tk, p.g, p.n, w); this.anotar(tk, 'artigo', ARTIGOS[tk.v]); this.anotar(w, 'nome', { g: p.g, n: p.n, papel: 'parâmetro' }); }
    } else this.esperado('um parâmetro');
    const p = params.find(x => x.nome === nome);
    if (!p) falha('escopo', `"${nome}" não é parâmetro desta função.`, tk);
    return p;
  }
  // tipo em posição de predicado: "um real" (um sujeito) ou "reais" (vários)
  predicadoTipo(plural, grupo, tk) {
    const at = this.peek();
    if (at.t === 'w' && ARTIGOS[at.v] && !ARTIGOS[at.v].def) {
      if (plural) falha('concordância', `Sujeito composto pede o tipo no plural, sem artigo: "${grupo.map(p => p.simbolo ? p.nome : artDef(p.g, p.n) + ' ' + p.nome).join(' e ')} ${this.pluralDeTipoAdiante()}".`, at);
      this.next();
      return this.tipoSing(at);
    }
    if (at.t === 'w' && (TIPO_PLUR[at.v] || this.registrosPl.has(at.v))) {
      if (!plural) falha('concordância', `Sujeito no singular pede artigo indefinido e tipo no singular: "${artIndef(TIPO_SING[TIPO_PLUR[at.v]] || 'm', 's')} ${TIPO_PLUR[at.v] || this.registrosPl.get(at.v).nome}".`, at);
      return this.tipoPlural();
    }
    if (at.t === 'w' && (TIPO_SING[at.v] || this.registros.has(at.v)))
      falha('morfologia', `Tipo sem artigo: escreva "${artIndef(TIPO_SING[at.v] || this.registros.get(at.v).g, 's')} ${at.o}".`, at);
    this.esperado('um tipo ("um inteiro", "um real", "reais"...)');
  }
  pluralDeTipoAdiante() {
    const w = this.peek(1);
    if (w.t !== 'w') return '...';
    if (TIPO_SING[w.v]) return pluralizar(w.v);
    if (this.registros.has(w.v)) return this.registros.get(w.v).plural;
    return pluralizar(w.v);
  }
  checaNumeroNome(n, tipo, tk, simbolo) {
    if (simbolo) return;
    if (n === 'p' && tipo.k !== 'seq')
      falha('concordância', `Nome no plural é coleção: "${tk.o}" precisa ser "uma sequência de ...". Para um valor só, use o singular.`, tk);
    if (n === 's' && tipo.k === 'seq')
      falha('concordância', `Sequência é coleção e pede nome no plural ("as notas", não "a nota").`, tk);
  }

  bits(ehInt) {
    if (this.isW('de') && this.peek(1).t === 'n' && this.isW('bits', 2)) {
      this.anotar(this.next(), 'preposição');
      const nt = this.next(); this.anotar(nt, 'número');
      this.anotar(this.next(), 'nome', { papel: 'unidade' });
      const b = Number(nt.val);
      const ok = ehInt ? [8, 16, 32, 64] : [32, 64];
      if (!ok.includes(b)) falha('tipo', `${ehInt ? 'Inteiros' : 'Reais'} têm ${ok.join(', ')} bits; ${b} não existe.`, nt);
      return b;
    }
    return 64;
  }
  tipoSing(artTk) {
    const w = this.next();
    if (w.t !== 'w') { this.i--; this.esperado('um tipo'); }
    let t;
    switch (w.v) {
      case 'inteiro': t = T.int(this.bits(true)); break;
      case 'natural': t = T.nat(this.bits(true)); break;
      case 'real': t = T.real(this.bits(false)); break;
      case 'byte': t = T.byte; break;
      case 'lógico': t = T.bool; break;
      case 'caractere': t = T.car; break;
      case 'texto': t = T.texto; break;
      case 'endereço': this.anotar(w, 'tipo', {}); this.expPrep('de', 'em "endereço de"'); t = T.ptr(this.tipoSing(null)); break;
      case 'sequência': {
        this.anotar(w, 'tipo', {});
        this.expPrep('de', 'em "sequência de N ..."');
        const nt = this.next();
        if (nt.t !== 'n' || nt.real || nt.val < 1n) { this.i--; this.esperado('a quantidade de itens (um inteiro positivo)', 'depois de "sequência de"'); }
        this.anotar(nt, 'número');
        t = T.seq(Number(nt.val), this.tipoPlural());
        break;
      }
      default:
        if (this.registros.has(w.v)) { t = T.rec(this.registros.get(w.v)); break; }
        if (TIPO_PLUR[w.v] || this.registrosPl.has(w.v)) falha('concordância', `Aqui o tipo vai no singular: "${TIPO_PLUR[w.v] || this.registrosPl.get(w.v).nome}".`, w);
        falha('tipo', `Tipo desconhecido: "${w.o}".`, w, this.dicaAcento(w) || 'Tipos: inteiro, natural, real, byte, lógico, caractere, texto, endereço de T, sequência de N Ts, ou um registro definido com "Um X é um registro de:".');
    }
    if (!w.ana) this.anotar(w, 'tipo', { g: generoTipo(t) });
    if (artTk) {
      const a = ARTIGOS[artTk.v], g = generoTipo(t);
      if (a.g !== g || a.n !== 's')
        falha('concordância', `"${artTk.o} ${w.o}" não concorda: "${w.o}" é ${descGN(g, 's')}. Escreva "${artIndef(g, 's')} ${w.o}".`, artTk);
      this.anotar(artTk, 'artigo', a);
    }
    return t;
  }
  tipoPlural() {
    const w = this.next();
    if (w.t !== 'w') { this.i--; this.esperado('um tipo no plural'); }
    let t;
    switch (w.v) {
      case 'inteiros': t = T.int(this.bits(true)); break;
      case 'naturais': t = T.nat(this.bits(true)); break;
      case 'reais': t = T.real(this.bits(false)); break;
      case 'bytes': t = T.byte; break;
      case 'lógicos': t = T.bool; break;
      case 'caracteres': t = T.car; break;
      case 'textos': t = T.texto; break;
      case 'endereços': this.anotar(w, 'tipo', { n: 'p' }); this.expPrep('de'); t = T.ptr(this.tipoSing(null)); break;
      case 'sequências': {
        this.anotar(w, 'tipo', { n: 'p' }); this.expPrep('de');
        const nt = this.next();
        if (nt.t !== 'n' || nt.real || nt.val < 1n) { this.i--; this.esperado('a quantidade de itens'); }
        t = T.seq(Number(nt.val), this.tipoPlural());
        break;
      }
      default:
        if (this.registrosPl.has(w.v)) { t = T.rec(this.registrosPl.get(w.v)); break; }
        if (TIPO_SING[w.v] || this.registros.has(w.v))
          falha('concordância', `Aqui o tipo vai no plural: "${TIPO_SING[w.v] ? pluralizar(w.v) : this.registros.get(w.v).plural}".`, w);
        falha('tipo', `Tipo desconhecido: "${w.o}".`, w, this.dicaAcento(w));
    }
    if (!w.ana) this.anotar(w, 'tipo', { n: 'p' });
    return t;
  }

  corpoFuncao(f) {
    this.i = f.fimCab;
    this.funcAtual = f;
    this.abreEscopo('função');
    for (const p of f.params) p.d = this.declarar({ nome: p.nome, g: p.g, n: p.n, simbolo: p.simbolo, tipo: p.tipo, tok: p.tok, papel: 'parâmetro' });
    f.corpo = this.corpo();
    this.fechaEscopo();
    this.funcAtual = null;
  }

  // ============================================================ PERÍODOS
  periodo() {
    const tk = this.peek();
    if (tk.t === 'INDENT') falha('sintaxe', 'Recuo inesperado: esta linha não abre nem continua nenhum bloco.', tk, 'Só se recua depois de uma linha terminada em dois-pontos.');
    if (tk.t !== 'w') this.esperado('um verbo no começo do período');
    switch (tk.v) {
      case 'se': return this.blocoSe();
      case 'enquanto': return this.blocoEnquanto();
      case 'repito': return this.blocoRepito();
      case 'senão': falha('sintaxe', '"Senão" sem um "Se" logo antes, no mesmo recuo.', tk);
      case 'para':
        if (this.isW('cada', 1)) return this.blocoCada();
        falha('sintaxe', 'Funções ("Para ...:") só se definem fora de qualquer bloco, sem recuo.', tk);
    }
    return this.periodoSimples();
  }
  periodoSimples() {
    const p = { k: 'periodo', oracoes: [], tok: this.peek() };
    const salvoRes = this.res; this.res = null;
    for (;;) {
      const o = this.oracao();
      if (o.k === 'leitura' && this.ultimaOracao && this.ultimaOracao.k === 'escrita' && p.oracoes.includes(this.ultimaOracao)) this.ultimaOracao.prompt = true;
      p.oracoes.push(o);
      this.ultimaOracao = o;
      if (this.isP('.')) { this.anotar(this.next(), 'pontuação', { papel: 'fim do período' }); break; }
      let j = -1;
      if (this.isW('e')) j = 1; else if (this.isP(',')) j = this.isW('e', 1) ? 2 : 1;
      if (j > 0 && this.ehVerbo(this.peek(j))) {
        for (let k = 0; k < j; k++) { const c = this.next(); this.anotar(c, c.t === 'p' ? 'pontuação' : 'conjunção', { papel: 'coordena orações' }); }
        continue;
      }
      this.esperado('o ponto final', 'para fechar o período (ou "e" seguido de outro verbo)');
    }
    if (this.peek().t === 'NL') this.next();
    this.res = salvoRes;
    return p;
  }
  corpo() {
    if (this.peek().t === 'NL') {
      this.next();
      if (this.peek().t !== 'INDENT') falha('sintaxe', 'Depois de dois-pontos no fim da linha, o bloco vem na linha de baixo, recuado.', this.peek());
      this.next();
      this.abreEscopo('bloco');
      const ps = [];
      while (this.peek().t !== 'DEDENT' && this.peek().t !== 'EOF') {
        if (this.peek().t === 'NL') { this.next(); continue; }
        ps.push(this.periodo());
      }
      if (this.peek().t === 'DEDENT') this.next();
      this.fechaEscopo();
      return ps;
    }
    this.abreEscopo('bloco');
    const p = this.periodo();
    this.fechaEscopo();
    return [p];
  }
  blocoSe() {
    const tk = this.next();
    this.anotar(tk, 'controle', { papel: 'condicional' });
    const ramos = [];
    const c = this.condicao();
    this.expP(':', 'depois da condição');
    ramos.push({ cond: c, corpo: this.corpo() });
    let senao = null;
    while (this.isW('senão')) {
      const st = this.next();
      this.anotar(st, 'controle', { papel: 'alternativa' });
      if (this.isP(',') && this.isW('se', 1)) {
        this.anotar(this.next(), 'pontuação'); this.anotar(this.next(), 'controle', { papel: 'condicional' });
        const c2 = this.condicao(); this.expP(':', 'depois da condição');
        ramos.push({ cond: c2, corpo: this.corpo() });
        continue;
      }
      if (this.isW('se')) falha('sintaxe', 'Entre "senão" e "se" vai vírgula: "Senão, se ...:".', this.peek());
      this.expP(':', 'depois de "Senão"');
      senao = this.corpo();
      break;
    }
    return { k: 'se', ramos, senao, tok: tk };
  }
  blocoEnquanto() {
    const tk = this.next();
    this.anotar(tk, 'controle', { papel: 'laço condicional' });
    const c = this.condicao();
    this.expP(':', 'depois da condição');
    this.laco++; const corpo = this.corpo(); this.laco--;
    return { k: 'enquanto', cond: c, corpo, tok: tk };
  }
  blocoRepito() {
    const tk = this.next();
    this.anotar(tk, 'verbo', { lema: 'repetir', forma: 'presente do indicativo, 1ª p. sing.' });
    let vezes = null;
    if (this.isW('sempre')) this.anotar(this.next(), 'advérbio');
    else {
      vezes = this.expr();
      if (vezes.tipo.k !== 'int') falha('tipo', `O número de repetições precisa ser inteiro, não ${descTipo(vezes.tipo)}.`, tk);
      if (!this.isW('vezes')) this.esperado('"vezes"', 'depois da quantidade');
      this.anotar(this.next(), 'nome', { papel: 'contagem' });
    }
    this.expP(':', vezes ? 'depois de "vezes"' : 'depois de "Repito sempre"');
    this.laco++; const corpo = this.corpo(); this.laco--;
    return { k: 'repete', vezes, corpo, tok: tk };
  }
  blocoCada() {
    const tk = this.next(); this.anotar(tk, 'controle', { papel: 'laço' });
    const cada = this.next(); this.anotar(cada, 'determinante', { papel: 'distributivo' });
    const v = this.next();
    if (v.t !== 'w') { this.i--; this.esperado('o nome do item ou um símbolo', 'depois de "cada"'); }
    if (ehSimbolo(v.v)) {
      this.anotar(v, 'símbolo', { papel: 'contador' });
      this.expPrep('de', `em "Para cada ${v.v} de ... até ..."`);
      const de = this.expr();
      this.anotar(this.expW('até', 'para fechar o intervalo'), 'preposição');
      const ate = this.expr();
      if (de.tipo.k !== 'int' || ate.tipo.k !== 'int') falha('tipo', 'Os limites do intervalo precisam ser inteiros.', v);
      this.expP(':', 'depois do intervalo');
      this.abreEscopo('bloco');
      const d = this.declarar({ nome: v.v, simbolo: true, g: 'm', n: 's', tipo: T.int(64), tok: v, papel: 'contador' });
      this.laco++; const corpo = this.corpo(); this.laco--;
      this.fechaEscopo();
      return { k: 'faixa', d, de, ate, corpo, tok: tk };
    }
    if (RESERVADOS.has(v.v)) falha('morfologia', `"${v.o}" é palavra da linguagem e não pode nomear o item do laço.`, v);
    let col;
    if (this.isW('de')) {
      this.expPrep('de', 'antes da coleção');
      const ct = this.peek();
      col = this.primario();
      if (!LVALUES.has(col.k)) falha('sintaxe', 'A coleção do laço precisa ser uma sequência guardada na memória.', ct);
    } else {
      const pl = pluralizar(v.v);
      const d = this.buscar(pl);
      if (!d) falha('escopo', `"Para cada ${v.o}" procura a coleção "${artDef(v.v.endsWith('a') ? 'f' : 'm', 'p')} ${pl}" (plural de "${v.o}"), que não foi declarada.`, v,
        `O compilador forma o plural pelas regras do português. Se a coleção tem outro nome, diga qual: "Para cada ${v.o} das ...:".`);
      col = { k: 'var', d, tipo: d.tipo, tok: v, art: artDef(d.g, d.n) };
      v.ana = { cl: 'nome', g: d.g, n: 's', papel: 'item do laço', plural: pl };
    }
    if (col.tipo.k !== 'seq') falha('tipo', `"Para cada" percorre sequências; isto é ${descTipo(col.tipo)}.`, v);
    const g = col.d ? col.d.g : (col.art ? ARTIGOS[col.art].g : (v.v.endsWith('a') ? 'f' : 'm'));
    if (!v.ana) this.anotar(v, 'nome', { g, n: 's', papel: 'item do laço' });
    this.expP(':', 'depois da coleção');
    this.abreEscopo('bloco');
    const d = this.declarar({ nome: v.v, g, n: 's', tipo: col.tipo.of, tok: v, alias: true, papel: 'item do laço' });
    this.laco++; const corpo = this.corpo(); this.laco--;
    this.fechaEscopo();
    return { k: 'cada', d, col, corpo, tok: tk };
  }

  // ============================================================ ORAÇÕES
  oracao() {
    const tk = this.peek();
    if (tk.t !== 'w') this.esperado('um verbo no começo da oração');
    const v = tk.v;
    if (NUCLEO[v] && v !== 'repito') {
      this.next();
      this.anotar(tk, 'verbo', { lema: NUCLEO[v], forma: v.startsWith('seja') ? `presente do subjuntivo, 3ª p. ${v === 'sejam' ? 'pl.' : 'sing.'} (declaração)` : 'presente do indicativo, 1ª p. sing.' });
      switch (v) {
        case 'seja': case 'sejam': return this.oSeja(tk);
        case 'guardo': return this.oGuardo(tk);
        case 'acrescento': return this.oAjuste(tk, '+', 'a', false);
        case 'subtraio': return this.oAjuste(tk, '-', 'de', false);
        case 'multiplico': return this.oAjuste(tk, '*', 'por', true);
        case 'divido': return this.oAjuste(tk, '/', 'por', true);
        case 'escrevo': return this.oEscrevo(tk);
        case 'leio': return this.oLeio(tk);
        case 'devolvo': return this.oDevolvo(tk);
        case 'reservo': return this.oReservo(tk);
        case 'libero': return this.oLibero(tk);
        case 'paro': case 'pulo':
          if (!this.laco) falha('sintaxe', `"${tk.o}" só faz sentido dentro de um laço.`, tk);
          this.res = null;
          return { k: v, tok: tk };
      }
    }
    if (this.porForma.has(v)) return this.oChamada(tk);
    this.diagnosticoVerbo(tk);
  }
  diagnosticoVerbo(tk) {
    const v = tk.v;
    if (INF_NUCLEO[v]) falha('morfologia', `Na execução o verbo vai na 1ª pessoa do presente: "${INF_NUCLEO[v]}", não "${tk.o}".`, tk);
    if (this.porInf.has(v)) {
      const f = this.porInf.get(v);
      falha('morfologia', `"${tk.o}" é o infinitivo. Para executar, conjugue: "${f.forma}".`, tk, `O infinitivo aparece na definição ("Para ${v} ...") e como valor ("o resultado de ${v} ...").`);
    }
    for (const [forma, lema] of Object.entries(NUCLEO)) if (terceiraPessoa(lema) === v)
      falha('morfologia', `"${tk.o}" está na 3ª pessoa. Lavra narra em 1ª pessoa: "${forma}".`, tk);
    for (const f of this.porForma.values()) if (terceiraPessoa(f.inf) === v)
      falha('morfologia', `"${tk.o}" está na 3ª pessoa. Lavra narra em 1ª pessoa: "${f.forma}".`, tk);
    if ((ARTIGOS[v] && ARTIGOS[v].def) || ehSimbolo(v))
      falha('sintaxe', 'Toda oração começa pelo verbo. Para mudar um valor: "Guardo X em ALVO."', tk);
    if (['se', 'enquanto', 'para', 'senão'].includes(v)) falha('sintaxe', `"${tk.o}" abre bloco e só pode iniciar um período.`, tk);
    if (v === 'repito') falha('sintaxe', '"Repito" abre bloco e só pode iniciar um período.', tk);
    if (/(ar|er|ir)$/.test(v)) falha('escopo', `O verbo "${tk.o}" não foi definido. Defina-o com "Para ${v} ...:".`, tk);
    const forms = [...this.porForma.keys()];
    const parecido = forms.find(f => semAcento(f) === semAcento(v));
    falha('sintaxe', `"${tk.o}" não é um verbo conhecido.`, tk, parecido ? `Quis dizer "${parecido}"?` : this.dicaAcento(tk) || 'Orações começam por um verbo na 1ª pessoa: guardo, escrevo, leio, acrescento...');
  }

  oSeja(vt) {
    const sujeitos = [];
    const inicioSujeito = () => this.ehParamInicio(0);
    const a0 = this.peek(), w0 = this.peek(1);
    if (a0.t === 'w' && ARTIGOS[a0.v] && ARTIGOS[a0.v].def && w0.t === 'w') {
      if (ehSimbolo(w0.v)) falha('morfologia', `Símbolos não levam artigo: "Seja ${w0.v} ...".`, a0);
      if (RESERVADOS.has(w0.v)) falha('morfologia', `"${w0.o}" é palavra da linguagem e não pode ser nome de variável.`, w0,
        OPS_BIN[w0.v] || OPS_UN[w0.v] ? `"${w0.o}" é uma operação ("${artDef((OPS_BIN[w0.v] || OPS_UN[w0.v]).g, 's')} ${w0.o} de ..."). Escolha outro nome, como "o total".` : null);
    }
    if (a0.t === 'w' && !ARTIGOS[a0.v] && !ehSimbolo(a0.v) && /^[a-zà-ÿ]/.test(a0.v) && a0.v.length > 1)
      falha('morfologia', `"${a0.o}" tem mais de uma letra, então é nome e leva artigo: "Seja o ${a0.o} ..." ou "Seja a ${a0.o} ...".`, a0,
        'Só símbolos (uma letra, com dígitos opcionais: x, y, n, x1) vão sem artigo.');
    if (a0.t === 'w' && ARTIGOS[a0.v] && !ARTIGOS[a0.v].def) falha('morfologia', `Quem se declara já leva artigo definido ("Seja a prova um real"); o indefinido vai no tipo.`, a0);
    if (!inicioSujeito()) this.esperado('o nome a declarar ("a prova", "o contador" ou um símbolo como x)', `depois de "${vt.o}"`);
    sujeitos.push(this.sujeitoDecl());
    for (;;) {
      if (this.isP(',') && this.ehParamInicio(1)) { this.anotar(this.next(), 'pontuação'); sujeitos.push(this.sujeitoDecl()); continue; }
      if (this.isW('e') && this.ehParamInicio(1)) { this.anotar(this.next(), 'conjunção'); sujeitos.push(this.sujeitoDecl()); continue; }
      break;
    }
    const plural = sujeitos.length > 1 || sujeitos.some(s => s.n === 'p');
    if (plural && vt.v === 'seja') falha('concordância', `Sujeito ${sujeitos.length > 1 ? 'composto' : 'no plural'} pede o verbo no plural: "Sejam".`, vt);
    if (!plural && vt.v === 'sejam') falha('concordância', 'Sujeito no singular pede o verbo no singular: "Seja".', vt);
    let fixo = false;
    if (this.isP(',') && /^fix[oa]s?$/.test(this.peek(1).v || '')) {
      this.anotar(this.next(), 'pontuação');
      const ft = this.next();
      const g = sujeitos.every(s => s.g === 'f' && !s.simbolo) ? 'f' : 'm';
      const n = sujeitos.length > 1 || sujeitos[0].n === 'p' ? 'p' : 's';
      const certo = 'fix' + (g === 'f' ? 'a' : 'o') + (n === 'p' ? 's' : '');
      if (ft.v !== certo) falha('concordância', `O adjetivo concorda com o sujeito: "${certo}", não "${ft.o}".`, ft);
      this.anotar(ft, 'adjetivo', { g, n, papel: 'constante' });
      this.expP(',', 'depois do adjetivo');
      fixo = true;
    }
    const tk0 = this.peek();
    const tipo = this.predicadoTipo(sujeitos.length > 1, sujeitos, tk0);
    for (const s of sujeitos) this.checaNumeroNome(s.n, tipo, s.tok, s.simbolo);
    let valor = null, lista = null;
    if (this.isP(',') && this.isW('valendo', 1)) {
      this.anotar(this.next(), 'pontuação');
      this.anotar(this.next(), 'gerúndio', { lema: 'valer', papel: 'valor inicial' });
      const primeiro = this.expr();
      if (tipo.k === 'seq' && (this.isP(',') || this.isW('e')) && this.podeIniciarValor(this.peek(1)) && !this.ehVerbo(this.peek(1))) {
        lista = [primeiro];
        while (this.isP(',') && this.podeIniciarValor(this.peek(1))) { this.next(); lista.push(this.expr()); }
        if (this.isW('e') && this.podeIniciarValor(this.peek(1))) { this.anotar(this.next(), 'conjunção'); lista.push(this.expr()); }
        if (lista.length !== tipo.n) falha('tipo', `A sequência tem ${tipo.n} itens, mas vieram ${lista.length} valores.`, tk0);
        for (const e of lista) { const m = compativel(tipo.of, e.tipo); if (m) falha('tipo', `Valor inicial incompatível: ${m}`, e.tok || tk0); }
      } else {
        valor = primeiro;
        const alvoT = tipo.k === 'seq' && !(valor.tipo.k === 'seq') ? tipo.of : tipo;
        const m = compativel(alvoT, valor.tipo);
        if (m) falha('tipo', `Valor inicial incompatível: ${m}`, tk0);
      }
    }
    if (fixo && !valor && !lista) falha('sintaxe', 'Constante fixa precisa de valor: ", valendo ...".', vt);
    const ds = sujeitos.map(s => this.declarar({ nome: s.nome, g: s.g, n: s.n, simbolo: s.simbolo, tipo, fixo, tok: s.tok, papel: 'variável' }));
    for (const s of sujeitos) if (s.tok.ana) s.tok.ana.papel = fixo ? 'constante' : 'variável';
    this.res = null;
    return { k: 'decl', ds, tipo, valor, lista, tok: vt };
  }
  sujeitoDecl() {
    const tk = this.next();
    if (ehSimbolo(tk.v)) { this.anotar(tk, 'símbolo', { papel: 'variável' }); return { nome: tk.v, simbolo: true, g: 'm', n: 's', tok: tk }; }
    const w = this.next();
    if (RESERVADOS.has(w.v)) falha('morfologia', `"${w.o}" é palavra da linguagem e não pode ser nome de variável.`, w, OPS_BIN[w.v] || OPS_UN[w.v] ? `"${w.o}" é uma operação ("${artDef(OPS_BIN[w.v] ? OPS_BIN[w.v].g : OPS_UN[w.v].g, 's')} ${w.o} de ..."). Escolha outro nome, como "o total".` : null);
    if (analisarOrdinal(w.v)) falha('morfologia', `"${w.o}" é numeral ordinal e não pode ser nome.`, w);
    if (w.v.length < 2) falha('morfologia', 'Nomes têm pelo menos duas letras; uma letra só é símbolo e vai sem artigo.', w);
    const a = ARTIGOS[tk.v];
    this.anotar(tk, 'artigo', a);
    this.anotar(w, 'nome', { g: a.g, n: a.n, papel: 'variável' });
    return { nome: w.v, g: a.g, n: a.n, art: tk.v, tok: w };
  }
  oGuardo(vt) {
    const e = this.expr();
    this.expPrep('em', 'antes do lugar onde guardar ("em ALVO")');
    const at = this.peek();
    const alvo = this.alvo('depois de "em"');
    this.gravavel(alvo, at);
    const m = compativel(alvo.tipo, e.tipo);
    if (m) falha('tipo', `Não dá para guardar isso ${this.comPrep('em', alvo)}: ${m}`, at);
    this.res = alvo.tipo;
    return { k: 'guarda', expr: e, alvo, tok: vt };
  }
  oAjuste(vt, op, prep, alvoPrimeiro) {
    let alvo, e, at;
    if (alvoPrimeiro) {
      at = this.peek(); alvo = this.alvo(); this.expPrep(prep, `em "${vt.o} ALVO ${prep} X"`); e = this.expr();
    } else {
      e = this.expr(); this.expPrep(prep, `em "${vt.o} X ${prep === 'a' ? 'ao/à' : prep} ALVO"`); at = this.peek(); alvo = this.alvo();
    }
    this.gravavel(alvo, at);
    const tipoOp = this.tipoBin(op, alvo.tipo, e.tipo, vt);
    const m = compativel(alvo.tipo, tipoOp);
    if (m) falha('tipo', `O resultado não cabe ${this.comPrep('em', alvo)}: ${m}`, vt);
    this.res = alvo.tipo;
    return { k: 'ajuste', op, alvo, expr: e, tipoOp, tok: vt };
  }
  oEscrevo(vt) {
    const partes = [this.expr()];
    while (this.isW('seguido de') || this.isW('seguida de')) {
      const s = this.next(); this.anotar(s, 'locução', { papel: 'concatenação na saída' }); this.checaContracao(s);
      partes.push(this.expr());
    }
    for (const p of partes) if (p.tipo.k === 'void') falha('tipo', 'Isso não produz valor para escrever.', vt);
    this.res = null;
    return { k: 'escrita', partes, tok: vt };
  }
  oLeio(vt) {
    const at = this.peek();
    const LEGIVEL = t => ['int', 'real', 'car', 'texto', 'bool'].includes(t.k);
    if (at.t === 'w' && ARTIGOS[at.v] && !ARTIGOS[at.v].def) {
      this.next();
      const tipo = this.tipoSing(at);
      if (!LEGIVEL(tipo)) falha('tipo', `Não se lê ${descTipo(tipo)} do teclado.`, at);
      this.res = tipo;
      return { k: 'leitura', tipo, tok: vt };
    }
    const alvo = this.alvo('depois de "leio"');
    this.gravavel(alvo, at);
    if (!LEGIVEL(alvo.tipo)) falha('tipo', `Não se lê ${descTipo(alvo.tipo)} do teclado.`, at);
    this.res = alvo.tipo;
    return { k: 'leitura', alvo, tipo: alvo.tipo, tok: vt };
  }
  oDevolvo(vt) {
    const f = this.funcAtual;
    if (!f) falha('sintaxe', '"Devolvo" só existe dentro de uma função.', vt);
    const fimOracao = this.isP('.') || ((this.isW('e') || this.isP(',')) && this.ehVerbo(this.peek(1)));
    if (fimOracao) {
      if (f.retorno) falha('tipo', `"${f.inf}" promete devolver ${artIndef(generoTipo(f.retorno), 's')} ${descTipo(f.retorno)}; diga o quê: "Devolvo X."`, vt);
      return { k: 'retorno', expr: null, tok: vt };
    }
    const at = this.peek();
    const e = this.expr();
    if (!f.retorno) falha('tipo', `"${f.inf}" não declara retorno. Para devolver valor, acrescente ", devolvendo um ..." ao cabeçalho.`, at);
    const m = compativel(f.retorno, e.tipo);
    if (m) falha('tipo', `Valor devolvido incompatível: ${m}`, at);
    this.res = null;
    return { k: 'retorno', expr: e, tok: vt };
  }
  oReservo(vt) {
    const at = this.peek();
    let qtd, elem;
    if (at.t === 'w' && ARTIGOS[at.v] && !ARTIGOS[at.v].def) {
      this.next(); elem = this.tipoSing(at); qtd = { k: 'num', v: 1n, tipo: T.int(64), tok: at };
    } else {
      qtd = this.expr();
      if (qtd.tipo.k !== 'int') falha('tipo', 'A quantidade a reservar precisa ser inteira.', at);
      const w = this.peek();
      if (qtd.k === 'num' && qtd.v === 1n && w.t === 'w' && (TIPO_PLUR[w.v] || this.registrosPl.has(w.v)))
        falha('concordância', `Uma unidade pede singular: "Reservo ${artIndef(TIPO_PLUR[w.v] ? TIPO_SING[TIPO_PLUR[w.v]] : this.registrosPl.get(w.v).g, 's')} ${TIPO_PLUR[w.v] || this.registrosPl.get(w.v).nome}".`, w);
      elem = this.tipoPlural();
    }
    this.res = T.ptr(elem);
    return { k: 'reserva', qtd, elem, tok: vt };
  }
  oLibero(vt) {
    const at = this.peek();
    const e = this.expr();
    if (e.tipo.k !== 'ptr') falha('tipo', `Só se libera um endereço que veio de "Reservo"; isto é ${descTipo(e.tipo)}.`, at);
    this.res = null;
    return { k: 'libera', expr: e, tok: vt };
  }
  oChamada(vt) {
    this.next();
    const f = this.porForma.get(vt.v);
    this.anotar(vt, 'verbo', { lema: f.inf, forma: 'presente do indicativo, 1ª p. sing.', usuario: true });
    const args = this.argumentos(f, false, vt);
    this.res = f.retorno;
    return { k: 'chamada', f, args, tok: vt };
  }
  exemploChamada(f) {
    const ps = f.objetos.map(p => p.fixo ? `${p.art} ${p.nome}` : p.nome.toUpperCase());
    let s = (f.subj ? (f.sujeito.nome.toUpperCase() + ' ' + f.subj) : f.forma) + (ps.length ? ' ' + ps.join(' e ') : '');
    for (const [p, x] of f.preps) s += ` ${p} ${x.nome.toUpperCase()}`;
    return s;
  }
  argumentos(f, ehPredicado, vt) {
    const vals = [];
    const objs = f.objetos;
    for (let j = 0; j < objs.length; j++) {
      if (j > 0) {
        if (this.isW('e') || (this.isP(',') && j < objs.length - 1)) this.anotar(this.next(), 'conjunção', { papel: 'separa argumentos' });
        else this.esperado(`"e" e o ${j + 1}º argumento de "${f.inf}"`, `(forma: ${this.exemploChamada(f)})`);
      }
      if (objs[j].fixo) { this.complementoFixo(f, objs[j]); continue; }
      vals.push(this.argExpr(f, objs[j]));
    }
    for (const [prep, p] of f.preps) {
      const tk = this.peek();
      if (prep === 'e') {
        if (!this.isW('e')) this.esperado(`"e" e o segundo termo`, `(forma: ${this.exemploChamada(f)})`);
        this.anotar(this.next(), 'conjunção', { papel: 'segundo termo da preposição' });
        vals.push(this.argExpr(f, p));
        continue;
      }
      if (!(tk.t === 'w' && tk.v === prep))
        falha('regência', `"${f.inf}" foi definido com a preposição "${prep}" antes de ${p.simbolo ? p.nome : artDef(p.g, p.n) + ' ' + p.nome}: ${this.exemploChamada(f)}.`, tk,
          'Na Lavra a preposição faz parte da assinatura do verbo, como na regência do português.');
      this.expPrep(prep).ana.papel = `regência de ${f.inf}`;
      vals.push(this.argExpr(f, p));
    }
    return vals;
  }
  complementoFixo(f, p) {
    const at = this.peek(), w = this.peek(1);
    if (!(at.t === 'w' && at.v === p.art && w.t === 'w' && w.v === p.nome))
      falha('regência', `"${f.inf}" se usa com o complemento "${p.art} ${p.nome}": ${this.exemploChamada(f)}.`, at);
    this.next(); this.next();
    this.anotar(at, 'artigo', ARTIGOS[at.v]);
    this.anotar(w, 'nome', { g: p.g, n: p.n, papel: `complemento fixo de ${f.inf}` });
  }
  argExpr(f, p) {
    const tk = this.peek();
    const e = this.expr();
    const m = compativel(p.tipo, e.tipo);
    if (m) falha('tipo', `Argumento "${p.nome}" de "${f.inf}": ${m}`, tk);
    return e;
  }
  alvo(ctx) {
    const tk = this.peek();
    const e = this.primario();
    if (!LVALUES.has(e.k)) falha('sintaxe', `Isso não é um lugar onde se guarda valor${ctx ? ' (' + ctx + ')' : ''}.`, tk, 'Alvos: uma variável, um campo, um item de sequência ou "o apontado por" um endereço.');
    return e;
  }
  gravavel(e, tk) {
    if (e.k === 'var' && e.d.fixo) falha('tipo', `${cap(refNome(e.d))} é ${e.d.g === 'f' ? 'fixa' : 'fixo'} e não muda de valor.`, tk);
  }

  // ============================================================ CONDIÇÕES
  condicao() {
    let a = this.condE();
    while (this.isW('ou')) { const o = this.next(); this.anotar(o, 'conjunção', { papel: 'disjunção lógica' }); a = { k: 'ou', a, b: this.condE(), tok: o }; }
    return a;
  }
  condE() {
    let a = this.clausula('subj');
    while (this.isW('e') && !this.ehVerbo(this.peek(1))) { const o = this.next(); this.anotar(o, 'conjunção', { papel: 'conjunção lógica' }); a = { k: 'e', a, b: this.clausula('subj'), tok: o }; }
    return a;
  }
  clausula(modo) {
    const t0 = this.peek();
    const sujeito = this.expr();
    let neg = false;
    const nt = this.accW('não');
    if (nt) { neg = true; this.anotar(nt, 'advérbio', { papel: 'negação' }); }
    const v = this.peek();
    const formaSer = modo === 'subj' ? 'for' : 'ser';
    if (this.isW(formaSer)) {
      this.next();
      this.anotar(v, 'verbo', { lema: 'ser', forma: modo === 'subj' ? 'futuro do subjuntivo, 3ª p. sing. (condição)' : 'infinitivo pessoal (oração nominalizada)' });
      const aw = this.peek();
      const pa = aw.t === 'w' ? this.buscaAdj(aw.v) : null;
      if (pa) {
        this.next();
        const f = pa.f;
        const gn = sujeito.k === 'var' && !sujeito.d.simbolo ? [sujeito.d.g, sujeito.d.n] : sujeito.k === 'campo' && !sujeito.c.simbolo ? [sujeito.c.g, sujeito.c.n] : null;
        if (gn) {
          const certo = flexAdj(f.adj, f.adjG, gn[0], gn[1]);
          if (certo !== aw.v) falha('concordância', `O adjetivo concorda com o sujeito: "${certo}", não "${aw.o}".`, aw);
        }
        this.anotar(aw, 'adjetivo', { g: gn ? gn[0] : pa.g, n: gn ? gn[1] : pa.n, papel: `predicado "ser ${f.adj}"`, usuario: true });
        const m = compativel(f.sujeito.tipo, sujeito.tipo);
        if (m) falha('tipo', `Sujeito de "ser ${f.adj}": ${m}`, t0);
        const args = this.argumentos(f, true, aw);
        return { k: 'pred', f, sujeito, args, neg, tok: aw };
      }
      return this.relacao(sujeito, neg, v);
    }
    const pred = modo === 'subj' ? this.predSubj.get(v.v) : this.predInf.get(v.v);
    if (v.t === 'w' && pred) {
      this.next();
      this.anotar(v, 'verbo', { lema: pred.inf, forma: modo === 'subj' ? 'futuro do subjuntivo, 3ª p. sing. (predicado)' : 'infinitivo pessoal (predicado nominalizado)', usuario: true });
      const m = compativel(pred.sujeito.tipo, sujeito.tipo);
      if (m) falha('tipo', `Sujeito de "${pred.inf}": ${m}`, t0);
      const args = this.argumentos(pred, true, v);
      return { k: 'pred', f: pred, sujeito, args, neg, tok: v };
    }
    if (v.t === 'w') {
      if (modo === 'subj' && ['é', 'está', 'seja', 'fosse', 'ser', 'esteja', 'estiver'].includes(v.v))
        falha('morfologia', `A condição pede o futuro do subjuntivo: "for", não "${v.o}".`, v, 'Ex.: "Se a média for maior que 7:"');
      if (modo === 'inf' && ['for', 'é'].includes(v.v)) falha('morfologia', `Depois de "o fato de", o verbo vai no infinitivo: "ser", não "${v.o}".`, v);
      for (const p of this.predInf.values()) {
        if (modo === 'subj' && (v.v === terceiraPessoa(p.inf) || (v.v === p.inf && p.subj !== p.inf)))
          falha('morfologia', `Em condição, o predicado vai no futuro do subjuntivo: "${p.subj}", não "${v.o}".`, v);
        if (modo === 'inf' && v.v === p.subj && p.subj !== p.inf) falha('morfologia', `Depois de "o fato de", o verbo vai no infinitivo: "${p.inf}".`, v);
      }
      if (/(ar|er|ir)$/.test(v.v) && !this.predInf.has(v.v) && this.porInf.has(v.v))
        falha('tipo', `"${v.v}" não é predicado. Predicados se definem com sujeito: "Para X ${v.v} ..., devolvendo um lógico:".`, v);
    }
    if (sujeito.tipo.k === 'bool' && (modo === 'subj')) {
      if (neg) falha('sintaxe', 'Esperava "for" depois de "não".', v);
      this.esperado(`"for verdadeiro", "for falso" ou outra relação`, 'depois do lógico');
    }
    this.esperado(modo === 'subj' ? '"for" (futuro do subjuntivo) ou um predicado' : '"ser" ou um predicado no infinitivo', 'depois do sujeito da condição');
  }
  buscaAdj(w) {
    for (const [base, f] of this.predAdj)
      for (const g of ['m', 'f']) for (const n of ['s', 'p'])
        if (flexAdj(base, f.adjG, g, n) === w) return { f, g, n };
    return null;
  }
  relacao(a, neg, vtok) {
    const tk = this.next();
    const REL = { 'maior que': '>', 'menor que': '<', 'igual a': '==', 'diferente de': '!=', 'pelo menos': '>=', 'no máximo': '<=' };
    let op = REL[tk.v], b;
    if (tk.t === 'w' && (tk.v === 'nada' || tk.v === 'verdadeiro' || tk.v === 'falso')) {
      this.anotar(tk, tk.v === 'nada' ? 'pronome' : 'adjetivo', { papel: 'valor' });
      op = '==';
      b = tk.v === 'nada' ? { k: 'nada', tipo: T.nada, tok: tk } : { k: 'logico', v: tk.v === 'verdadeiro', tipo: T.bool, tok: tk };
    } else {
      if (!op) {
        this.i--;
        if (tk.t === 'w' && (tk.v === 'maior' || tk.v === 'menor')) falha('sintaxe', `Depois de "${tk.o}" vem "que": "${tk.v} que".`, tk);
        if (tk.t === 'w' && tk.v === 'igual') falha('regência', '"igual" rege "a": "igual a", "igual ao", "igual à".', tk);
        if (tk.t === 'w' && tk.v === 'diferente') falha('regência', '"diferente" rege "de": "diferente de", "diferente do".', tk);
        this.esperado('uma relação ("maior que", "menor que", "igual a", "diferente de", "pelo menos", "no máximo", "nada")', 'depois do verbo');
      }
      this.anotar(tk, 'locução', { papel: 'relação ' + op });
      this.checaContracao(tk);
      b = this.expr();
    }
    this.tipoComparacao(op, a.tipo, b.tipo, tk);
    return { k: 'cmp', op, a, b, neg, tok: tk };
  }
  tipoComparacao(op, ta, tb, tk) {
    const ord = ['<', '>', '<=', '>='].includes(op);
    if (ehNum(ta) && ehNum(tb)) return;
    if (ta.k === 'car' && tb.k === 'car') return;
    if (ta.k === 'texto' && tb.k === 'texto') return;
    if ((ta.k === 'ptr' || ta.k === 'nada') && (tb.k === 'ptr' || tb.k === 'nada')) return;
    if (ta.k === 'bool' && tb.k === 'bool' && !ord) return;
    falha('tipo', `Não dá para comparar ${descTipo(ta)} com ${descTipo(tb)}${ord ? ' por ordem' : ''}.`, tk);
  }

  // ============================================================ EXPRESSÕES
  expr() {
    let a = this.termo();
    for (;;) {
      if ((this.isW('mais') || this.isW('menos')) && this.podeIniciarValor(this.peek(1))) {
        const op = this.next(); this.anotar(op, 'advérbio', { papel: 'operador ' + (op.v === 'mais' ? '+' : '-') });
        const b = this.termo();
        a = this.binaria(op.v === 'mais' ? '+' : '-', a, b, op, 'infixa');
      } else break;
    }
    return a;
  }
  termo() {
    let a = this.fator();
    for (;;) {
      const tk = this.peek();
      if ((this.isW('vezes') || this.isW('dividido por') || this.isW('módulo')) && this.podeIniciarValor(this.peek(1))) {
        this.next();
        const op = tk.v === 'vezes' ? '*' : tk.v === 'módulo' ? '%' : '/';
        this.anotar(tk, tk.loc ? 'locução' : 'nome', { papel: 'operador ' + op });
        if (tk.loc) this.checaContracao(tk);
        a = this.binaria(op, a, this.fator(), tk, 'infixa');
      } else break;
    }
    return a;
  }
  fator() {
    let a = this.primario();
    while (this.isW('como')) {
      const c = this.next(); this.anotar(c, 'conjunção', { papel: 'conversão de tipo' });
      const t = this.tipoSing(null);
      a = this.conversao(a, t, c);
    }
    return a;
  }
  conversao(a, t, tk) {
    const de = a.tipo;
    const ok = t.k === 'texto' || (de.k === 'texto' && ['int', 'real', 'car', 'bool'].includes(t.k)) ||
      (['int', 'real', 'car', 'bool', 'ptr'].includes(de.k) && ['int', 'real', 'car', 'bool'].includes(t.k)) ||
      (de.k === 'int' && t.k === 'ptr') || (de.k === 'ptr' && t.k === 'ptr');
    if (!ok || de.k === 'rec' || de.k === 'seq') falha('tipo', `Não há conversão de ${descTipo(de)} para ${descTipo(t)}.`, tk);
    return { k: 'conv', e: a, tipo: t, tok: tk };
  }
  tipoBin(op, ta, tb, tok) {
    if (op === '+' && (ta.k === 'texto' || tb.k === 'texto')) return T.texto;
    if (op === '+' || op === '-') {
      if (ta.k === 'ptr' && tb.k === 'int') return ta;
      if (op === '-' && ta.k === 'ptr' && tb.k === 'ptr') return T.int(64);
      if (op === '+' && ta.k === 'int' && tb.k === 'ptr') falha('tipo', 'Na aritmética de endereços o endereço vem primeiro: "o cursor mais 1".', tok);
    }
    if (['&', '|', '^', '<<', '>>'].includes(op)) {
      if (ta.k !== 'int' || tb.k !== 'int') falha('tipo', `"${NOMES_OP[op]}" opera bit a bit e pede inteiros; recebeu ${descTipo(ta)} e ${descTipo(tb)}.`, tok);
      return igualTipo(ta, tb) || op === '<<' || op === '>>' ? ta : T.int(64);
    }
    if (!ehNum(ta) || !ehNum(tb)) {
      const outro = !ehNum(ta) ? ta : tb;
      falha('tipo', `Operação aritmética com ${descTipo(outro)}.`, tok, outro.k === 'car' ? 'Para a aritmética com o código do caractere, converta: "c como inteiro".' : null);
    }
    if (ta.k === 'real' || tb.k === 'real') return T.real(ta.k === 'real' && tb.k === 'real' && ta.bits === 32 && tb.bits === 32 ? 32 : 64);
    return igualTipo(ta, tb) ? ta : T.int(64);
  }
  binaria(op, a, b, tok, forma) {
    return { k: 'bin', op, a, b, tipo: this.tipoBin(op, a.tipo, b.tipo, tok), tok, forma };
  }

  primario() {
    const tk = this.peek();
    switch (tk.t) {
      case 'n': this.next(); this.anotar(tk, 'número', { tipo: tk.real ? 'real' : 'inteiro' }); return { k: 'num', v: tk.val, tipo: tk.real ? T.real(64) : T.int(64), tok: tk };
      case 's': this.next(); this.anotar(tk, 'texto'); return { k: 'texto', v: tk.v, tipo: T.texto, tok: tk };
      case 'c': this.next(); this.anotar(tk, 'caractere'); return { k: 'car', v: tk.v, tipo: T.car, tok: tk };
      case 'p':
        if (tk.v === '(') { this.next(); const e = this.expr(); this.expP(')', 'para fechar o parêntese'); return e; }
        this.esperado('um valor');
      case 'w': break;
      default: this.esperado('um valor');
    }
    const v = tk.v;
    if (v === 'verdadeiro' || v === 'falso') { this.next(); this.anotar(tk, 'adjetivo', { papel: 'valor lógico' }); return { k: 'logico', v: v === 'verdadeiro', tipo: T.bool, tok: tk }; }
    if (v === 'nada') { this.next(); this.anotar(tk, 'pronome', { papel: 'endereço nulo' }); return { k: 'nada', tipo: T.nada, tok: tk }; }
    if (ehSimbolo(v)) {
      this.next();
      const d = this.buscar(v);
      if (!d) falha('escopo', `O símbolo "${v}" não foi declarado.`, tk, `Declare com "Seja ${v} um inteiro."`);
      if (!d.simbolo) falha('morfologia', `"${v}"`, tk);
      this.anotar(tk, 'símbolo', { papel: d.papel || 'variável' });
      return { k: 'var', d, tipo: d.tipo, tok: tk };
    }
    if (ARTIGOS[v] && ARTIGOS[v].def) return this.nominal();
    if (ARTIGOS[v]) falha('morfologia', `O artigo indefinido "${tk.o}" cria algo novo. Para se referir a algo que já existe, use o definido ("${artDef(ARTIGOS[v].g, ARTIGOS[v].n)}").`, tk);
    const d = this.buscar(v);
    if (d) falha('morfologia', `Nome sem artigo: "${tk.o}". Escreva "${artDef(d.g, d.n)} ${tk.o}".`, tk, 'Na Lavra todo nome leva determinante; só símbolos de uma letra (x, y, n) vão nus.');
    if (this.porForma.has(v) || this.porInf.has(v)) {
      const f = this.porForma.get(v) || this.porInf.get(v);
      falha('sintaxe', `Verbo no lugar de um valor. Para usar o que "${f.inf}" devolve: "o resultado de ${f.inf} ...".`, tk);
    }
    if (OPS_BIN[v] || OPS_UN[v]) falha('morfologia', `Operação sem artigo: "${artDef((OPS_BIN[v] || OPS_UN[v]).g, 's')} ${tk.o} de ...".`, tk);
    this.esperado('um valor');
  }

  nominal() {
    const at = this.next();
    const art = ARTIGOS[at.v];
    const w = this.peek();
    if (w.t !== 'w') { this.esperado('um nome', `depois do artigo "${at.parte ? at.contr : at.o}"`); }
    this.anotar(at, 'artigo', art);
    const v = w.v;
    // operações binárias: "a soma de X e Y"
    if (OPS_BIN[v]) {
      const op = OPS_BIN[v];
      this.concordaArtigo(at, op.g, 's', w);
      this.next();
      this.anotar(w, 'nome', { g: op.g, n: 's', papel: 'operação ' + op.op });
      let entre = false;
      if (this.isW('entre')) {
        const et = this.next(); this.anotar(et, 'preposição');
        if (!op.comut) falha('morfologia', `"entre" indica simetria e só vale para operação comutativa. "${w.o}" não é comutativa: escreva "${at.o} ${w.o} de X e Y".`, et);
        entre = true;
      } else this.expPrep('de', `depois de "${at.o} ${w.o}"`);
      const a = this.expr();
      if (!this.isW('e')) this.esperado('"e" e o segundo termo', `de "${w.o}"`);
      this.anotar(this.next(), 'conjunção', { papel: 'segundo termo' });
      if (!entre && this.isW('de')) this.expPrep('de');
      const b = this.expr();
      return this.binaria(op.op, a, b, w, 'nominal');
    }
    if (OPS_UN[v]) {
      const op = OPS_UN[v];
      this.concordaArtigo(at, op.g, 's', w);
      this.next(); this.anotar(w, 'nome', { g: op.g, n: 's', papel: 'operação' });
      this.expPrep('de', `depois de "${at.o} ${w.o}"`);
      const t0 = this.peek();
      const a = this.expr();
      let tipo;
      if (op.op === 'tam') {
        if (a.tipo.k !== 'seq' && a.tipo.k !== 'texto') falha('tipo', `"o tamanho de" se aplica a sequências e textos; isto é ${descTipo(a.tipo)}.`, t0);
        tipo = T.int(64);
      } else if (op.op === 'raiz') { if (!ehNum(a.tipo)) falha('tipo', 'A raiz pede um número.', t0); tipo = T.real(64); }
      else if (op.op === '~') { if (a.tipo.k !== 'int') falha('tipo', 'O inverso bit a bit pede um inteiro.', t0); tipo = a.tipo; }
      else { if (!ehNum(a.tipo)) falha('tipo', 'O negativo pede um número.', t0); tipo = a.tipo; }
      return { k: 'un', op: op.op, a, tipo, tok: w };
    }
    if (v === 'deslocamento') {
      this.concordaArtigo(at, 'm', 's', w); this.next(); this.anotar(w, 'nome', { g: 'm', n: 's', papel: 'operação' });
      this.expPrep('de'); const a = this.expr();
      this.expPrep('por', 'antes da quantidade de bits'); const b = this.expr();
      const dir = this.next();
      if (!(dir.t === 'w' && (dir.v === 'à esquerda' || dir.v === 'à direita'))) { this.i--; this.esperado('"à esquerda" ou "à direita"'); }
      this.anotar(dir, 'locução', { papel: 'direção' });
      const op = dir.v === 'à esquerda' ? '<<' : '>>';
      return this.binaria(op, a, b, w, 'nominal');
    }
    if (v === 'resultado') {
      this.concordaArtigo(at, 'm', 's', w); this.next();
      if (this.isW('de') && this.peek(1).t === 'w' && this.porInf.has(this.peek(1).v)) {
        this.anotar(w, 'nome', { papel: 'retorno de função' });
        this.expPrep('de');
        const vt = this.next();
        const f = this.porInf.get(vt.v);
        this.anotar(vt, 'verbo', { lema: f.inf, forma: 'infinitivo (valor)', usuario: true });
        if (!f.retorno) falha('tipo', `"${f.inf}" não devolve valor; chame como oração: "${cap(f.forma)} ...".`, vt);
        const args = this.argumentos(f, false, vt);
        return { k: 'chamada', f, args, tipo: f.retorno, tok: vt };
      }
      if (this.isW('de') && this.peek(1).t === 'w') {
        const vt = this.peek(1);
        if (this.porForma.has(vt.v)) falha('morfologia', `Depois de "o resultado de" o verbo vai no infinitivo: "${this.porForma.get(vt.v).inf}".`, vt);
        if (/(ar|er|ir)$/.test(vt.v) && !ARTIGOS[vt.v]) falha('escopo', `O verbo "${vt.o}" não foi definido.`, vt);
      }
      this.anotar(w, 'pronome', { papel: 'anáfora: valor da oração anterior do período' });
      if (!this.res) falha('sintaxe', '"o resultado" retoma o valor da oração anterior do mesmo período, mas aqui não há nenhum.', w,
        'Use "o resultado" logo depois de um "leio", "reservo", "guardo", de um ajuste ou de uma chamada que devolve valor, no mesmo período: "Leio um real e guardo o resultado na prova."');
      if (this.res.k === 'void') falha('tipo', 'A oração anterior não produz valor.', w);
      return { k: 'anafora', tipo: this.res, tok: w };
    }
    if (v === 'fato') {
      this.concordaArtigo(at, 'm', 's', w); this.next(); this.anotar(w, 'nome', { papel: 'nominaliza uma oração' });
      const de = this.next();
      if (!(de.t === 'w' && de.v === 'de')) { this.i--; this.esperado('"de"', 'em "o fato de"'); }
      if (de.contr) falha('morfologia', `Antes do sujeito de um infinitivo não se contrai: escreva "o fato de ${CONTRACOES[de.contr][1]} ...", não "o fato ${de.contr} ...".`, de,
        'Regra da norma culta: em "o fato de a prova ser alta", o artigo pertence ao sujeito de "ser", e não à preposição.');
      this.anotar(de, 'preposição', { papel: 'não se contrai antes de sujeito de infinitivo' });
      const c = this.clausula('inf');
      return { k: 'fato', cond: c, tipo: T.bool, tok: w };
    }
    if (v === 'item') {
      this.concordaArtigo(at, 'm', 's', w); this.next(); this.anotar(w, 'nome', { papel: 'acesso por posição (base 0)' });
      const t0 = this.peek();
      const idx = this.expr();
      if (idx.tipo.k !== 'int') falha('tipo', 'A posição precisa ser um inteiro.', t0);
      this.expPrep('de', 'antes da coleção');
      const ct = this.peek();
      const base = this.primario();
      if (base.tipo.k !== 'seq') falha('tipo', `"o item ... de" acessa sequências; isto é ${descTipo(base.tipo)}.`, ct);
      if (!LVALUES.has(base.k)) falha('sintaxe', 'A coleção precisa estar na memória.', ct);
      return { k: 'indice', base, idx, tipo: base.tipo.of, tok: w };
    }
    if (v === 'endereço') {
      this.concordaArtigo(at, 'm', 's', w); this.next(); this.anotar(w, 'nome', { papel: 'operador de endereço' });
      this.expPrep('de');
      const t0 = this.peek();
      const alvo = this.primario();
      if (!LVALUES.has(alvo.k)) falha('sintaxe', 'Só tem endereço o que está na memória (variável, campo, item).', t0);
      return { k: 'endereco', alvo, tipo: T.ptr(alvo.tipo), tok: w };
    }
    if (v === 'apontado') {
      this.concordaArtigo(at, 'm', 's', w); this.next(); this.anotar(w, 'adjetivo', { papel: 'particípio: conteúdo do endereço' });
      this.expPrep('por', 'em "o apontado por"');
      const t0 = this.peek();
      const p = this.expr();
      if (p.tipo.k !== 'ptr') falha('tipo', `"o apontado por" pede um endereço; isto é ${descTipo(p.tipo)}.`, t0);
      return { k: 'apontado', e: p, tipo: p.tipo.to, tok: w };
    }
    const ord = analisarOrdinal(v);
    if (ord) {
      this.next();
      if (ARTIGOS[at.v].g !== ord.g || ARTIGOS[at.v].n !== 's')
        falha('concordância', `"${at.o} ${w.o}" não concorda: escreva "${artDef(ord.g, 's')} ${w.o}".`, at);
      this.anotar(w, 'numeral', { g: ord.g, n: 's', papel: ord.valor === -1 ? 'último' : `ordinal ${ord.valor}º` });
      this.expPrep('de', 'antes da coleção');
      const ct = this.peek();
      const base = this.primario();
      if (base.tipo.k !== 'seq') falha('tipo', `Ordinais acessam sequências; isto é ${descTipo(base.tipo)}.`, ct);
      if (!LVALUES.has(base.k)) falha('sintaxe', 'A coleção precisa estar na memória.', ct);
      const gCol = base.k === 'var' && !base.d.simbolo ? base.d.g : base.k === 'campo' && !base.c.simbolo ? base.c.g : null;
      if (gCol && gCol !== ord.g) {
        const certo = w.o.replace(/[oa]$/, gCol === 'f' ? 'a' : 'o');
        falha('concordância', `O ordinal concorda com a coleção: "${artDef(gCol, 's')} ${certo}".`, w);
      }
      if (ord.valor > base.tipo.n) falha('tipo', `A sequência tem só ${base.tipo.n} itens; não existe o ${ord.valor}º.`, w);
      const pos = ord.valor === -1 ? base.tipo.n - 1 : ord.valor - 1;
      return { k: 'indice', base, idx: { k: 'num', v: BigInt(pos), tipo: T.int(64), tok: w }, ordinal: ord.valor, tipo: base.tipo.of, tok: w };
    }
    if (TIPO_SING[v] || TIPO_PLUR[v]) falha('sintaxe', `"${at.o} ${w.o}": tipo no lugar de um valor.`, w, 'Tipos aparecem com artigo indefinido nas declarações ("Seja x um real").');
    // nome (variável ou campo)
    this.next();
    const d = this.buscar(v);
    const ehCampo = this.campos.has(v) || ehSimbolo(v);
    if (ehCampo && this.isW('de')) {
      const salvo = this.i;
      let base = null, erroBase = null;
      try { this.expPrep('de'); base = this.primario(); }
      catch (e) { if (!(e instanceof ErroLavra)) throw e; erroBase = e; }
      if (base) {
        const bt = base.tipo;
        if (bt.k === 'rec') {
          const c = bt.r.campos.find(c => c.nome === v);
          if (c) {
            if (!c.simbolo) this.concordaArtigo(at, c.g, c.n, w);
            this.anotar(w, c.simbolo ? 'símbolo' : 'nome', { g: c.g, n: c.n, papel: `campo de ${bt.r.nome}` });
            return { k: 'campo', base, c, off: c.off, tipo: c.tipo, art: at.v, tok: w };
          }
        }
        if (!d && bt.k === 'ptr' && bt.to.k === 'rec' && bt.to.r.campos.some(c => c.nome === v))
          falha('tipo', `${cap(this.descExpr(base))} é um endereço, não ${artIndef(bt.to.r.g, 's')} ${bt.to.r.nome}. Para chegar ao campo: "${at.v} ${w.o} do apontado ${this.comPrep('por', base)}".`, w);
        if (!d) falha('tipo', `"${w.o}" não é campo de ${descTipo(bt)}.`, w);
      }
      if (!d && erroBase) throw erroBase;
      this.i = salvo;
    }
    if (ehSimbolo(v)) falha('morfologia', `Símbolos não levam artigo: escreva "${v}", não "${at.o} ${v}".`, at);
    if (!d) {
      const alt = this.buscar(pluralizar(v)) || [...this.todosNomes()].map(n => this.buscar(n)).find(x => x && !x.simbolo && pluralizar(x.nome) === v);
      if (alt) falha('concordância', `"${at.v} ${w.o}" não foi declarado, mas existe a coleção "${refNome(alt)}". ${alt.n === 'p' ? 'Para um item da coleção use um ordinal ("a primeira das notas") ou "o item i das notas".' : ''}`.trim(), w);
      const parecido = [...this.todosNomes()].find(n => semAcento(n) === semAcento(v));
      falha('escopo', `"${at.v} ${w.o}" não foi declarad${art.g === 'f' ? 'a' : 'o'}.`, w,
        parecido ? `Quis dizer "${parecido}"?` : `Declare antes de usar: "Seja ${at.v} ${w.o} um ...".`);
    }
    if (d.simbolo) falha('morfologia', `Símbolos não levam artigo: escreva "${v}".`, at);
    this.concordaArtigo(at, d.g, d.n, w);
    this.anotar(w, 'nome', { g: d.g, n: d.n, papel: d.papel || 'variável' });
    return { k: 'var', d, tipo: d.tipo, art: at.v, tok: w };
  }
  *todosNomes() { for (let e = this.escopo; e; e = e.pai) yield* e.vars.keys(); }
  comPrep(prep, e) {
    const d = this.descExpr(e);
    const m = d.match(/^(o|a|os|as) (.*)$/);
    return m ? `${contrair(prep, m[1])} ${m[2]}` : `${prep} ${d}`;
  }
  descExpr(e) {
    switch (e.k) {
      case 'var': return refNome(e.d);
      case 'campo': return `${e.c.simbolo ? 'o ' + e.c.nome : artDef(e.c.g, e.c.n) + ' ' + e.c.nome} de ${this.descExpr(e.base)}`;
      case 'indice': return e.ordinal ? `o item ${e.ordinal === -1 ? 'último' : e.ordinal + 'º'} de ${this.descExpr(e.base)}` : `um item de ${this.descExpr(e.base)}`;
      case 'apontado': return 'o apontado';
      default: return 'esse valor';
    }
  }
}

// ============================================================ MÁQUINA
const hex = a => '0x' + a.toString(16).padStart(6, '0');
function formatarReal(v) {
  if (Number.isNaN(v)) return 'indefinido';
  if (v === Infinity) return 'infinito';
  if (v === -Infinity) return '-infinito';
  let s = String(parseFloat(v.toPrecision(12)));
  if (s.includes('e')) return s.replace('.', ',');
  return s.replace('.', ',');
}
class Maquina {
  constructor(prog, io) {
    this.p = prog; this.io = io;
    this.mem = []; this.est = [];
    this.BASE_PILHA = 0x100; this.sp = this.BASE_PILHA; this.LIM_PILHA = 0x80000; this.BASE_HEAP = 0x100000; this.hp = this.BASE_HEAP;
    this.blocos = new Map(); this.livres = [];
    this.globais = new Map(); this.quadro = null; this.res = undefined; this.prof = 0; this.passos = 0;
  }
  falha(msg, tok, dica) { throw new ErroLavra('execução', msg, tok, dica); }
  aloca(n, tok) {
    const a = this.sp;
    if (a + n > this.LIM_PILHA) this.falha('Estouro de pilha: a memória da pilha acabou.', tok);
    for (let k = 0; k < n; k++) { this.est[a + k] = 1; this.mem[a + k] = undefined; }
    this.sp += n; return a;
  }
  restauraPilha(sp) { for (let k = sp; k < this.sp; k++) this.est[k] = 4; this.sp = sp; }
  reservaHeap(n, tok) {
    if (n <= 0) this.falha('Reserva de tamanho zero ou negativo.', tok);
    for (let j = 0; j < this.livres.length; j++) {
      const [a, sz] = this.livres[j];
      if (sz >= n) { this.livres.splice(j, 1); if (sz > n) this.livres.push([a + n, sz - n]); this.marcaHeap(a, n); return a; }
    }
    const a = this.hp; this.hp += n;
    if (this.hp - this.BASE_HEAP > 4000000) this.falha('Memória esgotada: reservas demais.', tok);
    this.marcaHeap(a, n); return a;
  }
  marcaHeap(a, n) { this.blocos.set(a, n); for (let k = 0; k < n; k++) { this.est[a + k] = 2; this.mem[a + k] = undefined; } }
  libera(a, tok) {
    if (a === 0) this.falha('"Libero" recebeu nada.', tok);
    const n = this.blocos.get(a);
    if (n === undefined) {
      if (this.est[a] === 3) this.falha(`O endereço ${hex(a)} já tinha sido liberado (liberação dupla).`, tok);
      this.falha(`Só se libera o início de um bloco que veio de "Reservo"; ${hex(a)} não é.`, tok);
    }
    this.blocos.delete(a);
    for (let k = 0; k < n; k++) this.est[a + k] = 3;
    this.livres.push([a, n]);
  }
  checaEnd(a, tok) {
    if (a === 0) this.falha('Acesso ao apontado por nada.', tok);
    const e = this.est[a];
    if (e === undefined || e === 0) this.falha(`O endereço ${hex(a)} não pertence a nenhuma memória reservada.`, tok);
    if (e === 3) this.falha(`Uso de memória já liberada (${hex(a)}).`, tok);
    if (e === 4) this.falha(`O endereço ${hex(a)} era de uma variável que já saiu de escopo.`, tok);
  }
  ler(a, tok, nome) {
    this.checaEnd(a, tok);
    const v = this.mem[a];
    if (v === undefined) this.falha(`${nome ? cap(nome) : 'Esta posição da memória'} foi lid${nome && /^a /.test(nome) ? 'a' : 'o'} antes de receber valor.`, tok);
    return v;
  }
  gravar(a, v, tok) { this.checaEnd(a, tok); this.mem[a] = v; }
  lerComp(a, t, tok) { const n = tam(t); for (let k = 0; k < n; k++) this.checaEnd(a + k, tok); return { celulas: this.mem.slice(a, a + n) }; }
  mapa(d) { return d.func ? this.quadro : this.globais; }
  addrVar(d) { const a = this.mapa(d).get(d.id); if (a === undefined) this.falha('Variável sem endereço (erro interno).'); return a; }

  paraBig(v, t) {
    switch (t.k) {
      case 'int': return v; case 'car': return BigInt(v); case 'bool': return v ? 1n : 0n;
      case 'ptr': case 'nada': return BigInt(v); case 'real': return BigInt(Math.trunc(v));
    }
    return 0n;
  }
  paraNum(v, t) { return t.k === 'int' ? Number(v) : t.k === 'bool' ? (v ? 1 : 0) : v; }
  ajusta(r, t) { return t.sinal ? BigInt.asIntN(t.bits, r) : BigInt.asUintN(t.bits, r); }
  coagir(v, de, para) {
    if (para.k === 'int') return this.ajusta(this.paraBig(v, de), para);
    if (para.k === 'real') { const n = this.paraNum(v, de); return para.bits === 32 ? Math.fround(n) : n; }
    if (para.k === 'ptr') return de.k === 'nada' ? 0 : v;
    return v;
  }
  converter(v, de, para, tok) {
    if (para.k === 'texto') return this.formatar(v, de);
    if (de.k === 'texto') return this.interpretarEntrada(v, para, tok, true);
    switch (para.k) {
      case 'int':
        if (de.k === 'real') { if (!isFinite(v)) this.falha(`Não há inteiro para ${formatarReal(v)}.`, tok); return this.ajusta(BigInt(Math.trunc(v)), para); }
        return this.ajusta(this.paraBig(v, de), para);
      case 'real': { const n = this.paraNum(v, de); return para.bits === 32 ? Math.fround(n) : n; }
      case 'car': { const n = Number(this.paraBig(v, de)); if (n < 0 || n > 0x10ffff) this.falha(`${n} não é código de caractere válido.`, tok); return n; }
      case 'bool': return de.k === 'real' ? v !== 0 : this.paraBig(v, de) !== 0n;
      case 'ptr': return de.k === 'ptr' ? v : Number(this.paraBig(v, de));
    }
    return v;
  }
  formatar(v, t) {
    switch (t.k) {
      case 'int': return v.toString();
      case 'real': return formatarReal(v);
      case 'bool': return v ? 'verdadeiro' : 'falso';
      case 'car': return String.fromCodePoint(v);
      case 'texto': return v;
      case 'ptr': case 'nada': return v === 0 ? 'nada' : hex(v);
      case 'seq': case 'rec': return this.formatarComp(v.celulas, 0, t);
    }
    return String(v);
  }
  formatarComp(cel, off, t) {
    if (t.k === 'seq') {
      const s = tam(t.of), itens = [];
      for (let k = 0; k < t.n; k++) itens.push(this.formatarComp(cel, off + k * s, t.of));
      return '[' + itens.join('; ') + ']';
    }
    if (t.k === 'rec') return '{' + t.r.campos.map(c => `${c.nome}: ${this.formatarComp(cel, off + c.off, c.tipo)}`).join('; ') + '}';
    const v = cel[off];
    return v === undefined ? '?' : this.formatar(v, t);
  }
  interpretarEntrada(linha, t, tok, conversao) {
    const s = String(linha).trim();
    const onde = conversao ? 'Conversão' : 'Leitura';
    switch (t.k) {
      case 'int': {
        const limpo = s.replace(/[ _]/g, '');
        if (!/^-?\d+$/.test(limpo)) this.falha(`${onde}: esperava um número inteiro, veio "${s}".`, tok);
        return this.ajusta(BigInt(limpo), t);
      }
      case 'real': {
        const limpo = s.replace(/ /g, '').replace(',', '.');
        if (!/^-?\d+(\.\d+)?$|^-?\.\d+$/.test(limpo)) this.falha(`${onde}: esperava um número real (como 7,5), veio "${s}".`, tok);
        const n = parseFloat(limpo); return t.bits === 32 ? Math.fround(n) : n;
      }
      case 'car': if (!s.length) this.falha(`${onde}: esperava um caractere e veio linha vazia.`, tok); return s.codePointAt(0);
      case 'bool': {
        const x = semAcento(s.toLowerCase());
        if (['verdadeiro', 'v', 'sim', 's'].includes(x)) return true;
        if (['falso', 'f', 'nao', 'n'].includes(x)) return false;
        this.falha(`${onde}: esperava verdadeiro/falso (ou sim/não), veio "${s}".`, tok);
      }
      case 'texto': return String(linha);
    }
    this.falha(`${onde} de ${descTipo(t)} não é possível.`, tok);
  }
  potInt(x, y, t, tok) {
    if (y < 0n) this.falha('Potência inteira com expoente negativo; use reais.', tok);
    let r = 1n, b = x, e = y;
    const m = (z) => BigInt.asUintN(t.bits, z);
    while (e > 0n) { if (e & 1n) r = m(r * b); b = m(b * b); e >>= 1n; }
    return this.ajusta(r, t);
  }
  opBin(op, a, b, ta, tb, tr, tok) {
    if (tr.k === 'texto') return this.formatar(a, ta) + this.formatar(b, tb);
    if (tr.k === 'ptr') { const n = Number(this.paraBig(b, tb)); const s = tam(tr.to); return op === '+' ? a + n * s : a - n * s; }
    if (ta.k === 'ptr' && tb.k === 'ptr') return BigInt(Math.trunc((a - b) / tam(ta.to)));
    if (tr.k === 'real') {
      const x = this.paraNum(a, ta), y = this.paraNum(b, tb);
      let r;
      switch (op) { case '+': r = x + y; break; case '-': r = x - y; break; case '*': r = x * y; break; case '/': r = x / y; break; case '%': r = x % y; break; case '**': r = Math.pow(x, y); break; }
      return tr.bits === 32 ? Math.fround(r) : r;
    }
    const x = this.paraBig(a, ta), y = this.paraBig(b, tb);
    let r;
    switch (op) {
      case '+': r = x + y; break;
      case '-': r = x - y; break;
      case '*': r = x * y; break;
      case '/': if (y === 0n) this.falha('Divisão inteira por zero.', tok); r = x / y; break;
      case '%': if (y === 0n) this.falha('Resto de divisão por zero.', tok); r = x % y; break;
      case '**': return this.potInt(x, y, tr, tok);
      case '&': r = x & y; break; case '|': r = x | y; break; case '^': r = x ^ y; break;
      case '<<': if (y < 0n) this.falha('Deslocamento negativo.', tok); r = y >= BigInt(tr.bits) ? 0n : x << y; break;
      case '>>': if (y < 0n) this.falha('Deslocamento negativo.', tok); r = x >> (y > 128n ? 128n : y); break;
    }
    return this.ajusta(r, tr);
  }
  comparar(op, a, b, ta, tb) {
    let x = a, y = b;
    if (ta.k === 'real' || tb.k === 'real') { x = this.paraNum(a, ta); y = this.paraNum(b, tb); }
    else if (ta.k === 'int' || tb.k === 'int') { x = this.paraBig(a, ta); y = this.paraBig(b, tb); }
    switch (op) {
      case '==': return x === y; case '!=': return x !== y;
      case '<': return x < y; case '>': return x > y; case '<=': return x <= y; case '>=': return x >= y;
    }
  }

  *endereco(e) {
    switch (e.k) {
      case 'var': return this.addrVar(e.d);
      case 'campo': return (yield* this.endereco(e.base)) + e.off;
      case 'indice': {
        const b = yield* this.endereco(e.base);
        const i = yield* this.valor(e.idx);
        const n = e.base.tipo.n;
        if (i < 0n || i >= BigInt(n)) this.falha(`Posição ${i} fora da sequência de ${n} itens (válidas: 0 a ${n - 1}).`, e.tok);
        return b + Number(i) * tam(e.tipo);
      }
      case 'apontado': {
        const p = yield* this.valor(e.e);
        this.checaEnd(p, e.tok);
        return p;
      }
    }
  }
  nomeExpr(e) {
    if (e.k === 'var') return refNome(e.d);
    if (e.k === 'campo') return `${e.c.simbolo ? 'o ' + e.c.nome : artDef(e.c.g, e.c.n) + ' ' + e.c.nome}`;
    if (e.k === 'apontado') return 'o apontado';
    return null;
  }
  *valor(e) {
    switch (e.k) {
      case 'num': case 'texto': case 'car': case 'logico': return e.v;
      case 'nada': return 0;
      case 'var': case 'campo': case 'indice': case 'apontado': {
        const a = yield* this.endereco(e);
        if (e.tipo.k === 'seq' || e.tipo.k === 'rec') return this.lerComp(a, e.tipo, e.tok);
        return this.ler(a, e.tok, this.nomeExpr(e));
      }
      case 'bin': {
        const a = yield* this.valor(e.a), b = yield* this.valor(e.b);
        return this.opBin(e.op, a, b, e.a.tipo, e.b.tipo, e.tipo, e.tok);
      }
      case 'un': {
        if (e.op === 'tam') { if (e.a.tipo.k === 'seq') return BigInt(e.a.tipo.n); const s = yield* this.valor(e.a); return BigInt([...s].length); }
        const v = yield* this.valor(e.a);
        if (e.op === 'raiz') { const n = this.paraNum(v, e.a.tipo); if (n < 0) this.falha('Raiz de número negativo.', e.tok); return Math.sqrt(n); }
        if (e.op === '~') return this.ajusta(~v, e.tipo);
        return e.tipo.k === 'real' ? -v : this.ajusta(-v, e.tipo);
      }
      case 'conv': return this.converter(yield* this.valor(e.e), e.e.tipo, e.tipo, e.tok);
      case 'anafora': return this.res;
      case 'chamada': {
        const args = [];
        for (const a of e.args) args.push(yield* this.valor(a));
        return yield* this.chamar(e.f, args, e.tok, e.args.map(a => a.tipo));
      }
      case 'fato': return yield* this.cond(e.cond);
      case 'endereco': return yield* this.endereco(e.alvo);
    }
    this.falha('Expressão desconhecida (erro interno): ' + e.k);
  }
  *cond(c) {
    switch (c.k) {
      case 'e': return (yield* this.cond(c.a)) && (yield* this.cond(c.b));
      case 'ou': return (yield* this.cond(c.a)) || (yield* this.cond(c.b));
      case 'cmp': {
        const a = yield* this.valor(c.a), b = yield* this.valor(c.b);
        const r = this.comparar(c.op, a, b, c.a.tipo, c.b.tipo);
        return c.neg ? !r : r;
      }
      case 'pred': {
        const args = [yield* this.valor(c.sujeito)];
        for (const a of c.args) args.push(yield* this.valor(a));
        const r = yield* this.chamar(c.f, args, c.tok, [c.sujeito.tipo, ...c.args.map(a => a.tipo)]);
        return c.neg ? !r : r;
      }
    }
  }
  atribuir(a, t, v, tf, tok) {
    if (t.k === 'seq' || t.k === 'rec') {
      for (let k = 0; k < v.celulas.length; k++) { this.checaEnd(a + k, tok); this.mem[a + k] = v.celulas[k]; }
      return;
    }
    this.gravar(a, this.coagir(v, tf, t), tok);
  }
  *chamar(f, args, tok, tipos) {
    if (this.prof >= 2000) this.falha(`Estouro de pilha: recursão profunda demais em "${f.inf}".`, tok);
    this.ultimoTok = tok;
    const sp = this.sp, quadroAnt = this.quadro, resAnt = this.res;
    const q = new Map();
    f.params.forEach((p, j) => {
      const a = this.aloca(tam(p.tipo), tok);
      q.set(p.d.id, a);
      this.atribuir(a, p.tipo, args[j], (tipos && tipos[j]) || p.tipo, tok);
    });
    this.quadro = q; this.prof++;
    let sinal;
    try { sinal = yield* this.execBloco(f.corpo); }
    finally { this.prof--; this.quadro = quadroAnt; this.restauraPilha(sp); this.res = resAnt; }
    if (f.retorno && !(sinal && sinal.k === 'devolve'))
      this.falha(`"${f.inf}" terminou sem devolver ${artIndef(generoTipo(f.retorno), 's')} ${descTipo(f.retorno)}.`, f.tok);
    return sinal && sinal.k === 'devolve' ? sinal.v : undefined;
  }
  *execBloco(ps) {
    const sp = this.sp;
    for (const p of ps) {
      const s = yield* this.execPeriodo(p);
      if (s) { this.restauraPilha(sp); return s; }
    }
    this.restauraPilha(sp);
    return null;
  }
  *tique() { if (++this.passos % 2000 === 0) yield { k: 'tique' }; }
  *execPeriodo(p) {
    yield* this.tique();
    switch (p.k) {
      case 'se':
        for (const r of p.ramos) if (yield* this.cond(r.cond)) return yield* this.execBloco(r.corpo);
        if (p.senao) return yield* this.execBloco(p.senao);
        return null;
      case 'enquanto':
        while (yield* this.cond(p.cond)) {
          const s = yield* this.execBloco(p.corpo);
          if (s) { if (s.k === 'paro') break; if (s.k === 'devolve') return s; }
          yield* this.tique();
        }
        return null;
      case 'repete': {
        const n = p.vezes ? yield* this.valor(p.vezes) : null;
        for (let k = 0n; n === null || k < n; k++) {
          const s = yield* this.execBloco(p.corpo);
          if (s) { if (s.k === 'paro') break; if (s.k === 'devolve') return s; }
          yield* this.tique();
        }
        return null;
      }
      case 'faixa': {
        const de = yield* this.valor(p.de), ate = yield* this.valor(p.ate);
        const sp = this.sp;
        const a = this.aloca(1, p.tok);
        this.mapa(p.d).set(p.d.id, a);
        const passo = de <= ate ? 1n : -1n;
        this.mem[a] = de;
        let sinal = null;
        while (passo > 0n ? this.mem[a] <= ate : this.mem[a] >= ate) {
          const s = yield* this.execBloco(p.corpo);
          if (s) { if (s.k === 'paro') break; if (s.k === 'devolve') { sinal = s; break; } }
          this.mem[a] = this.ler(a, p.tok) + passo;
          yield* this.tique();
        }
        this.restauraPilha(sp);
        return sinal;
      }
      case 'cada': {
        const base = yield* this.endereco(p.col);
        const s1 = tam(p.col.tipo.of);
        for (let k = 0; k < p.col.tipo.n; k++) {
          this.mapa(p.d).set(p.d.id, base + k * s1);
          const s = yield* this.execBloco(p.corpo);
          if (s) { if (s.k === 'paro') break; if (s.k === 'devolve') return s; }
          yield* this.tique();
        }
        return null;
      }
    }
    for (const o of p.oracoes) {
      const s = yield* this.execOracao(o);
      if (s) return s;
    }
    return null;
  }
  *execOracao(o) {
    switch (o.k) {
      case 'decl': {
        let v;
        if (o.valor) v = yield* this.valor(o.valor);
        const lista = [];
        if (o.lista) for (const e of o.lista) lista.push(yield* this.valor(e));
        for (const d of o.ds) {
          const a = this.aloca(tam(d.tipo), o.tok);
          this.mapa(d).set(d.id, a);
          if (o.lista) {
            const s = tam(d.tipo.of);
            o.lista.forEach((e, k) => this.atribuir(a + k * s, d.tipo.of, lista[k], e.tipo, o.tok));
          } else if (o.valor) {
            if (d.tipo.k === 'seq' && o.valor.tipo.k !== 'seq') {
              const s = tam(d.tipo.of);
              for (let k = 0; k < d.tipo.n; k++) this.atribuir(a + k * s, d.tipo.of, v, o.valor.tipo, o.tok);
            } else this.atribuir(a, d.tipo, v, o.valor.tipo, o.tok);
          }
        }
        return null;
      }
      case 'guarda': {
        const v = yield* this.valor(o.expr);
        const a = yield* this.endereco(o.alvo);
        this.atribuir(a, o.alvo.tipo, v, o.expr.tipo, o.tok);
        this.res = o.alvo.tipo.k === 'seq' || o.alvo.tipo.k === 'rec' ? v : this.coagir(v, o.expr.tipo, o.alvo.tipo);
        return null;
      }
      case 'ajuste': {
        const a = yield* this.endereco(o.alvo);
        const atual = this.ler(a, o.tok, this.nomeExpr(o.alvo));
        const v = yield* this.valor(o.expr);
        const r = this.opBin(o.op, atual, v, o.alvo.tipo, o.expr.tipo, o.tipoOp, o.tok);
        const final = this.coagir(r, o.tipoOp, o.alvo.tipo);
        this.gravar(a, final, o.tok);
        this.res = final;
        return null;
      }
      case 'escrita': {
        let s = '';
        for (const pt of o.partes) s += this.formatar(yield* this.valor(pt), pt.tipo);
        this.io.escreve(s + (o.prompt ? '' : '\n'));
        return null;
      }
      case 'leitura': {
        const linha = yield { k: 'ler', tipo: o.tipo };
        if (linha === null || linha === undefined) this.falha('A entrada acabou: o programa pediu mais um valor e não havia.', o.tok);
        const v = this.interpretarEntrada(linha, o.tipo, o.tok);
        if (o.alvo) { const a = yield* this.endereco(o.alvo); this.gravar(a, v, o.tok); }
        this.res = v;
        return null;
      }
      case 'retorno': {
        if (!o.expr) return { k: 'devolve', v: undefined };
        const v = yield* this.valor(o.expr);
        const f = this.funcDe(o);
        return { k: 'devolve', v: f ? (f.retorno.k === 'seq' || f.retorno.k === 'rec' ? v : this.coagir(v, o.expr.tipo, f.retorno)) : v };
      }
      case 'chamada': {
        const args = [];
        for (const a of o.args) args.push(yield* this.valor(a));
        this.res = yield* this.chamar(o.f, args, o.tok, o.args.map(a => a.tipo));
        return null;
      }
      case 'reserva': {
        const n = Number(this.paraBig(yield* this.valor(o.qtd), o.qtd.tipo));
        if (n <= 0) this.falha(`Reserva de ${n} itens: a quantidade precisa ser positiva.`, o.tok);
        this.res = this.reservaHeap(n * tam(o.elem), o.tok);
        return null;
      }
      case 'libera': { this.libera(yield* this.valor(o.expr), o.tok); return null; }
      case 'paro': return { k: 'paro' };
      case 'pulo': return { k: 'pulo' };
    }
    this.falha('Oração desconhecida (erro interno): ' + o.k);
  }
  funcDe(o) { return o.f || this.funcAtualExec; }
  *executar() {
    for (const p of this.p.corpo) {
      const s = yield* this.execPeriodo(p);
      if (s && s.k === 'devolve') break;
    }
    if (this.p.comecar) yield* this.chamar(this.p.comecar, [], this.p.comecar.tok);
    if (this.blocos.size) {
      let total = 0; for (const n of this.blocos.values()) total += n;
      this.io.aviso && this.io.aviso(`${this.blocos.size} bloco(s) reservado(s) com "Reservo" não foram liberados (${total} células). Em um programa de verdade isso é vazamento de memória.`);
    }
  }
}
// liga cada "Devolvo" à sua função (a máquina precisa do tipo de retorno para coagir)
function ligarRetornos(prog) {
  const visita = (ps, f) => {
    for (const p of ps) {
      if (p.k === 'periodo') { for (const o of p.oracoes) if (o.k === 'retorno') o.f = f; }
      if (p.ramos) for (const r of p.ramos) visita(r.corpo, f);
      if (p.senao) visita(p.senao, f);
      if (p.corpo) visita(p.corpo, f);
    }
  };
  for (const f of prog.funcs) visita(f.corpo, f);
}

// ============================================================ VISUALIZAÇÕES
function classeBase(tk) {
  switch (tk.t) {
    case 'n': return 'número'; case 's': return 'texto'; case 'c': return 'caractere'; case 'p': return 'pontuação';
  }
  const v = tk.v;
  if (tk.loc) return 'locução';
  if (ARTIGOS[v]) return 'artigo';
  if (PREPOSICOES.has(v)) return 'preposição';
  if (v === 'e' || v === 'ou') return 'conjunção';
  if (NUCLEO[v]) return 'verbo';
  if (ehSimbolo(v)) return 'símbolo';
  if (TIPO_SING[v] || TIPO_PLUR[v]) return 'tipo';
  return 'palavra';
}
function tracos(tk) {
  const a = tk.ana || {};
  const cl = a.cl || classeBase(tk);
  const out = [];
  if (cl === 'artigo') {
    const x = ARTIGOS[tk.v];
    out.push(x.def ? 'definido' : 'indefinido', x.g === 'f' ? 'feminino' : 'masculino', x.n === 'p' ? 'plural' : 'singular');
    out.push(x.def ? 'refere algo existente' : 'introduz algo novo');
  } else if (cl === 'verbo') {
    if (a.lema && a.lema !== tk.v) out.push('lema: ' + a.lema);
    if (a.forma) out.push(a.forma);
    if (a.conj) out.push(`1ª p.: ${a.conj}${a.declarada ? ' (declarada)' : ''}`);
    if (a.subjSempre) out.push('em condição: ' + a.subj);
    if (a.usuario) out.push('definido no programa');
  } else {
    if (a.g) out.push(a.g === 'f' ? 'feminino' : 'masculino');
    if (a.n) out.push(a.n === 'p' ? 'plural' : 'singular');
    if (a.tipo) out.push(a.tipo);
    if (a.papel) out.push(a.papel);
    if (a.plural) out.push(`plural derivado: ${a.plural}`);
  }
  return { cl, tracos: out };
}
function analiseMorfologica(toks) {
  const linhas = [];
  for (let k = 0; k < toks.length; k++) {
    const tk = toks[k];
    if (!['w', 'n', 's', 'c', 'p'].includes(tk.t)) continue;
    const { cl, tracos: tr } = tracos(tk);
    if (tk.contr && tk.parte === 1) {
      const [p, a] = CONTRACOES[tk.contr];
      linhas.push({ linha: tk.line, escrito: tk.contr, lexema: `${p} + ${a}`, classe: 'contração', tracos: [`desfeita em "${p}" + "${a}"`] });
      linhas.push({ linha: tk.line, escrito: '', lexema: tk.v, classe: cl, tracos: tr, sub: true });
      continue;
    }
    if (tk.contr && tk.parte === 2) { linhas.push({ linha: tk.line, escrito: '', lexema: tk.v, classe: cl, tracos: tr, sub: true }); continue; }
    linhas.push({ linha: tk.line, escrito: tk.o, lexema: tk.loc ? tk.v : (tk.t === 'w' ? tk.v : tk.o), classe: cl, tracos: tr });
  }
  return linhas;
}

function descExprArvore(e) {
  switch (e.k) {
    case 'num': return { r: `${e.tipo.k === 'real' ? formatarReal(e.v) : e.v.toString()}`, t: e.tipo };
    case 'texto': return { r: JSON.stringify(e.v), t: e.tipo };
    case 'car': return { r: `'${String.fromCodePoint(e.v)}'`, t: e.tipo };
    case 'logico': return { r: e.v ? 'verdadeiro' : 'falso', t: e.tipo };
    case 'nada': return { r: 'nada', t: e.tipo };
    case 'var': return { r: refNome(e.d) + (e.d.alias ? ' (item do laço)' : ''), t: e.tipo };
    case 'anafora': return { r: 'o resultado (anáfora)', t: e.tipo };
  }
  return null;
}
function noExpr(e) {
  const simples = descExprArvore(e);
  if (simples) return { r: simples.r, t: simples.t };
  switch (e.k) {
    case 'bin': return { r: `${NOMES_OP[e.op]} (${e.op})${e.forma === 'infixa' ? ' infixa' : ''}`, t: e.tipo, f: [noExpr(e.a), noExpr(e.b)] };
    case 'un': return { r: { neg: 'negativo', '~': 'inverso bit a bit', raiz: 'raiz quadrada', tam: 'tamanho' }[e.op], t: e.tipo, f: [noExpr(e.a)] };
    case 'conv': return { r: 'conversão', t: e.tipo, f: [noExpr(e.e)] };
    case 'campo': return { r: `campo "${e.c.nome}" (+${e.off})`, t: e.tipo, f: [noExpr(e.base)] };
    case 'indice': return { r: e.ordinal ? `item ${e.ordinal === -1 ? 'último' : e.ordinal + 'º'}` : 'item (posição)', t: e.tipo, f: [noExpr(e.base)].concat(e.ordinal ? [] : [noExpr(e.idx)]) };
    case 'endereco': return { r: 'endereço de', t: e.tipo, f: [noExpr(e.alvo)] };
    case 'apontado': return { r: 'apontado por', t: e.tipo, f: [noExpr(e.e)] };
    case 'chamada': return { r: `chamada: ${e.f.inf}`, t: e.tipo, f: e.args.map(noExpr) };
    case 'fato': return { r: 'o fato de', t: e.tipo, f: [noCond(e.cond)] };
  }
  return { r: e.k };
}
function noCond(c) {
  const REL = { '>': 'maior que', '<': 'menor que', '==': 'igual a', '!=': 'diferente de', '>=': 'pelo menos', '<=': 'no máximo' };
  switch (c.k) {
    case 'e': return { r: 'e (lógico)', f: [noCond(c.a), noCond(c.b)] };
    case 'ou': return { r: 'ou (lógico)', f: [noCond(c.a), noCond(c.b)] };
    case 'cmp': return { r: `${c.neg ? 'não ' : ''}for ${REL[c.op]}`, f: [noExpr(c.a), noExpr(c.b)] };
    case 'pred': return { r: `${c.neg ? 'não ' : ''}predicado: ${c.f.inf} (${c.f.subj})`, f: [noExpr(c.sujeito)].concat(c.args.map(noExpr)) };
  }
}
function noOracao(o) {
  switch (o.k) {
    case 'decl': return { r: `declaração${o.ds.some(d => d.fixo) ? ' fixa' : ''}: ${o.ds.map(refNome).join(', ')}`, t: o.tipo, f: o.lista ? [{ r: 'valendo (lista)', f: o.lista.map(noExpr) }] : o.valor ? [{ r: 'valendo', f: [noExpr(o.valor)] }] : [] };
    case 'guarda': return { r: 'guardo', f: [noExpr(o.expr), { r: 'em', f: [noExpr(o.alvo)] }] };
    case 'ajuste': return { r: `ajuste no lugar (${o.op}=)`, f: [noExpr(o.alvo), noExpr(o.expr)] };
    case 'escrita': return { r: 'escrevo' + (o.prompt ? ' (prompt, sem quebra de linha)' : ''), f: o.partes.map(noExpr) };
    case 'leitura': return { r: 'leio', t: o.tipo, f: o.alvo ? [noExpr(o.alvo)] : [] };
    case 'retorno': return { r: 'devolvo', f: o.expr ? [noExpr(o.expr)] : [] };
    case 'chamada': return { r: `chamada: ${o.f.forma} (${o.f.inf})`, f: o.args.map(noExpr) };
    case 'reserva': return { r: 'reservo', t: T.ptr(o.elem), f: [noExpr(o.qtd)] };
    case 'libera': return { r: 'libero', f: [noExpr(o.expr)] };
    case 'paro': return { r: 'paro (interrompe o laço)' };
    case 'pulo': return { r: 'pulo (próxima volta)' };
  }
  return { r: o.k };
}
function noPeriodo(p) {
  switch (p.k) {
    case 'se': return { r: 'se', f: p.ramos.flatMap((r, j) => [{ r: j ? 'senão, se' : 'condição', f: [noCond(r.cond)] }, { r: 'então', f: r.corpo.map(noPeriodo) }]).concat(p.senao ? [{ r: 'senão', f: p.senao.map(noPeriodo) }] : []) };
    case 'enquanto': return { r: 'enquanto', f: [noCond(p.cond), { r: 'corpo', f: p.corpo.map(noPeriodo) }] };
    case 'repete': return { r: p.vezes ? 'repito N vezes' : 'repito sempre', f: (p.vezes ? [noExpr(p.vezes)] : []).concat([{ r: 'corpo', f: p.corpo.map(noPeriodo) }]) };
    case 'faixa': return { r: `para cada ${p.d.nome} de … até …`, f: [noExpr(p.de), noExpr(p.ate), { r: 'corpo', f: p.corpo.map(noPeriodo) }] };
    case 'cada': return { r: `para cada ${p.d.nome}`, f: [noExpr(p.col), { r: 'corpo', f: p.corpo.map(noPeriodo) }] };
  }
  if (p.oracoes.length === 1) return Object.assign({ periodo: true }, noOracao(p.oracoes[0]));
  return { r: `período (${p.oracoes.length} orações coordenadas)`, f: p.oracoes.map(noOracao) };
}
function arvore(prog) {
  const raizes = [];
  for (const r of prog.registros) raizes.push({ r: `registro ${r.nome} (${r.g === 'f' ? 'fem.' : 'masc.'}, ${r.tamanho} células)`, f: r.campos.map(c => ({ r: `+${c.off} ${c.simbolo ? c.nome : artDef(c.g, c.n) + ' ' + c.nome}`, t: c.tipo })) });
  for (const f of prog.funcs) {
    const sig = [];
    if (f.sujeito) sig.push(`sujeito: ${f.sujeito.nome}`);
    if (f.objetos.length) sig.push(`objetos: ${f.objetos.map(p => p.nome).join(', ')}`);
    for (const [p, x] of f.preps) sig.push(`${p}: ${x.nome}`);
    raizes.push({
      r: `${f.subj ? 'predicado' : 'função'} ${f.inf} → ${f.subj ? 'condição: ' + f.subj : 'execução: ' + f.forma}`, t: f.retorno,
      f: [{ r: 'regência: ' + (sig.join(' · ') || 'sem argumentos'), f: f.params.map(p => ({ r: p.simbolo ? p.nome : artDef(p.g, p.n) + ' ' + p.nome, t: p.tipo })) },
        { r: 'corpo', f: f.corpo.map(noPeriodo) }]
    });
  }
  if (prog.corpo.length) raizes.push({ r: 'nível de fora', f: prog.corpo.map(noPeriodo) });
  const linhas = [];
  const desenha = (n, pref, ultimo, raiz) => {
    const tipo = n.t ? ` : ${descTipo(n.t)}` : '';
    linhas.push({ texto: (raiz ? '' : pref + (ultimo ? '└─ ' : '├─ ')) + n.r, tipo, prof: raiz ? 0 : 1 });
    const f = n.f || [];
    f.forEach((c, j) => desenha(c, raiz ? '' : pref + (ultimo ? '   ' : '│  '), j === f.length - 1, false));
  };
  raizes.forEach(n => desenha(n, '', true, true));
  return linhas;
}

// ============================================================ API
function compilar(fonte) {
  let toks = null;
  try {
    toks = lexar(fonte);
    const a = new Analisador(toks);
    const prog = a.programa();
    ligarRetornos(prog);
    return { ok: true, prog, toks };
  } catch (e) {
    if (e instanceof ErroLavra) return { ok: false, erro: e, toks };
    throw e;
  }
}
// Erros do motor JavaScript que têm significado na Lavra.
function traduzirErro(e, maquina) {
  if (e instanceof ErroLavra) return e;
  if (e instanceof RangeError && /call stack/i.test(e.message))
    return new ErroLavra('execução', 'Estouro de pilha: recursão profunda demais (chamadas que nunca terminam?).', maquina && maquina.ultimoTok);
  return null;
}
// Execução síncrona (testes): entradas é uma lista de linhas.
function executarSincrono(prog, entradas, limitePassos) {
  let saida = '';
  const avisos = [];
  const fila = (entradas || []).slice();
  const m = new Maquina(prog, { escreve: s => { saida += s; }, aviso: s => avisos.push(s) });
  const g = m.executar();
  let r;
  try {
    r = g.next();
    let passos = 0;
    while (!r.done) {
      if (r.value.k === 'ler') {
        const v = fila.length ? fila.shift() : null;
        if (v !== null) saida += v + '\n';
        r = g.next(v);
      } else {
        if (limitePassos && ++passos > limitePassos) return { saida, erro: new ErroLavra('execução', 'Limite de passos atingido.'), avisos };
        r = g.next();
      }
    }
  } catch (e) {
    const er = traduzirErro(e, m);
    if (er) return { saida, erro: er, avisos };
    throw e;
  }
  return { saida, erro: null, avisos };
}

const Lavra = {
  classe: tk => (tk.ana && tk.ana.cl) || classeBase(tk),
  compilar, executarSincrono, traduzirErro, Maquina, ErroLavra, ROTULO_ERRO, analiseMorfologica, arvore, lexar,
  morfologia: { conjugar1ps, futuroSubj, pluralizar, contrair, analisarOrdinal },
  util: { descTipo, tam, refNome, artDef, formatarReal, generoTipo, T }
};
if (typeof module !== 'undefined' && module.exports) module.exports = Lavra;
else raiz.Lavra = Lavra;
})(typeof window !== 'undefined' ? window : globalThis);
