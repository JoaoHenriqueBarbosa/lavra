//! O adaptador de depuração (`lavrac dap`): fala o Debug Adapter Protocol com o editor
//! pela entrada e saída padrão e liga o depurador (`debug`) ao programa.
//!
//! O programa roda no mesmo processo, numa thread, com a saída indo para o console de
//! depuração e a entrada vindo de um arquivo (`"stdin"` na configuração). Com
//! `"console": "integratedTerminal"` (o padrão quando o programa lê a entrada e o editor
//! sabe abrir terminal), o editor abre um terminal com `lavrac debuggee`, que liga de volta
//! por um socket local: o programa lê e escreve no terminal, e o depurador conversa pelo
//! socket, uma mensagem JSON por linha.

use crate::Analysis;
use crate::debug::{self, Command, Event, Sink};
use crate::diagnostics::{Style, render};
use crate::span::SourceFile;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Lê uma mensagem do protocolo: o cabeçalho `Content-Length` e o JSON.
pub fn read_message(r: &mut impl BufRead) -> std::io::Result<Option<Value>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            if len.is_some() {
                break;
            }
            continue;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let mut buf = vec![0; len.unwrap_or(0)];
    r.read_exact(&mut buf)?;
    Ok(serde_json::from_slice(&buf).ok())
}

/// Escreve uma mensagem do protocolo.
pub fn write_message(w: &mut impl Write, msg: &Value) -> std::io::Result<()> {
    let body = msg.to_string();
    write!(w, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    w.flush()
}

/// Quem escreve para o editor: respostas, eventos e pedidos ao contrário
/// (`runInTerminal`), numerados em sequência, de qualquer thread.
#[derive(Clone)]
pub struct Out(Arc<Mutex<(Box<dyn Write + Send>, i64)>>);

impl Out {
    pub fn new(w: Box<dyn Write + Send>) -> Out {
        Out(Arc::new(Mutex::new((w, 0))))
    }

    fn send(&self, mut msg: Value) -> i64 {
        let Ok(mut g) = self.0.lock() else { return 0 };
        g.1 += 1;
        msg["seq"] = json!(g.1);
        let _ = write_message(&mut g.0, &msg);
        g.1
    }

    fn event(&self, event: &str, body: Value) {
        self.send(json!({ "type": "event", "event": event, "body": body }));
    }

    fn respond(&self, seq: i64, command: &str, body: Result<Value, String>) {
        let msg = match body {
            Ok(body) => json!({ "type": "response", "request_seq": seq, "command": command, "success": true, "body": body }),
            Err(m) => json!({
                "type": "response", "request_seq": seq, "command": command, "success": false, "message": m,
                "body": { "error": { "id": 1, "format": m, "showUser": true } },
            }),
        };
        self.send(msg);
    }

    fn output(&self, category: &str, text: &str) {
        self.event("output", json!({ "category": category, "output": text }));
    }
}

/// As consultas mandadas ao programa, esperando a resposta: o número do pedido e o nome
/// do comando, que a resposta repete.
type Pending = Arc<Mutex<HashMap<i64, String>>>;

/// O que o programa conta vira mensagem do protocolo.
fn forward(out: &Out, pending: &Pending, e: Event) {
    match e {
        Event::Stopped { reason, text } => {
            let description = match reason.as_str() {
                "entry" => "Parado na entrada",
                "breakpoint" => "Parado no ponto de parada",
                "step" => "Parado depois do passo",
                "pause" => "Pausado",
                _ => "Parado na falha",
            };
            // O protocolo prevê o campo ausente, nunca nulo.
            let mut body = json!({ "reason": reason, "description": description, "threadId": 1, "allThreadsStopped": true });
            if let Some(text) = text {
                body["text"] = json!(text);
            }
            out.event("stopped", body);
        }
        Event::Reply { seq, body, error } => {
            let command = pending.lock().ok().and_then(|mut p| p.remove(&seq)).unwrap_or_default();
            out.respond(seq, &command, body.ok_or_else(|| error.unwrap_or_default()));
        }
        Event::Output { text, category } => out.output(&category, &text),
        Event::Exited { code } => {
            out.event("exited", json!({ "exitCode": code }));
            out.event("terminated", json!({}));
        }
    }
}

/// Um programa lido e analisado, pronto para começar.
struct Program {
    path: String,
    text: String,
    analysis: Analysis,
    lines: BTreeSet<u32>,
}

fn load(path: &str) -> Result<Program, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Não consegui ler {path}: {e}"))?;
    let analysis = crate::analyze(&text);
    let lines = debug::statement_lines(&analysis.hir, &text);
    Ok(Program { path: path.to_string(), text, analysis, lines })
}

