/* Lavra — gerador de código x86-64 (sintaxe Intel do GNU as, convenção System V).
   Recebe a árvore já verificada pelo analisador e produz um arquivo .s que,
   montado e ligado com runtime.c, vira um executável nativo. */
(function (raiz) {
'use strict';
const L = (typeof module !== 'undefined' && module.exports) ? require('./lavra.js') : raiz.Lavra;
const { refNome, descTipo } = L.util;
const semAcento = s => s.normalize('NFD').replace(/[̀-ͯ]/g, '');
const limpa = s => semAcento(s).replace(/[^A-Za-z0-9_]/g, '_');

// ---------- tamanhos e alinhamentos reais (bytes)
function layoutRec(r) {
  if (r.bytes !== undefined) return;
  r.bytes = 0;
  let off = 0, al = 1;
  for (const c of r.campos) {
    const a = alignOf(c.tipo);
    off = Math.ceil(off / a) * a;
    c.offB = off;
    off += sizeOf(c.tipo);
    al = Math.max(al, a);
  }
  r.alinha = al;
  r.bytes = Math.max(al, Math.ceil(off / al) * al);
}
function sizeOf(t) {
  switch (t.k) {
    case 'int': case 'real': return t.bits / 8;
    case 'bool': return 1;
    case 'car': return 4;
    case 'texto': case 'ptr': case 'nada': return 8;
    case 'seq': return t.n * sizeOf(t.of);
    case 'rec': layoutRec(t.r); return t.r.bytes;
  }
  return 8;
}
function alignOf(t) {
  if (t.k === 'seq') return alignOf(t.of);
  if (t.k === 'rec') { layoutRec(t.r); return t.r.alinha; }
  return sizeOf(t);
}
function descritor(t) {
  switch (t.k) {
    case 'int': return (t.sinal ? 'i' : 'u') + t.bits;
    case 'real': return 'r' + t.bits;
    case 'bool': return 'b';
    case 'car': return 'c';
    case 'texto': return 't';
    case 'ptr': case 'nada': return 'p';
    case 'seq': return `[${t.n}*${sizeOf(t.of)}:${descritor(t.of)}]`;
    case 'rec': layoutRec(t.r); return '{' + t.r.campos.map(c => `${c.nome}@${c.offB}=${descritor(c.tipo)}`).join(',') + '}';
  }
}
const ehComp = t => t.k === 'seq' || t.k === 'rec';
const INT_REGS = ['rdi', 'rsi', 'rdx', 'rcx', 'r8', 'r9'];
const SUB = {
  rdi: ['edi', 'di', 'dil'], rsi: ['esi', 'si', 'sil'], rdx: ['edx', 'dx', 'dl'], rcx: ['ecx', 'cx', 'cl'],
  r8: ['r8d', 'r8w', 'r8b'], r9: ['r9d', 'r9w', 'r9b'], rax: ['eax', 'ax', 'al']
};
const regTam = (r, bytes) => bytes === 8 ? r : SUB[r][{ 4: 0, 2: 1, 1: 2 }[bytes]];
const PTR = { 1: 'byte', 2: 'word', 4: 'dword', 8: 'qword' };

class ErroGerador extends Error {}

class GeradorX86 {
  constructor(prog, fonte) {
    this.p = prog;
    this.fonte = (fonte || '').split('\n');
    this.out = [];
    this.textos = new Map(); this.reais = new Map(); this.gl = new Map();
    this.nrot = 0; this.pilha = 0; this.q = null; this.lacos = []; this.funcAtual = null;
    this.linhaComentada = 0;
  }
  // ---------- emissão
  i(s, com) { this.out.push('    ' + (com ? s.padEnd(Math.max(34, s.length + 2)) + '# ' + com : s)); }
  rot(l) { this.out.push(l + ':'); }
  com(s) { this.out.push('    # ' + s); }
  novo(b) { return `.L${b}${++this.nrot}`; }
  fonteDe(tok) {
    if (!tok || !tok.line || tok.line === this.linhaComentada) return;
    this.linhaComentada = tok.line;
    const txt = (this.fonte[tok.line - 1] || '').trim();
    if (txt) this.out.push(`    # ── ${tok.line}: ${txt}`);
  }
  texto(s) {
    if (!this.textos.has(s)) this.textos.set(s, `.Ltexto${this.textos.size}`);
    return this.textos.get(s);
  }
  real(v) {
    const chave = Object.is(v, -0) ? '-0' : String(v);
    if (!this.reais.has(chave)) {
      const dv = new DataView(new ArrayBuffer(8));
      dv.setFloat64(0, v, true);
      this.reais.set(chave, { rot: `.Lreal${this.reais.size}`, bits: dv.getBigUint64(0, true), v });
    }
    return this.reais.get(chave).rot;
  }
  rotFunc(f) { return 'lavra_' + limpa(f.inf); }
  chama(f) {
    if (this.pilha % 2) { this.i('sub rsp, 8', 'alinha a pilha em 16 bytes'); this.i(`call ${f}`); this.i('add rsp, 8'); }
    else this.i(`call ${f}`);
  }
  empilha(real) {
    if (real) { this.i('sub rsp, 8'); this.i('movsd qword ptr [rsp], xmm0'); }
    else this.i('push rax');
    this.pilha++;
  }
  desempilha(reg) {
    if (reg.startsWith('xmm')) { this.i(`movsd ${reg}, qword ptr [rsp]`); this.i('add rsp, 8'); }
    else this.i(`pop ${reg}`);
    this.pilha--;
  }
  falha(linha, msg) {
    this.i(`mov edi, ${linha}`);
    this.i(`lea rsi, [rip + ${this.texto(msg)}]`);
    this.chama('lavra_falha@PLT');
  }

  // ---------- quadros
  global(d) {
    if (this.gl.has(d.id)) return;
    this.gl.set(d.id, { rot: `lv_${limpa(d.nome)}_${d.id}`, tam: sizeOf(d.tipo), al: alignOf(d.tipo), nome: refNome(d), tipo: d.tipo });
  }
  planeja(corpo, params) {
    const q = { slots: new Map(), ocultos: new Map(), off: 0 };
    q.reserva = (size, al) => { q.off = Math.ceil((q.off + size) / al) * al; return q.off; };
    for (const p of params) q.slots.set(p.d.id, q.reserva(sizeOf(p.tipo), alignOf(p.tipo)));
    q.spill = params.map(p => ehComp(p.tipo) ? q.reserva(8, 8) : null);
    q.res = q.reserva(8, 8);
    for (const p of corpo) this.planejaPeriodo(p, q);
    q.tamanho = Math.ceil(q.off / 16) * 16;
    return q;
  }
  planejaPeriodo(p, q) {
    const local = d => { if (d.func) q.slots.set(d.id, q.reserva(sizeOf(d.tipo), alignOf(d.tipo))); else this.global(d); };
    const sub = ps => { for (const x of ps) this.planejaPeriodo(x, q); };
    switch (p.k) {
      case 'se': p.ramos.forEach(r => sub(r.corpo)); if (p.senao) sub(p.senao); return;
      case 'enquanto': sub(p.corpo); return;
      case 'repete': if (p.vezes) q.ocultos.set(p, { cont: q.reserva(8, 8) }); sub(p.corpo); return;
      case 'faixa': local(p.d); q.ocultos.set(p, { lim: q.reserva(8, 8), passo: q.reserva(8, 8) }); sub(p.corpo); return;
      case 'cada': q.ocultos.set(p, { idx: q.reserva(8, 8), base: q.reserva(8, 8) }); q.slots.set(p.d.id, q.reserva(8, 8)); sub(p.corpo); return;
    }
    for (const o of p.oracoes) {
      if (o.k !== 'decl') continue;
      for (const d of o.ds) local(d);
      if (o.valor && o.tipo.k === 'seq' && o.valor.tipo.k !== 'seq') q.ocultos.set(o, { idx: q.reserva(8, 8) });
    }
  }

  // ---------- programa
  gerar() {
    const o = this.out;
    o.push('# Gerado pelo compilador Lavra — x86-64, sintaxe Intel (GNU as), convenção System V');
    o.push('# Montagem: gcc programa.s runtime.c -o programa -lm');
    o.push('    .intel_syntax noprefix');
    o.push('    .text');
    o.push('    .globl main');
    // main
    const qm = this.planeja(this.p.corpo, []);
    this.q = qm; this.pilha = 0; this.funcAtual = null;
    o.push('');
    this.com('entrada do programa: executa o nível de fora e depois "Para começar"');
    this.rot('main');
    this.i('push rbp'); this.i('mov rbp, rsp');
    if (qm.tamanho) this.i(`sub rsp, ${qm.tamanho}`, 'variáveis ocultas do nível de fora');
    this.i('call lavra_inicia@PLT');
    this.periodos(this.p.corpo);
    if (this.p.comecar) this.i(`call ${this.rotFunc(this.p.comecar)}`, 'Para começar');
    this.i('call lavra_termina@PLT');
    this.i('xor eax, eax');
    this.i('leave'); this.i('ret');
    for (const f of this.p.funcs) this.funcao(f);
    // dados
    o.push('');
    o.push('    .section .rodata');
    for (const [s, r] of this.textos) { o.push(`${r}:`); o.push(`    .asciz "${this.escapa(s)}"`); }
    if (this.reais.size) o.push('    .balign 8');
    for (const { rot, bits, v } of this.reais.values()) { o.push(`${rot}:`); o.push(`    .quad 0x${bits.toString(16).padStart(16, '0')}    # ${v}`); }
    if (this.gl.size) {
      o.push('');
      o.push('    .bss');
      for (const g of this.gl.values()) {
        o.push(`    .balign ${Math.max(g.al, 1)}`);
        o.push(`${g.rot}:    # ${g.nome} : ${descTipo(g.tipo)}`);
        o.push(`    .zero ${g.tam}`);
      }
    }
    o.push('    .section .note.GNU-stack,"",@progbits');
    return o.join('\n') + '\n';
  }
  escapa(s) {
    const bytes = new TextEncoder().encode(s);
    let r = '';
    for (const b of bytes) {
      if (b === 0x22) r += '\\"';
      else if (b === 0x5c) r += '\\\\';
      else if (b === 10) r += '\\n';
      else if (b === 9) r += '\\t';
      else if (b < 32 || b > 126) r += '\\' + b.toString(8).padStart(3, '0');
      else r += String.fromCharCode(b);
    }
    return r;
  }
  funcao(f) {
    const q = this.planeja(f.corpo, f.params);
    this.q = q; this.pilha = 0; this.funcAtual = f;
    this.out.push('');
    this.linhaComentada = 0;
    this.fonteDe(f.tok);
    this.rot(this.rotFunc(f));
    this.i('push rbp'); this.i('mov rbp, rsp');
    if (q.tamanho) this.i(`sub rsp, ${q.tamanho}`, `quadro de ${q.tamanho} bytes`);
    let ni = 0, nf = 0;
    const regs = f.params.map(p => p.tipo.k === 'real' ? 'xmm' + (nf++) : INT_REGS[ni++]);
    if (ni > 6 || nf > 8) throw new ErroGerador(`"${f.inf}" tem argumentos demais para os registradores (até 6 inteiros e 8 reais nesta versão).`);
    // escalares direto dos registradores para o quadro
    f.params.forEach((p, j) => {
      const r = regs[j], dst = q.slots.get(p.d.id), nome = p.simbolo ? p.nome : artNome(p);
      if (ehComp(p.tipo)) { this.i(`mov qword ptr [rbp - ${q.spill[j]}], ${r}`, `endereço de ${nome}`); return; }
      if (p.tipo.k === 'real') {
        if (p.tipo.bits === 64) this.i(`movsd qword ptr [rbp - ${dst}], ${r}`, nome);
        else { this.i(`cvtsd2ss xmm15, ${r}`); this.i(`movss dword ptr [rbp - ${dst}], xmm15`, nome); }
        return;
      }
      const t = sizeOf(p.tipo);
      this.i(`mov ${PTR[t]} ptr [rbp - ${dst}], ${regTam(r, t)}`, nome);
    });
    // registros e sequências passados por endereço: copia para o quadro
    f.params.forEach((p, j) => {
      if (!ehComp(p.tipo)) return;
      this.i(`lea rdi, [rbp - ${q.slots.get(p.d.id)}]`);
      this.i(`mov rsi, qword ptr [rbp - ${q.spill[j]}]`);
      this.i(`mov edx, ${sizeOf(p.tipo)}`);
      this.chama('memcpy@PLT');
    });
    this.fRet = this.novo('fim_' + limpa(f.inf) + '_');
    this.periodos(f.corpo);
    if (f.retorno) this.falha(f.tok.line, `"${f.inf}" terminou sem devolver ${artIndef(f.retorno)} ${descTipo(f.retorno)}.`);
    this.rot(this.fRet);
    this.i('leave'); this.i('ret');
  }

  // ---------- períodos e orações
  periodos(ps) { for (const p of ps) this.periodo(p); }
  periodo(p) {
    this.fonteDe(p.tok);
    switch (p.k) {
      case 'se': {
        const fim = this.novo('fimse');
        p.ramos.forEach(r => {
          const prox = this.novo('senao');
          this.cond(r.cond);
          this.i('test eax, eax'); this.i(`jz ${prox}`);
          this.periodos(r.corpo);
          this.i(`jmp ${fim}`);
          this.rot(prox);
        });
        if (p.senao) this.periodos(p.senao);
        this.rot(fim);
        return;
      }
      case 'enquanto': {
        const ini = this.novo('enquanto'), fim = this.novo('fimenquanto');
        this.rot(ini);
        this.cond(p.cond);
        this.i('test eax, eax'); this.i(`jz ${fim}`);
        this.lacos.push({ cont: ini, fim });
        this.periodos(p.corpo);
        this.lacos.pop();
        this.i(`jmp ${ini}`);
        this.rot(fim);
        return;
      }
      case 'repete': {
        const ini = this.novo('repito'), cont = this.novo('volta'), fim = this.novo('fimrepito');
        if (p.vezes) {
          const h = this.q.ocultos.get(p).cont;
          this.val(p.vezes);
          this.i(`mov qword ptr [rbp - ${h}], rax`, 'contador de voltas');
          this.rot(ini);
          this.i(`cmp qword ptr [rbp - ${h}], 0`); this.i(`jle ${fim}`);
          this.lacos.push({ cont, fim });
          this.periodos(p.corpo);
          this.lacos.pop();
          this.rot(cont);
          this.i(`dec qword ptr [rbp - ${h}]`); this.i(`jmp ${ini}`);
        } else {
          this.rot(ini);
          this.lacos.push({ cont: ini, fim });
          this.periodos(p.corpo);
          this.lacos.pop();
          this.i(`jmp ${ini}`);
        }
        this.rot(fim);
        return;
      }
      case 'faixa': {
        const h = this.q.ocultos.get(p);
        const ini = this.novo('cada'), desc = this.novo('desce'), corpo = this.novo('corpo'), cont = this.novo('volta'), fim = this.novo('fimcada');
        this.val(p.de); this.empilha(false);
        this.endVar(p.d); this.desempilha('rcx'); this.i('mov qword ptr [rax], rcx', `${p.d.nome} = início`);
        this.val(p.ate); this.i(`mov qword ptr [rbp - ${h.lim}], rax`, 'limite');
        this.endVar(p.d); this.i('mov rcx, qword ptr [rax]'); this.i(`mov rax, qword ptr [rbp - ${h.lim}]`);
        this.i('mov rdx, 1'); this.i('mov r8, -1'); this.i('cmp rcx, rax'); this.i('cmovg rdx, r8');
        this.i(`mov qword ptr [rbp - ${h.passo}], rdx`, 'passo: +1 ou -1');
        this.rot(ini);
        this.endVar(p.d); this.i('mov rax, qword ptr [rax]'); this.i(`mov rcx, qword ptr [rbp - ${h.lim}]`);
        this.i(`cmp qword ptr [rbp - ${h.passo}], 0`); this.i(`jl ${desc}`);
        this.i('cmp rax, rcx'); this.i(`jg ${fim}`); this.i(`jmp ${corpo}`);
        this.rot(desc);
        this.i('cmp rax, rcx'); this.i(`jl ${fim}`);
        this.rot(corpo);
        this.lacos.push({ cont, fim });
        this.periodos(p.corpo);
        this.lacos.pop();
        this.rot(cont);
        this.endVar(p.d); this.i(`mov rcx, qword ptr [rbp - ${h.passo}]`); this.i('add qword ptr [rax], rcx');
        this.i(`jmp ${ini}`);
        this.rot(fim);
        return;
      }
      case 'cada': {
        const h = this.q.ocultos.get(p), alias = this.q.slots.get(p.d.id);
        const n = p.col.tipo.n, s = sizeOf(p.col.tipo.of);
        const ini = this.novo('cada'), cont = this.novo('volta'), fim = this.novo('fimcada');
        this.end(p.col);
        this.i(`mov qword ptr [rbp - ${h.base}], rax`, 'início da coleção');
        this.i(`mov qword ptr [rbp - ${h.idx}], 0`);
        this.rot(ini);
        this.i(`cmp qword ptr [rbp - ${h.idx}], ${n}`); this.i(`jge ${fim}`);
        this.i(`mov rax, qword ptr [rbp - ${h.idx}]`);
        if (s !== 1) this.i(`imul rax, rax, ${s}`);
        this.i(`add rax, qword ptr [rbp - ${h.base}]`);
        this.i(`mov qword ptr [rbp - ${alias}], rax`, `${artNome(p.d)} aponta para o item`);
        this.lacos.push({ cont, fim });
        this.periodos(p.corpo);
        this.lacos.pop();
        this.rot(cont);
        this.i(`inc qword ptr [rbp - ${h.idx}]`); this.i(`jmp ${ini}`);
        this.rot(fim);
        return;
      }
    }
    // período de orações coordenadas: guarda o valor só quando uma oração seguinte usa "o resultado"
    p.oracoes.forEach((o, j) => {
      const t = this.oracao(o);
      if (t && p.oracoes.slice(j + 1).some(temAnafora)) {
        if (t.k === 'real') this.i(`movsd qword ptr [rbp - ${this.q.res}], xmm0`, '"o resultado"');
        else this.i(`mov qword ptr [rbp - ${this.q.res}], rax`, '"o resultado"');
      }
    });
  }
  oracao(o) {
    const lin = o.tok ? o.tok.line : 0;
    switch (o.k) {
      case 'decl': {
        if (o.lista) {
          const s = sizeOf(o.tipo.of);
          for (const d of o.ds) o.lista.forEach((e, k) => {
            this.val(e); this.coagir(e.tipo, o.tipo.of, lin);
            this.empilha(o.tipo.of.k === 'real');
            this.endVar(d); if (k) this.i(`add rax, ${k * s}`);
            this.desempilha(o.tipo.of.k === 'real' ? 'xmm0' : 'rcx');
            this.guarda(o.tipo.of);
          });
          return null;
        }
        if (!o.valor) return null;
        if (o.tipo.k === 'seq' && o.valor.tipo.k !== 'seq') {
          const h = this.q.ocultos.get(o).idx, of = o.tipo.of, s = sizeOf(of), real = of.k === 'real';
          this.val(o.valor); this.coagir(o.valor.tipo, of, lin); this.empilha(real);
          for (const d of o.ds) {
            const ini = this.novo('preenche'), fim = this.novo('fimpreenche');
            this.i(`mov qword ptr [rbp - ${h}], 0`);
            this.rot(ini);
            this.i(`cmp qword ptr [rbp - ${h}], ${o.tipo.n}`); this.i(`jge ${fim}`);
            this.endVar(d);
            this.i(`mov rcx, qword ptr [rbp - ${h}]`); if (s !== 1) this.i(`imul rcx, rcx, ${s}`); this.i('add rax, rcx');
            if (real) this.i('movsd xmm0, qword ptr [rsp]'); else this.i('mov rcx, qword ptr [rsp]');
            this.guarda(of);
            this.i(`inc qword ptr [rbp - ${h}]`); this.i(`jmp ${ini}`);
            this.rot(fim);
          }
          this.desempilha(real ? 'xmm0' : 'rcx');
          return null;
        }
        const real = o.tipo.k === 'real';
        this.val(o.valor); this.coagir(o.valor.tipo, o.tipo, lin); this.empilha(real);
        for (const d of o.ds) {
          this.endVar(d);
          if (real) this.i('movsd xmm0, qword ptr [rsp]'); else this.i('mov rcx, qword ptr [rsp]');
          this.guarda(o.tipo);
        }
        this.desempilha(real ? 'xmm0' : 'rcx');
        return null;
      }
      case 'guarda': {
        const t = o.alvo.tipo, real = t.k === 'real';
        if (real && t.bits === 64) this.valComo(o.expr, t); else { this.val(o.expr); this.coagir(o.expr.tipo, t, lin); }
        this.empilha(real);
        this.end(o.alvo);
        this.desempilha(real ? 'xmm0' : 'rcx');
        this.guarda(t);
        if (!ehComp(t)) this.carrega(t);
        return t;
      }
      case 'ajuste': {
        const t = o.alvo.tipo, tr = o.tipoOp;
        this.end(o.alvo); this.empilha(false);
        this.val(o.expr); this.operando(o.expr.tipo, tr); this.empilha(tr.k === 'real');
        this.i(`mov rax, qword ptr [rsp + 8]`, 'endereço do alvo');
        this.carrega(t); this.operando(t, tr);
        this.desempilha(tr.k === 'real' ? 'xmm1' : 'rcx');
        this.opCore(o.op, t, o.expr.tipo, tr, lin);
        this.coagir(tr, t, lin);
        if (t.k === 'real') this.desempilha('rax');
        else { this.i('mov rcx, rax'); this.desempilha('rax'); }
        this.guarda(t);
        this.carrega(t);
        return t;
      }
      case 'escrita': {
        for (const pt of o.partes) this.escreve(pt);
        if (!o.prompt) this.chama('lavra_nl@PLT');
        return null;
      }
      case 'leitura': {
        const t = o.tipo;
        const f = { int: 'i', real: 'r', car: 'c', texto: 't', bool: 'b' }[t.k];
        this.i(`mov edi, ${lin}`);
        this.chama(`lavra_le_${f}@PLT`);
        this.coagir(t.k === 'int' ? { k: 'int', bits: 64, sinal: true } : t.k === 'real' ? { k: 'real', bits: 64 } : t, t, lin);
        if (o.alvo) {
          const real = t.k === 'real';
          this.empilha(real); this.end(o.alvo); this.desempilha(real ? 'xmm0' : 'rcx');
          this.guarda(t); this.carrega(t);
        }
        return t;
      }
      case 'retorno': {
        const f = this.funcAtual;
        if (o.expr) {
          if (ehComp(f.retorno)) throw new ErroGerador('Esta versão do gerador x86 não devolve registros ou sequências por valor; devolva um endereço.');
          this.val(o.expr); this.coagir(o.expr.tipo, f.retorno, lin);
        }
        this.i(`jmp ${this.fRet}`);
        return null;
      }
      case 'chamada': this.chamada(o.f, o.args); return o.f.retorno;
      case 'reserva': {
        this.val(o.qtd);
        this.i('mov rdi, rax'); this.i(`mov esi, ${sizeOf(o.elem)}`, `bytes por ${descTipo(o.elem)}`); this.i(`mov edx, ${lin}`);
        this.chama('lavra_reserva@PLT');
        return { k: 'ptr', to: o.elem };
      }
      case 'libera':
        this.val(o.expr); this.i('mov rdi, rax'); this.i(`mov esi, ${lin}`);
        this.chama('lavra_libera@PLT');
        return null;
      case 'paro': this.i(`jmp ${this.lacos[this.lacos.length - 1].fim}`, 'paro'); return null;
      case 'pulo': this.i(`jmp ${this.lacos[this.lacos.length - 1].cont}`, 'pulo'); return null;
    }
    throw new ErroGerador('Oração sem tradução: ' + o.k);
  }
  escreve(e) {
    this.val(e);
    const t = e.tipo;
    switch (t.k) {
      case 'int': this.i('mov rdi, rax'); this.chama(t.sinal ? 'lavra_escreve_i@PLT' : 'lavra_escreve_u@PLT'); return;
      case 'real': this.chama('lavra_escreve_r@PLT'); return;
      case 'bool': this.i('mov rdi, rax'); this.chama('lavra_escreve_b@PLT'); return;
      case 'car': this.i('mov edi, eax'); this.chama('lavra_escreve_c@PLT'); return;
      case 'texto': this.i('mov rdi, rax'); this.chama('lavra_escreve_t@PLT'); return;
      case 'ptr': case 'nada': this.i('mov rdi, rax'); this.chama('lavra_escreve_p@PLT'); return;
      default:
        this.i('mov rdi, rax'); this.i(`lea rsi, [rip + ${this.texto(descritor(t))}]`, 'descrição do tipo');
        this.chama('lavra_escreve_bloco@PLT');
    }
  }
  chamada(f, args) {
    const ps = f.params;
    args.forEach((a, j) => {
      this.val(a); this.coagir(a.tipo, ps[j].tipo, 0);
      if (ps[j].tipo.k === 'real') this.i('movq rax, xmm0');
      this.i('push rax', `argumento ${ps[j].nome}`); this.pilha++;
    });
    let ni = 0, nf = 0;
    const regs = ps.map(p => p.tipo.k === 'real' ? 'xmm' + (nf++) : INT_REGS[ni++]);
    if (ni > 6 || nf > 8) throw new ErroGerador(`"${f.inf}" tem argumentos demais para os registradores.`);
    for (let j = ps.length - 1; j >= 0; j--) {
      if (regs[j].startsWith('xmm')) { this.i('pop rax'); this.i(`movq ${regs[j]}, rax`); }
      else this.i(`pop ${regs[j]}`);
      this.pilha--;
    }
    this.chama(this.rotFunc(f));
    if (f.retorno && f.retorno.k === 'bool') this.i('movzx eax, al');
  }

  // ---------- endereços, cargas e armazenamentos
  endVar(d) {
    if (d.alias) { this.i(`mov rax, qword ptr [rbp - ${this.q.slots.get(d.id)}]`, artNome(d)); return; }
    if (d.func) { this.i(`lea rax, [rbp - ${this.q.slots.get(d.id)}]`, artNome(d)); return; }
    this.global(d);
    this.i(`lea rax, [rip + ${this.gl.get(d.id).rot}]`, artNome(d));
  }
  end(e) {
    switch (e.k) {
      case 'var': this.endVar(e.d); return;
      case 'campo':
        this.end(e.base); layoutRec(e.base.tipo.r);
        if (e.c.offB) this.i(`add rax, ${e.c.offB}`, `campo ${e.c.nome}`);
        return;
      case 'indice': {
        const n = e.base.tipo.n, s = sizeOf(e.tipo);
        if (e.ordinal) {
          this.end(e.base);
          const k = Number(e.idx.v);
          if (k) this.i(`add rax, ${k * s}`, `${e.ordinal === -1 ? 'último' : e.ordinal + 'º'} item`);
          return;
        }
        this.val(e.idx);
        const ok = this.novo('dentro');
        this.i(`cmp rax, ${n}`); this.i(`jb ${ok}`, 'verificação de limite');
        this.i('mov rsi, rax'); this.i(`mov edi, ${e.tok.line}`); this.i(`mov edx, ${n}`);
        this.chama('lavra_falha_indice@PLT');
        this.rot(ok);
        if (s !== 1) this.i(`imul rax, rax, ${s}`);
        this.empilha(false); this.end(e.base); this.desempilha('rcx'); this.i('add rax, rcx');
        return;
      }
      case 'apontado': {
        this.val(e.e);
        const ok = this.novo('naonada');
        this.i('test rax, rax'); this.i(`jnz ${ok}`);
        this.falha(e.tok.line, 'Acesso ao apontado por nada.');
        this.rot(ok);
        return;
      }
    }
    throw new ErroGerador('Sem endereço: ' + e.k);
  }
  carrega(t, m, com) {
    m = m || '[rax]';
    switch (t.k) {
      case 'real': if (t.bits === 64) this.i(`movsd xmm0, qword ptr ${m}`, com); else this.i(`cvtss2sd xmm0, dword ptr ${m}`, com); return;
      case 'bool': this.i(`movzx eax, byte ptr ${m}`, com); return;
      case 'car': this.i(`mov eax, dword ptr ${m}`, com); return;
      case 'int':
        switch (t.bits) {
          case 64: this.i(`mov rax, qword ptr ${m}`, com); return;
          case 32: this.i(t.sinal ? `movsxd rax, dword ptr ${m}` : `mov eax, dword ptr ${m}`, com); return;
          case 16: this.i(t.sinal ? `movsx rax, word ptr ${m}` : `movzx eax, word ptr ${m}`, com); return;
          case 8: this.i(t.sinal ? `movsx rax, byte ptr ${m}` : `movzx eax, byte ptr ${m}`, com); return;
        }
        return;
      default: if (!ehComp(t)) this.i(`mov rax, qword ptr ${m}`, com);
    }
  }
  memDireta(e) { // operando de memória direto para variáveis simples (sem calcular endereço em rax)
    if (e.k !== 'var' || e.d.alias || ehComp(e.tipo)) return null;
    if (e.d.func) return `[rbp - ${this.q.slots.get(e.d.id)}]`;
    this.global(e.d);
    return `[rip + ${this.gl.get(e.d.id).rot}]`;
  }
  valComo(e, tr) { // valor já convertido para o tipo da operação; literais convertidos na compilação
    if (tr.k === 'real' && e.k === 'num' && e.tipo.k === 'int') { this.i(`movsd xmm0, qword ptr [rip + ${this.real(Number(e.v))}]`, `${e.v} (convertido na compilação)`); return; }
    this.val(e); this.operando(e.tipo, tr);
  }
  guarda(t) { // endereço em rax; valor em rcx (ou xmm0; ou endereço de origem em rcx para blocos)
    if (ehComp(t)) {
      this.i('mov rdi, rax'); this.i('mov rsi, rcx'); this.i(`mov edx, ${sizeOf(t)}`, 'cópia do bloco');
      this.chama('memcpy@PLT');
      return;
    }
    if (t.k === 'real') {
      if (t.bits === 64) this.i('movsd qword ptr [rax], xmm0');
      else { this.i('cvtsd2ss xmm1, xmm0'); this.i('movss dword ptr [rax], xmm1'); }
      return;
    }
    const n = sizeOf(t);
    this.i(`mov ${PTR[n]} ptr [rax], ${regTam('rcx', n)}`);
  }
  normaliza(t) {
    if (t.k !== 'int') return;
    switch (t.bits) {
      case 32: this.i(t.sinal ? 'movsxd rax, eax' : 'mov eax, eax', 'ajusta a 32 bits'); return;
      case 16: this.i(t.sinal ? 'movsx rax, ax' : 'movzx eax, ax', 'ajusta a 16 bits'); return;
      case 8: this.i(t.sinal ? 'movsx rax, al' : 'movzx eax, al', 'ajusta a 8 bits'); return;
    }
  }
  paraTexto(de) {
    switch (de.k) {
      case 'texto': return;
      case 'int': this.i('mov rdi, rax'); this.chama(de.sinal ? 'lavra_txt_i@PLT' : 'lavra_txt_u@PLT'); return;
      case 'real': this.chama('lavra_txt_r@PLT'); return;
      case 'bool': this.i('mov rdi, rax'); this.chama('lavra_txt_b@PLT'); return;
      case 'car': this.i('mov edi, eax'); this.chama('lavra_txt_c@PLT'); return;
      case 'ptr': case 'nada': this.i('mov rdi, rax'); this.chama('lavra_txt_p@PLT'); return;
      default: this.i('mov rdi, rax'); this.i(`lea rsi, [rip + ${this.texto(descritor(de))}]`); this.chama('lavra_txt_bloco@PLT');
    }
  }
  coagir(de, para, lin) {
    if (ehComp(para)) return;
    if (para.k === 'texto') { this.paraTexto(de); return; }
    if (de.k === 'texto') {
      const f = { int: 'i', real: 'r', car: 'c', bool: 'b' }[para.k];
      this.i('mov rdi, rax'); this.i(`mov esi, ${lin}`); this.chama(`lavra_txt_para_${f}@PLT`);
      if (para.k === 'int') this.normaliza(para);
      if (para.k === 'real' && para.bits === 32) { this.i('cvtsd2ss xmm0, xmm0'); this.i('cvtss2sd xmm0, xmm0'); }
      return;
    }
    switch (para.k) {
      case 'real':
        if (de.k !== 'real') this.i('cvtsi2sd xmm0, rax', 'inteiro → real');
        if (para.bits === 32 && !(de.k === 'real' && de.bits === 32)) { this.i('cvtsd2ss xmm0, xmm0', 'arredonda a 32 bits'); this.i('cvtss2sd xmm0, xmm0'); }
        return;
      case 'int':
        if (de.k === 'real') this.i('cvttsd2si rax, xmm0', 'real → inteiro (trunca)');
        if (!(de.k === 'int' && de.bits === para.bits && de.sinal === para.sinal)) this.normaliza(para);
        return;
      case 'car':
        if (de.k === 'real') this.i('cvttsd2si rax, xmm0');
        if (de.k !== 'car') this.i('mov eax, eax');
        return;
      case 'bool':
        if (de.k === 'bool') return;
        if (de.k === 'real') { this.i('xorpd xmm1, xmm1'); this.i('ucomisd xmm0, xmm1'); this.i('setne al'); this.i('setp cl'); this.i('or al, cl'); }
        else { this.i('test rax, rax'); this.i('setne al'); }
        this.i('movzx eax, al');
        return;
    }
  }
  operando(de, tr) { // prepara um operando para uma operação cujo resultado é tr
    if (tr.k === 'real' && de.k !== 'real') this.i('cvtsi2sd xmm0, rax', 'inteiro → real');
    if (tr.k === 'texto' && de.k !== 'texto') this.paraTexto(de);
  }

  // ---------- expressões (valor em rax, ou xmm0 para reais; blocos: endereço em rax)
  val(e) {
    switch (e.k) {
      case 'num':
        if (e.tipo.k === 'real') this.i(`movsd xmm0, qword ptr [rip + ${this.real(e.v)}]`, `${e.v}`.replace('.', ','));
        else this.i(`mov rax, ${BigInt.asIntN(64, e.v)}`);
        return;
      case 'texto': this.i(`lea rax, [rip + ${this.texto(e.v)}]`, JSON.stringify(e.v).slice(0, 30)); return;
      case 'car': this.i(`mov eax, ${e.v}`, `'${String.fromCodePoint(e.v)}'`); return;
      case 'logico': this.i(`mov eax, ${e.v ? 1 : 0}`, e.v ? 'verdadeiro' : 'falso'); return;
      case 'nada': this.i('xor eax, eax', 'nada'); return;
      case 'var': case 'campo': case 'indice': case 'apontado': {
        const m = this.memDireta(e);
        if (m) { this.carrega(e.tipo, m, artNome(e.d)); return; }
        this.end(e);
        this.carrega(e.tipo);
        return;
      }
      case 'bin': return this.bin(e);
      case 'un': return this.un(e);
      case 'conv': this.val(e.e); this.coagir(e.e.tipo, e.tipo, e.tok.line); return;
      case 'anafora':
        if (e.tipo.k === 'real') this.i(`movsd xmm0, qword ptr [rbp - ${this.q.res}]`, 'o resultado');
        else this.i(`mov rax, qword ptr [rbp - ${this.q.res}]`, 'o resultado');
        return;
      case 'chamada': this.chamada(e.f, e.args); return;
      case 'fato': this.cond(e.cond); return;
      case 'endereco': this.end(e.alvo); return;
    }
    throw new ErroGerador('Expressão sem tradução: ' + e.k);
  }
  bin(e) {
    const tr = e.tipo, ta = e.a.tipo, tb = e.b.tipo, lin = e.tok.line;
    if (tr.k === 'real') {
      this.valComo(e.b, tr); this.empilha(true);
      this.valComo(e.a, tr); this.desempilha('xmm1');
    } else if (tr.k === 'texto') {
      this.val(e.b); this.operando(tb, tr); this.empilha(false);
      this.val(e.a); this.operando(ta, tr); this.desempilha('rcx');
    } else {
      this.val(e.b); this.empilha(false);
      this.val(e.a); this.desempilha('rcx');
    }
    this.opCore(e.op, ta, tb, tr, lin);
  }
  opCore(op, ta, tb, tr, lin) { // a em rax/xmm0, b em rcx/xmm1
    if (tr.k === 'texto') { this.i('mov rdi, rax'); this.i('mov rsi, rcx'); this.chama('lavra_concatena@PLT'); return; }
    if (tr.k === 'ptr') {
      const s = sizeOf(tr.to);
      if (s !== 1) this.i(`imul rcx, rcx, ${s}`, `× tamanho de ${descTipo(tr.to)}`);
      this.i(op === '+' ? 'add rax, rcx' : 'sub rax, rcx');
      return;
    }
    if (ta.k === 'ptr' && tb.k === 'ptr') {
      this.i('sub rax, rcx');
      const s = sizeOf(ta.to);
      if (s !== 1) { this.i('cqo'); this.i(`mov rcx, ${s}`); this.i('idiv rcx'); }
      return;
    }
    if (tr.k === 'real') {
      switch (op) {
        case '+': this.i('addsd xmm0, xmm1'); break;
        case '-': this.i('subsd xmm0, xmm1'); break;
        case '*': this.i('mulsd xmm0, xmm1'); break;
        case '/': this.i('divsd xmm0, xmm1'); break;
        case '%': this.chama('fmod@PLT'); break;
        case '**': this.chama('pow@PLT'); break;
      }
      if (tr.bits === 32) { this.i('cvtsd2ss xmm0, xmm0'); this.i('cvtss2sd xmm0, xmm0'); }
      return;
    }
    switch (op) {
      case '+': this.i('add rax, rcx'); break;
      case '-': this.i('sub rax, rcx'); break;
      case '*': this.i('imul rax, rcx'); break;
      case '/': case '%': {
        const ok = this.novo('divok');
        this.i('test rcx, rcx'); this.i(`jnz ${ok}`);
        this.falha(lin, op === '/' ? 'Divisão inteira por zero.' : 'Resto de divisão por zero.');
        this.rot(ok);
        if (tr.sinal) { this.i('cqo'); this.i('idiv rcx'); } else { this.i('xor edx, edx'); this.i('div rcx'); }
        if (op === '%') this.i('mov rax, rdx', 'resto');
        break;
      }
      case '**': this.i('mov rdi, rax'); this.i('mov rsi, rcx'); this.i(`mov edx, ${lin}`); this.chama('lavra_pot_int@PLT'); break;
      case '&': this.i('and rax, rcx'); break;
      case '|': this.i('or rax, rcx'); break;
      case '^': this.i('xor rax, rcx'); break;
      case '<<': {
        const ok = this.novo('desl'), fim = this.novo('fimdesl');
        this.i(`cmp rcx, ${tr.bits}`); this.i(`jb ${ok}`); this.i('xor eax, eax'); this.i(`jmp ${fim}`);
        this.rot(ok); this.i('shl rax, cl'); this.rot(fim);
        break;
      }
      case '>>': {
        const ok = this.novo('desl'), fim = this.novo('fimdesl');
        this.i('cmp rcx, 63'); this.i(`jbe ${ok}`);
        if (tr.sinal) this.i('mov ecx, 63'); else { this.i('xor eax, eax'); this.i(`jmp ${fim}`); }
        this.rot(ok); this.i(tr.sinal ? 'sar rax, cl' : 'shr rax, cl'); this.rot(fim);
        break;
      }
    }
    this.normaliza(tr);
  }
  un(e) {
    switch (e.op) {
      case 'tam':
        if (e.a.tipo.k === 'seq') { this.i(`mov rax, ${e.a.tipo.n}`, 'tamanho conhecido na compilação'); return; }
        this.val(e.a); this.i('mov rdi, rax'); this.chama('lavra_tamanho_texto@PLT'); return;
      case 'raiz': {
        this.val(e.a); this.operando(e.a.tipo, { k: 'real', bits: 64 });
        const ok = this.novo('raizok');
        this.i('xorpd xmm1, xmm1'); this.i('ucomisd xmm0, xmm1'); this.i(`jae ${ok}`);
        this.falha(e.tok.line, 'Raiz de número negativo.');
        this.rot(ok); this.i('sqrtsd xmm0, xmm0');
        return;
      }
      case '~': this.val(e.a); this.i('not rax'); this.normaliza(e.tipo); return;
      case 'neg':
        this.val(e.a);
        if (e.tipo.k === 'real') { this.i('movq rax, xmm0'); this.i('btc rax, 63', 'troca o sinal'); this.i('movq xmm0, rax'); }
        else { this.i('neg rax'); this.normaliza(e.tipo); }
        return;
    }
  }

  // ---------- condições (resultado 0/1 em eax)
  cond(c) {
    switch (c.k) {
      case 'e': {
        const fim = this.novo('fime');
        this.cond(c.a); this.i('test eax, eax'); this.i(`jz ${fim}`);
        this.cond(c.b); this.rot(fim);
        return;
      }
      case 'ou': {
        const fim = this.novo('fimou');
        this.cond(c.a); this.i('test eax, eax'); this.i(`jnz ${fim}`);
        this.cond(c.b); this.rot(fim);
        return;
      }
      case 'pred':
        this.chamada(c.f, [c.sujeito, ...c.args]);
        if (c.neg) this.i('xor eax, 1', 'não');
        return;
      case 'cmp': {
        const ta = c.a.tipo, tb = c.b.tipo, op = c.op;
        if (ta.k === 'real' || tb.k === 'real') {
          const R = { k: 'real', bits: 64 };
          this.valComo(c.b, R); this.empilha(true);
          this.valComo(c.a, R); this.desempilha('xmm1');
          this.i('ucomisd xmm0, xmm1');
          switch (op) {
            case '>': this.i('seta al'); break;
            case '<': this.i('setb al'); break;
            case '>=': this.i('setae al'); break;
            case '<=': this.i('setbe al'); break;
            case '==': this.i('sete al'); this.i('setnp cl'); this.i('and al, cl'); break;
            case '!=': this.i('setne al'); this.i('setp cl'); this.i('or al, cl'); break;
          }
        } else if (ta.k === 'texto') {
          this.val(c.b); this.empilha(false);
          this.val(c.a); this.i('mov rdi, rax'); this.desempilha('rsi');
          this.chama('strcmp@PLT');
          this.i('cmp eax, 0');
          this.i({ '>': 'setg', '<': 'setl', '>=': 'setge', '<=': 'setle', '==': 'sete', '!=': 'setne' }[op] + ' al');
        } else {
          this.val(c.b); this.empilha(false);
          this.val(c.a); this.desempilha('rcx');
          this.i('cmp rax, rcx');
          const comSinal = (ta.k === 'int' && ta.sinal) || (tb.k === 'int' && tb.sinal);
          const S = { '>': 'setg', '<': 'setl', '>=': 'setge', '<=': 'setle', '==': 'sete', '!=': 'setne' };
          const U = { '>': 'seta', '<': 'setb', '>=': 'setae', '<=': 'setbe', '==': 'sete', '!=': 'setne' };
          this.i((comSinal ? S : U)[op] + ' al');
        }
        this.i('movzx eax, al');
        if (c.neg) this.i('xor eax, 1', 'não');
        return;
      }
    }
  }
}
function artNome(d) { return d.simbolo ? d.nome : `${{ m: { s: 'o', p: 'os' }, f: { s: 'a', p: 'as' } }[d.g][d.n]} ${d.nome}`; }
function artIndef(t) { return L.util.generoTipo(t) === 'f' ? 'uma' : 'um'; }
const CHAVES = ['a', 'b', 'e', 'base', 'idx', 'alvo', 'expr', 'valor', 'lista', 'partes', 'args', 'sujeito', 'cond', 'qtd', 'col'];
function temAnafora(n) {
  if (!n || typeof n !== 'object') return false;
  if (Array.isArray(n)) return n.some(temAnafora);
  if (n.k === 'anafora') return true;
  return CHAVES.some(c => n[c] && temAnafora(n[c]));
}

function gerarX86(prog, fonte) {
  try { return { ok: true, asm: new GeradorX86(prog, fonte).gerar() }; }
  catch (e) { if (e instanceof ErroGerador) return { ok: false, erro: e.message }; throw e; }
}
L.gerarX86 = gerarX86;
if (typeof module !== 'undefined' && module.exports) module.exports = gerarX86;
})(typeof window !== 'undefined' ? window : globalThis);
