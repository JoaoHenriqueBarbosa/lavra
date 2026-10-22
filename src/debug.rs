//! O depurador: pontos de parada, passos e o que se vê do programa parado. É o
//! acompanhante (`interp::Tracer`) do interpretador, que é a referência da língua.
//!
//! Ele não sabe de onde vêm os pedidos: recebe `Command`s por um canal e manda `Event`s
//! por um `Sink`. O adaptador DAP (`lavrac dap`) liga os dois ao editor, no mesmo processo
//! ou, com o terminal integrado, por um socket com outro processo (`lavrac debuggee`). Parado
//! num ponto, quem responde às consultas (pilha, escopos, variáveis) é a própria thread do
//! programa, com a pilha à vista: não há estado dividido entre duas threads.

use crate::Analysis;
use crate::hir::{self, Block, Stmt, StmtKind};
use crate::interp::{Fault, Tracer, Value, View, field_article, format_value};
use crate::span::{LineIndex, Span};
use crate::tables::Tables;
use crate::types::Ty;
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};

/// O que o editor pede ao programa.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
pub enum Command {
    /// As linhas com ponto de parada, contadas a partir de 1.
    SetBreakpoints {
        lines: Vec<u32>,
    },
    /// Parar nos erros de execução e antes de cada "Desisto".
    ExceptionFilters {
        errors: bool,
        failures: bool,
    },
    /// Começa a execução; com `stop_on_entry`, parando na primeira oração.
    Start {
        stop_on_entry: bool,
    },
    Pause,
    Continue,
    Next,
    StepIn,
    StepOut,
    Terminate,
    /// As consultas levam o número do pedido do editor, que volta com a resposta. O quadro
    /// é o da pilha, a partir de 1 a contar de baixo, como a pilha o mostra.
    StackTrace {
        seq: i64,
    },
    Scopes {
        seq: i64,
        frame: usize,
    },
    Variables {
        seq: i64,
        reference: u32,
    },
    Evaluate {
        seq: i64,
        text: String,
        frame: Option<usize>,
    },
}

/// O que o programa conta ao editor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum Event {
    /// Parou: "entry", "breakpoint", "step", "pause" ou "exception" (com a mensagem).
    Stopped { reason: String, text: Option<String> },
    /// A resposta a uma consulta, ou o motivo de não haver.
    Reply { seq: i64, body: Option<Json>, error: Option<String> },
    /// O que o programa escreveu ("stdout") ou a falha que o encerrou ("stderr").
    Output { text: String, category: String },
    /// O programa terminou, com o código de saída do processo.
    Exited { code: u8 },
}

/// Para onde vão os eventos. Também a saída do programa passa por aqui
/// (`OutputWriter`), por isso é dividido.
pub type Sink = Arc<Mutex<dyn FnMut(Event) + Send>>;

pub fn send(sink: &Sink, e: Event) {
    if let Ok(mut f) = sink.lock() {
        f(e);
    }
}

/// A saída do programa como eventos, na hora em que sai.
pub struct OutputWriter(pub Sink);

impl std::io::Write for OutputWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        send(&self.0, Event::Output { text: String::from_utf8_lossy(buf).into_owned(), category: "stdout".into() });
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// As linhas (a partir de 1) em que começa alguma oração: onde um ponto de parada pega.
pub fn statement_lines(prog: &hir::Program, text: &str) -> BTreeSet<u32> {
    fn walk(b: &Block, lines: &LineIndex, out: &mut BTreeSet<u32>) {
        for s in &b.0 {
            if !s.span.is_empty() {
                out.insert(lines.line(s.span.start as usize) as u32 + 1);
            }
            match &s.kind {
                StmtKind::If { arms, otherwise } => {
                    arms.iter().for_each(|(_, b)| walk(b, lines, out));
                    otherwise.iter().for_each(|b| walk(b, lines, out));
                }
                StmtKind::While { body, .. }
                | StmtKind::Repeat { body, .. }
                | StmtKind::ForRange { body, .. }
                | StmtKind::ForEach { body, .. } => walk(body, lines, out),
                StmtKind::Match { arms, otherwise, .. } => {
                    arms.iter().for_each(|a| walk(&a.body, lines, out));
                    otherwise.iter().for_each(|b| walk(b, lines, out));
                }
                _ => {}
            }
        }
    }
    let lines = LineIndex::new(text);
    let mut out = BTreeSet::new();
    walk(&prog.init.body, &lines, &mut out);
    for f in &prog.funcs {
        walk(&f.body, &lines, &mut out);
    }
    out
}