/// O que o launch pediu.
struct Launch {
    program: Program,
    stdin: Option<String>,
    stop_on_entry: bool,
    terminal: bool,
}

struct Adapter {
    out: Out,
    pending: Pending,
    /// O editor sabe abrir um terminal para o programa (`runInTerminal`).
    terminal: bool,
    launch: Option<Launch>,
    configured: bool,
    /// As linhas pedidas, por arquivo; valem as do programa lançado.
    breakpoints: HashMap<String, Vec<u32>>,
    filters: (bool, bool),
    /// O canal para o programa, depois que ele começou.
    program: Option<Sender<Command>>,
}

impl Adapter {
    fn send(&self, c: Command) {
        if let Some(p) = &self.program {
            let _ = p.send(c);
        }
    }

    /// Uma consulta ao programa parado; a resposta volta pelo `forward`.
    fn ask(&self, req: &Value, make: impl FnOnce(i64) -> Command) {
        let seq = req["seq"].as_i64().unwrap_or(0);
        let command = req["command"].as_str().unwrap_or_default();
        match &self.program {
            Some(p) => {
                if let Ok(mut pend) = self.pending.lock() {
                    pend.insert(seq, command.to_string());
                }
                let _ = p.send(make(seq));
            }
            None => self.out.respond(seq, command, Err("O programa não está rodando.".into())),
        }
    }

    /// Os pontos de parada do programa lançado, onde eles pegam.
    fn program_breakpoints(&self) -> Vec<u32> {
        let Some(l) = &self.launch else { return Vec::new() };
        let asked = self.breakpoints.get(&l.program.path).cloned().unwrap_or_default();
        asked.into_iter().filter_map(|line| debug::place_breakpoint(&l.program.lines, line)).collect()
    }

    /// Começa o programa: no mesmo processo, ou no terminal do editor.
    fn start(&mut self) {
        let Some(l) = &self.launch else { return };
        if self.program.is_some() {
            return;
        }
        let (path, text, stdin, stop_on_entry, terminal) =
            (l.program.path.clone(), l.program.text.clone(), l.stdin.clone(), l.stop_on_entry, l.terminal);
        let (out, pending) = (self.out.clone(), self.pending.clone());
        let sink: Sink = Arc::new(Mutex::new(move |e| forward(&out, &pending, e)));
        let first = [
            Command::SetBreakpoints { lines: self.program_breakpoints() },
            Command::ExceptionFilters { errors: self.filters.0, failures: self.filters.1 },
            Command::Start { stop_on_entry },
        ];
        let started = if terminal {
            self.start_in_terminal(&path, sink)
        } else {
            let input: Result<Box<dyn BufRead + Send>, String> = match &stdin {
                Some(f) => std::fs::File::open(f)
                    .map(|file| Box::new(BufReader::new(file)) as Box<dyn BufRead + Send>)
                    .map_err(|e| format!("Não consegui abrir a entrada {f}: {e}")),
                None => Ok(Box::new(std::io::Cursor::new(Vec::new()))),
            };
            input.and_then(|input| {
                debug::spawn(crate::analyze(&text), path, text, sink, input)
                    .map(|(tx, _)| tx)
                    .map_err(|e| format!("Não consegui começar o programa: {e}"))
            })
        };
        match started {
            Ok(tx) => {
                for c in first {
                    let _ = tx.send(c);
                }
                self.program = Some(tx);
            }
            Err(e) => self.fail_start(&e),
        }
    }

    fn fail_start(&self, message: &str) {
        self.out.output("stderr", &format!("{message}\n"));
        self.out.event("terminated", json!({}));
    }

    /// Pede ao editor um terminal com o `lavrac debuggee`, que liga de volta pelo socket.
    /// Os comandos esperam num canal até a ligação.
    fn start_in_terminal(&mut self, path: &str, sink: Sink) -> Result<Sender<Command>, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Não consegui abrir o socket local: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let cwd = std::path::Path::new(path).parent().map(|p| p.display().to_string()).unwrap_or_default();
        let name = std::path::Path::new(path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        self.out.send(json!({
            "type": "request",
            "command": "runInTerminal",
            "arguments": {
                "kind": "integrated",
                "title": format!("Lavra: {name}"),
                "cwd": cwd,
                "args": [exe.display().to_string(), "debuggee", "--port", port.to_string(), path],
            },
        }));
        let (tx, rx) = channel::<Command>();
        let out = self.out.clone();
        std::thread::spawn(move || {
            let stream = match accept_within(&listener, Duration::from_secs(60)) {
                Some(s) => s,
                None => {
                    out.output("stderr", "O terminal não ligou de volta ao depurador.\n");
                    out.event("terminated", json!({}));
                    return;
                }
            };
            let Ok(reader) = stream.try_clone() else { return };
            // Do programa para o editor.
            std::thread::spawn(move || {
                for line in BufReader::new(reader).lines().map_while(Result::ok) {
                    if let Ok(e) = serde_json::from_str::<Event>(&line) {
                        debug::send(&sink, e);
                    }
                }
            });
            // Do editor para o programa.
            let mut writer = stream;
            for c in rx {
                let Ok(line) = serde_json::to_string(&c) else { continue };
                if writeln!(writer, "{line}").is_err() {
                    break;
                }
            }
        });
        Ok(tx)
    }

    /// Um pedido do editor. Devolve `false` quando a sessão acabou.
    fn request(&mut self, req: &Value) -> bool {
        let seq = req["seq"].as_i64().unwrap_or(0);
        let command = req["command"].as_str().unwrap_or_default().to_string();
        let args = &req["arguments"];
        let respond = |body: Result<Value, String>| self.out.respond(seq, &command, body);
        match command.as_str() {
            "initialize" => {
                self.terminal = args["supportsRunInTerminalRequest"].as_bool().unwrap_or(false);
                respond(Ok(json!({
                    "supportsConfigurationDoneRequest": true,
                    "supportsEvaluateForHovers": true,
                    "supportsTerminateRequest": true,
                    "exceptionBreakpointFilters": [
                        { "filter": "erros", "label": "Erros de execução", "default": true },
                        { "filter": "falhas", "label": "Falhas declaradas (\"Desisto\")", "default": false },
                    ],
                })));
                self.out.event("initialized", json!({}));
            }
            "launch" => {
                let Some(path) = args["program"].as_str() else {
                    respond(Err("Falta o \"program\": o arquivo .lav a depurar.".into()));
                    return true;
                };
                let program = match load(path) {
                    Ok(p) => p,
                    Err(e) => {
                        respond(Err(e));
                        return true;
                    }
                };
                if program.analysis.has_errors() {
                    let src = SourceFile::new(path, program.text.clone());
                    for d in program.analysis.diagnostics.iter().filter(|d| d.is_error()) {
                        self.out.output("stderr", &render(d, &src, Style { color: false }));
                    }
                    respond(Err("O programa tem erros: veja o console de depuração.".into()));
                    return true;
                }
                let stdin = args["stdin"].as_str().map(str::to_string);
                // O programa que lê a entrada vai para o terminal, se ninguém disse onde.
                let reads = program.analysis.tokens.iter().any(|t| t.is_word("leio"));
                let terminal = match args["console"].as_str() {
                    Some("integratedTerminal") => self.terminal,
                    Some(_) => false,
                    None => self.terminal && reads && stdin.is_none(),
                };
                self.launch = Some(Launch { program, stdin, stop_on_entry: args["stopOnEntry"].as_bool().unwrap_or(false), terminal });
                respond(Ok(json!({})));
                if self.configured {
                    self.start();
                }
            }
            "setBreakpoints" => {
                let path = args["source"]["path"].as_str().unwrap_or_default().to_string();
                let asked: Vec<u32> =
                    args["breakpoints"].as_array().into_iter().flatten().filter_map(|b| b["line"].as_u64()).map(|l| l as u32).collect();
                let lines = match &self.launch {
                    Some(l) if l.program.path == path => Some(l.program.lines.clone()),
                    _ => load(&path).ok().map(|p| p.lines),
                };
                let placed: Vec<Value> = asked
                    .iter()
                    .map(|&line| match lines.as_ref().and_then(|ls| debug::place_breakpoint(ls, line)) {
                        Some(at) => json!({ "verified": true, "line": at }),
                        None => json!({ "verified": false, "line": line, "message": "Nenhuma oração começa aqui ou depois." }),
                    })
                    .collect();
                self.breakpoints.insert(path, asked);
                respond(Ok(json!({ "breakpoints": placed })));
                let lines = self.program_breakpoints();
                self.send(Command::SetBreakpoints { lines });
            }
            "setExceptionBreakpoints" => {
                let filters: Vec<&str> = args["filters"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
                self.filters = (filters.contains(&"erros"), filters.contains(&"falhas"));
                respond(Ok(json!({})));
                self.send(Command::ExceptionFilters { errors: self.filters.0, failures: self.filters.1 });
            }
            "configurationDone" => {
                respond(Ok(json!({})));
                self.configured = true;
                if self.launch.is_some() {
                    self.start();
                }
            }
            "threads" => respond(Ok(json!({ "threads": [{ "id": 1, "name": "programa" }] }))),
            "stackTrace" => self.ask(req, |seq| Command::StackTrace { seq }),
            "scopes" => {
                let frame = args["frameId"].as_u64().unwrap_or(0) as usize;
                self.ask(req, |seq| Command::Scopes { seq, frame });
            }
            "variables" => {
                let reference = args["variablesReference"].as_u64().unwrap_or(0) as u32;
                self.ask(req, |seq| Command::Variables { seq, reference });
            }
            "evaluate" => {
                let text = args["expression"].as_str().unwrap_or_default().to_string();
                let frame = args["frameId"].as_u64().map(|f| f as usize);
                self.ask(req, |seq| Command::Evaluate { seq, text, frame });
            }
            "continue" => {
                respond(Ok(json!({ "allThreadsContinued": true })));
                self.send(Command::Continue);
            }
            "next" | "stepIn" | "stepOut" => {
                respond(Ok(json!({})));
                self.send(match command.as_str() {
                    "next" => Command::Next,
                    "stepIn" => Command::StepIn,
                    _ => Command::StepOut,
                });
            }
            "pause" => {
                respond(Ok(json!({})));
                self.send(Command::Pause);
            }
            "terminate" => {
                respond(Ok(json!({})));
                self.send(Command::Terminate);
            }
            "disconnect" => {
                self.send(Command::Terminate);
                respond(Ok(json!({})));
                return false;
            }
            other => respond(Err(format!("O depurador da Lavra não conhece o pedido \"{other}\"."))),
        }
        true
    }
}

/// Espera a ligação no socket por até `limit`.
fn accept_within(listener: &TcpListener, limit: Duration) -> Option<TcpStream> {
    listener.set_nonblocking(true).ok()?;
    let end = std::time::Instant::now() + limit;
    while std::time::Instant::now() < end {
        match listener.accept() {
            Ok((s, _)) => {
                s.set_nonblocking(false).ok()?;
                return Some(s);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => return None,
        }
    }
    None
}

/// `lavrac dap`: o adaptador sobre a entrada e a saída padrão.
pub fn run_stdio() -> std::io::Result<()> {
    run(&mut BufReader::new(std::io::stdin().lock()), Box::new(std::io::stdout()))
}

/// O adaptador sobre um leitor e um escritor quaisquer.
pub fn run(input: &mut impl BufRead, output: Box<dyn Write + Send>) -> std::io::Result<()> {
    let mut adapter = Adapter {
        out: Out::new(output),
        pending: Arc::new(Mutex::new(HashMap::new())),
        terminal: false,
        launch: None,
        configured: false,
        breakpoints: HashMap::new(),
        filters: (true, false),
        program: None,
    };
    while let Some(msg) = read_message(input)? {
        // As respostas do editor (ao `runInTerminal`) não pedem nada de volta.
        if msg["type"] == "request" && !adapter.request(&msg) {
            break;
        }
    }
    adapter.send(Command::Terminate);
    Ok(())
}

/// `lavrac debuggee --port N arquivo.lav`: o programa no terminal que o editor abriu, ligado
/// ao adaptador pelo socket local. Lê e escreve no terminal; os comandos e os eventos do
/// depurador vão pelo socket. Devolve o código de saída.
pub fn debuggee(port: u16, path: &str) -> u8 {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("lavrac: não consegui ler {path}: {e}");
            return 2;
        }
    };
    let analysis = crate::analyze(&text);
    let stream = match TcpStream::connect(("127.0.0.1", port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("lavrac: não consegui ligar ao depurador: {e}");
            return 2;
        }
    };
    let (tx, rx) = channel::<Command>();
    let Ok(reader) = stream.try_clone() else { return 2 };
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            if let Ok(c) = serde_json::from_str::<Command>(&line)
                && tx.send(c).is_err()
            {
                break;
            }
        }
    });
    let writer = Mutex::new(stream);
    let sink: Sink = Arc::new(Mutex::new(move |e: Event| {
        // A falha que encerra o programa aparece também no terminal.
        if let Event::Output { text, category } = &e
            && category == "stderr"
        {
            eprint!("{text}");
        }
        if let (Ok(line), Ok(mut w)) = (serde_json::to_string(&e), writer.lock()) {
            let _ = writeln!(w, "{line}");
        }
    }));
    let path = path.to_string();
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || {
            let stdout = std::io::stdout();
            let stdin = std::io::stdin();
            debug::run(analysis, &path, &text, rx, sink, stdout, stdin.lock())
        })
        .ok()
        .and_then(|h| h.join().ok())
        .unwrap_or(1)
}