/// Onde um ponto de parada pedido numa linha pega: a própria linha, ou a próxima em que
/// começa uma oração.
pub fn place_breakpoint(lines: &BTreeSet<u32>, line: u32) -> Option<u32> {
    lines.range(line..).next().copied()
}

/// Como seguir depois de uma parada.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Run,
    Entry,
    Pause,
    /// Até a próxima linha neste quadro ou num de fora.
    Next {
        depth: usize,
        line: u32,
    },
    /// Até a próxima linha, em qualquer quadro.
    StepIn {
        depth: usize,
        line: u32,
    },
    /// Até voltar ao quadro de fora.
    StepOut {
        depth: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ScopeKind {
    Params,
    Locals,
    Constants,
}

/// O que uma referência de variável abre: um escopo de um quadro, ou um valor com partes.
enum Target {
    Scope { frame: usize, kind: ScopeKind },
    Value { value: Value, ty: Ty },
}

pub struct Debugger {
    commands: Receiver<Command>,
    sink: Sink,
    path: String,
    lines: LineIndex,
    text: String,
    /// As tabelas da análise: dizem que locais valem em cada ponto.
    tables: Tables,
    breakpoints: BTreeSet<u32>,
    errors: bool,
    failures: bool,
    mode: Mode,
    /// Onde a execução passou por último: profundidade da pilha e linha.
    last: Option<(usize, u32)>,
    /// Onde está o quadro de cima enquanto parado.
    at: Span,
    refs: Vec<Target>,
}

impl Debugger {
    pub fn new(analysis: &Analysis, path: &str, text: &str, commands: Receiver<Command>, sink: Sink) -> Debugger {
        Debugger {
            commands,
            sink,
            path: path.to_string(),
            lines: LineIndex::new(text),
            text: text.to_string(),
            tables: analysis.tables.clone(),
            breakpoints: BTreeSet::new(),
            errors: true,
            failures: false,
            mode: Mode::Run,
            last: None,
            at: Span::default(),
            refs: Vec::new(),
        }
    }

    /// Espera a ordem de começar, guardando os pontos de parada que vêm antes dela.
    /// Devolve `false` se a sessão acabou antes.
    pub fn wait_start(&mut self) -> bool {
        loop {
            match self.commands.recv() {
                Ok(Command::Start { stop_on_entry }) => {
                    if stop_on_entry {
                        self.mode = Mode::Entry;
                    }
                    return true;
                }
                Ok(Command::Terminate) | Err(_) => return false,
                Ok(other) => {
                    if !self.while_running(other) {
                        return false;
                    }
                }
            }
        }
    }

    fn line(&self, offset: u32) -> u32 {
        self.lines.line(offset as usize) as u32 + 1
    }

    fn reply(&self, seq: i64, body: Result<Json, String>) {
        let (body, error) = match body {
            Ok(b) => (Some(b), None),
            Err(e) => (None, Some(e)),
        };
        send(&self.sink, Event::Reply { seq, body, error });
    }

    /// Um pedido que chegou com o programa andando. Devolve `false` para encerrar.
    fn while_running(&mut self, c: Command) -> bool {
        match c {
            Command::SetBreakpoints { lines } => self.breakpoints = lines.into_iter().collect(),
            Command::ExceptionFilters { errors, failures } => (self.errors, self.failures) = (errors, failures),
            Command::Pause => self.mode = Mode::Pause,
            Command::Terminate => return false,
            Command::StackTrace { seq } | Command::Scopes { seq, .. } | Command::Variables { seq, .. } | Command::Evaluate { seq, .. } => {
                self.reply(seq, Err("O programa está andando.".into()));
            }
            Command::Start { .. } | Command::Continue | Command::Next | Command::StepIn | Command::StepOut => {}
        }
        true
    }

    /// Parado: conta ao editor e responde às consultas até a ordem de seguir. Devolve
    /// `false` para encerrar.
    fn pause(&mut self, reason: &str, text: Option<String>, at: Span, view: &View) -> bool {
        self.refs.clear();
        self.at = at;
        let depth = view.frames.len();
        let line = self.line(at.start);
        send(&self.sink, Event::Stopped { reason: reason.into(), text });
        loop {
            let Ok(c) = self.commands.recv() else { return false };
            match c {
                Command::StackTrace { seq } => self.reply(seq, Ok(self.stack(view))),
                Command::Scopes { seq, frame } => {
                    let r = self.scopes(view, frame.wrapping_sub(1));
                    self.reply(seq, r);
                }
                Command::Variables { seq, reference } => {
                    let r = self.variables(view, reference);
                    self.reply(seq, r);
                }
                Command::Evaluate { seq, text, frame } => {
                    let r = self.evaluate(view, &text, frame.map_or(depth.saturating_sub(1), |f| f.wrapping_sub(1)));
                    self.reply(seq, r);
                }
                Command::Continue => {
                    self.mode = Mode::Run;
                    return true;
                }
                Command::Next => {
                    self.mode = Mode::Next { depth, line };
                    return true;
                }
                Command::StepIn => {
                    self.mode = Mode::StepIn { depth, line };
                    return true;
                }
                Command::StepOut => {
                    self.mode = Mode::StepOut { depth };
                    return true;
                }
                Command::Pause | Command::Start { .. } => {}
                other => {
                    if !self.while_running(other) {
                        return false;
                    }
                }
            }
        }
    }

    /// Onde está o quadro `i`: o de cima na oração em que parou; os outros, na chamada ao
    /// de cima deles.
    fn frame_at(&self, view: &View, i: usize) -> Span {
        if i + 1 >= view.frames.len() { self.at } else { view.frames[i + 1].call_span }
    }

    fn stack(&self, view: &View) -> Json {
        let n = view.frames.len();
        let name = std::path::Path::new(&self.path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let frames: Vec<Json> = (0..n)
            .rev()
            .map(|i| {
                let at = self.frame_at(view, i);
                let (line, col) = self.lines.line_col(&self.text, at.start as usize);
                let label = match view.frames[i].func {
                    Some(f) => format!("Para {}", view.prog.funcs[f].name),
                    None => "as constantes".to_string(),
                };
                json!({
                    // A partir de 1: o editor, em JavaScript, trata o 0 como "nenhum".
                    "id": i + 1,
                    "name": label,
                    "line": line,
                    "column": col,
                    "source": { "path": self.path, "name": name },
                })
            })
            .collect();
        json!({ "stackFrames": frames, "totalFrames": n })
    }

    fn alloc(&mut self, t: Target) -> u32 {
        self.refs.push(t);
        self.refs.len() as u32
    }

    fn scopes(&mut self, view: &View, frame: usize) -> Result<Json, String> {
        let f = view.frames.get(frame).ok_or("Esse quadro não existe.")?;
        let mut out = Vec::new();
        if let Some(id) = f.func {
            if !view.prog.funcs[id].params.is_empty() {
                out.push(("Parâmetros", ScopeKind::Params, "arguments"));
            }
            out.push(("Locais", ScopeKind::Locals, "locals"));
        }
        if !view.prog.globals.is_empty() {
            out.push(("Constantes", ScopeKind::Constants, "globals"));
        }
        let scopes: Vec<Json> = out
            .into_iter()
            .map(|(name, kind, hint)| {
                let reference = self.alloc(Target::Scope { frame, kind });
                json!({ "name": name, "presentationHint": hint, "variablesReference": reference, "expensive": false })
            })
            .collect();
        Ok(json!({ "scopes": scopes }))
    }

    /// O local vale neste ponto do quadro (declarado antes, num escopo em volta)?
    fn visible(&self, local: &hir::Local, at: u32) -> bool {
        match self.tables.defs.iter().position(|d| d.span == local.span) {
            Some(def) => self.tables.visible_at(at).contains(&def),
            None => local.span.start <= at,
        }
    }

    /// Os nomes de um escopo: (nome com artigo, valor, tipo).
    fn scope_values<'v>(&self, view: &'v View, frame: usize, kind: ScopeKind) -> Vec<(String, &'v Value, &'v Ty)> {
        let f = &view.frames[frame];
        let func = f.func.map(|id| &view.prog.funcs[id]);
        let at = self.frame_at(view, frame).start;
        match (kind, func) {
            (ScopeKind::Params, Some(func)) => {
                func.params.iter().map(|(id, _)| (func.locals[*id].display.clone(), &f.values[*id], &func.locals[*id].ty)).collect()
            }
            (ScopeKind::Locals, Some(func)) => func
                .locals
                .iter()
                .enumerate()
                .filter(|(id, l)| !l.hidden && !func.params.iter().any(|(p, _)| p == id) && f.values[*id] != Value::Unit)
                .filter(|(_, l)| self.visible(l, at))
                .map(|(id, l)| (l.display.clone(), &f.values[id], &l.ty))
                .collect(),
            (ScopeKind::Constants, _) => view
                .prog
                .globals
                .iter()
                .zip(view.globals)
                .filter(|(_, v)| **v != Value::Unit)
                .map(|(g, v)| (g.display.clone(), v, &g.ty))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Uma variável no formato do protocolo; com partes (lista, registro, caso), ganha
    /// uma referência para abrir.
    fn variable(&mut self, name: String, value: &Value, ty: &Ty) -> Json {
        let (value, ty) = unwrap_maybe(value, ty);
        let has_parts = match (value, ty) {
            (Value::List(items), Ty::List(_)) => !items.is_empty(),
            (Value::Record(_), Ty::Record(_)) => true,
            (Value::Variant(_, fields), Ty::Enum(_)) => !fields.is_empty(),
            _ => false,
        };
        let reference = if has_parts { self.alloc(Target::Value { value: value.clone(), ty: ty.clone() }) } else { 0 };
        json!({ "name": name, "value": summary(value, ty), "type": ty.describe(), "variablesReference": reference })
    }

    fn variables(&mut self, view: &View, reference: u32) -> Result<Json, String> {
        let target = self.refs.get(reference.wrapping_sub(1) as usize).ok_or("Essa referência não existe mais.")?;
        let parts: Vec<(String, Value, Ty)> = match target {
            Target::Scope { frame, kind } => {
                let (frame, kind) = (*frame, *kind);
                self.scope_values(view, frame, kind).into_iter().map(|(n, v, t)| (n, v.clone(), t.clone())).collect()
            }
            Target::Value { value, ty } => parts(value, ty),
        };
        let vars: Vec<Json> = parts.into_iter().map(|(n, v, t)| self.variable(n, &v, &t)).collect();
        Ok(json!({ "variables": vars }))
    }

    /// Um nome do quadro ("o saldo", "saldo"), ou uma constante.
    fn evaluate(&mut self, view: &View, text: &str, frame: usize) -> Result<Json, String> {
        let wanted = text.trim().to_lowercase();
        let word = wanted.split_once(' ').filter(|(a, _)| crate::morphology::article(a).is_some()).map_or(wanted.as_str(), |(_, w)| w);
        let mut found = None;
        if frame < view.frames.len() {
            for kind in [ScopeKind::Params, ScopeKind::Locals, ScopeKind::Constants] {
                let hit = self
                    .scope_values(view, frame, kind)
                    .into_iter()
                    .find(|(name, _, _)| name == &wanted || name.rsplit(' ').next() == Some(word));
                if let Some((name, v, t)) = hit {
                    found = Some((name, v.clone(), t.clone()));
                    break;
                }
            }
        }
        let (name, v, t) = found.ok_or_else(|| format!("\"{}\" não é um nome que vale aqui.", text.trim()))?;
        let var = self.variable(name, &v, &t);
        Ok(json!({ "result": var["value"], "type": var["type"], "variablesReference": var["variablesReference"] }))
    }
}

/// Roda o programa sob o depurador, na thread atual: espera a ordem de começar, executa,
/// e conta como terminou (a falha vai como o `lavrac run` a mostra). A saída vai para
/// `out`, a entrada vem de `input`. Devolve o código de saída do processo.
pub fn run(
    analysis: Analysis,
    path: &str,
    text: &str,
    commands: Receiver<Command>,
    sink: Sink,
    out: impl std::io::Write,
    input: impl std::io::BufRead,
) -> u8 {
    let mut debugger = Debugger::new(&analysis, path, text, commands, sink.clone());
    if !debugger.wait_start() {
        send(&sink, Event::Exited { code: 0 });
        return 0;
    }
    let prog = analysis.hir;
    let mut m = crate::interp::Machine::new(&prog, out, input);
    m.trace(Box::new(debugger));
    let result = m.run();
    let code = match result {
        Err(f) if !m.stopped() => {
            let src = crate::span::SourceFile::new(path, text.to_string());
            let shown = crate::diagnostics::render(&f.diagnostic(), &src, crate::diagnostics::Style { color: false });
            send(&sink, Event::Output { text: shown, category: "stderr".into() });
            f.exit_code()
        }
        _ => 0,
    };
    send(&sink, Event::Exited { code });
    code
}

/// Uma sessão no mesmo processo: o programa roda numa thread própria, com a pilha grande
/// que a recursão da Lavra pede; os comandos entram pelo canal devolvido.
pub fn spawn(
    analysis: Analysis,
    path: String,
    text: String,
    sink: Sink,
    input: Box<dyn std::io::BufRead + Send>,
) -> std::io::Result<(std::sync::mpsc::Sender<Command>, std::thread::JoinHandle<()>)> {
    let (tx, rx) = std::sync::mpsc::channel();
    let out = OutputWriter(sink.clone());
    let handle = std::thread::Builder::new().name("programa".into()).stack_size(1 << 30).spawn(move || {
        run(analysis, &path, &text, rx, sink, out, input);
    })?;
    Ok((tx, handle))
}

/// O "talvez" presente é o próprio valor.
fn unwrap_maybe<'a>(value: &'a Value, ty: &'a Ty) -> (&'a Value, &'a Ty) {
    match (value, ty) {
        (Value::Nothing, _) => (value, ty),
        (v, Ty::Maybe(inner)) => unwrap_maybe(v, inner),
        _ => (value, ty),
    }
}

/// O valor numa linha: o texto entre aspas, o resto como o programa o escreveria, cortado
/// se for comprido.
fn summary(value: &Value, ty: &Ty) -> String {
    let s = match value {
        Value::Text(t) => format!("\"{t}\""),
        Value::Char(c) => format!("'{c}'"),
        Value::Nothing => "nada".into(),
        v => format_value(v, ty),
    };
    if s.chars().count() > 120 { format!("{}...", s.chars().take(117).collect::<String>()) } else { s }
}

/// As partes de um valor: os itens de uma lista ("o item 1"), os campos de um registro ou
/// de um caso ("o saldo").
fn parts(value: &Value, ty: &Ty) -> Vec<(String, Value, Ty)> {
    match (value, ty) {
        (Value::List(items), Ty::List(elem)) => {
            items.iter().enumerate().map(|(i, v)| (format!("o item {}", i + 1), v.clone(), (**elem).clone())).collect()
        }
        (Value::Record(fields), Ty::Record(r)) => fields
            .iter()
            .zip(&r.fields)
            .map(|(v, (name, t))| (format!("{} {name}", field_article(name, r)), v.clone(), t.clone()))
            .collect(),
        (Value::Variant(tag, fields), Ty::Enum(e)) => {
            let var = &e.variants[*tag];
            fields
                .iter()
                .zip(&var.fields)
                .zip(&var.field_articles)
                .map(|((v, (name, t)), (_, art))| (format!("{art} {name}"), v.clone(), t.clone()))
                .collect()
        }
        _ => Vec::new(),
    }
}

impl Tracer for Debugger {
    fn before(&mut self, stmt: &Stmt, view: &View) -> bool {
        // Os pedidos que chegaram com o programa andando.
        loop {
            match self.commands.try_recv() {
                Ok(c) => {
                    if !self.while_running(c) {
                        return false;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
        let depth = view.frames.len();
        let line = self.line(stmt.span.start);
        // Chegar a uma linha é vir de outra (ou de outro quadro): as orações seguintes da
        // mesma linha, e as voltas de um laço escrito numa linha só, não param de novo.
        let arrived = self.last != Some((depth, line));
        self.last = Some((depth, line));
        let reason = match self.mode {
            Mode::Entry => Some("entry"),
            Mode::Pause => Some("pause"),
            Mode::Next { depth: d, line: l } if depth < d || (depth == d && line != l) => Some("step"),
            Mode::StepIn { depth: d, line: l } if depth != d || line != l => Some("step"),
            Mode::StepOut { depth: d } if depth < d => Some("step"),
            _ if arrived && self.breakpoints.contains(&line) => Some("breakpoint"),
            _ => None,
        };
        if let Some(reason) = reason {
            return self.pause(reason, None, stmt.span, view);
        }
        if self.failures && matches!(stmt.kind, StmtKind::Desist(_)) {
            return self.pause("exception", Some("O programa vai desistir aqui.".into()), stmt.span, view);
        }
        true
    }

    fn fault(&mut self, fault: &Fault, view: &View) {
        if self.errors {
            let _ = self.pause("exception", Some(fault.message.clone()), fault.span, view);
        }
    }
}
