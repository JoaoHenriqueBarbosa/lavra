//! O adaptador de depuração (`lavrac dap`) por cima do binário, como o editor o vê: um
//! cliente mínimo do protocolo conversa pelos pipes, e no terminal integrado faz o papel
//! do editor, abrindo o `lavrac debuggee` que o adaptador pede.

use lavra::dap::{read_message, write_message};
use serde_json::{Value, json};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Client {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    seq: i64,
    /// Os eventos que chegaram enquanto se esperava outra coisa.
    events: Vec<Value>,
    /// O programa que o adaptador pediu para abrir no terminal.
    terminal: Option<Child>,
}

impl Client {
    fn start() -> Client {
        let mut child =
            Command::new(env!("CARGO_BIN_EXE_lavrac")).arg("dap").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("lavrac dap");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        Client { child, stdin, out, seq: 0, events: Vec::new(), terminal: None }
    }

    fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        write_message(&mut self.stdin, &json!({ "seq": self.seq, "type": "request", "command": command, "arguments": arguments })).unwrap();
        self.seq
    }

    /// A próxima mensagem; o pedido ao contrário (`runInTerminal`) é atendido aqui.
    fn read(&mut self) -> Value {
        loop {
            let msg = read_message(&mut self.out).unwrap().expect("o adaptador fechou a saída");
            if msg["type"] == "request" && msg["command"] == "runInTerminal" {
                let args: Vec<String> =
                    msg["arguments"]["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect();
                let child = Command::new(&args[0])
                    .args(&args[1..])
                    .current_dir(msg["arguments"]["cwd"].as_str().unwrap())
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .expect("o programa no terminal");
                self.terminal = Some(child);
                self.seq += 1;
                let reply = json!({
                    "seq": self.seq, "type": "response", "request_seq": msg["seq"], "command": "runInTerminal",
                    "success": true, "body": {},
                });
                write_message(&mut self.stdin, &reply).unwrap();
                continue;
            }
            return msg;
        }
    }

    fn request(&mut self, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        loop {
            let msg = self.read();
            if msg["type"] == "response" && msg["request_seq"] == seq {
                return msg;
            }
            self.events.push(msg);
        }
    }

    fn ok(&mut self, command: &str, arguments: Value) -> Value {
        let r = self.request(command, arguments);
        assert_eq!(r["success"], true, "{command}: {r}");
        r["body"].clone()
    }

    fn event(&mut self, name: &str) -> Value {
        if let Some(i) = self.events.iter().position(|e| e["event"] == name) {
            return self.events.remove(i)["body"].clone();
        }
        loop {
            let msg = self.read();
            if msg["type"] == "event" && msg["event"] == name {
                return msg["body"].clone();
            }
            self.events.push(msg);
        }
    }

    /// O que o programa escreveu no console de depuração até agora.
    fn output(&self, category: &str) -> String {
        self.events
            .iter()
            .filter(|e| e["event"] == "output" && e["body"]["category"] == category)
            .filter_map(|e| e["body"]["output"].as_str())
            .collect()
    }

    fn top(&mut self) -> Value {
        self.ok("stackTrace", json!({ "threadId": 1 }))["stackFrames"][0].clone()
    }

    fn finish(mut self) {
        let _ = self.request("disconnect", json!({}));
        let status = self.child.wait().unwrap();
        assert!(status.success());
    }
}

const PROGRAM: &str = "\
Para render a quantia, sendo a quantia um inteiro confiado:
    A quantia cresce 10.

Para começar:
    O saldo começa em 100.
    As notas são 7, 8 e 9.
    Rendo o saldo.
    Escrevo o saldo.
";

fn program_file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lavra-dap-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("conta.lav");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn a_session_in_the_debug_console() {
    let path = program_file("console", PROGRAM);
    let p = path.display().to_string();
    let mut c = Client::start();
    let caps = c.ok("initialize", json!({ "adapterID": "lavra", "linesStartAt1": true, "columnsStartAt1": true }));
    assert_eq!(caps["supportsConfigurationDoneRequest"], true);
    c.event("initialized");
    c.ok("launch", json!({ "program": p }));
    // A linha em branco pega na próxima em que começa uma oração.
    let bps = c.ok("setBreakpoints", json!({ "source": { "path": p }, "breakpoints": [{ "line": 3 }, { "line": 7 }] }));
    assert_eq!(bps["breakpoints"], json!([{ "verified": true, "line": 5 }, { "verified": true, "line": 7 }]));
    c.ok("configurationDone", json!({}));

    assert_eq!(c.event("stopped")["reason"], "breakpoint");
    assert_eq!(c.top()["line"], 5);
    c.ok("continue", json!({ "threadId": 1 }));
    let stopped = c.event("stopped");
    assert_eq!((&stopped["reason"], &stopped["description"]), (&json!("breakpoint"), &json!("Parado no ponto de parada")));
    let top = c.top();
    assert_eq!((&top["name"], &top["line"], &top["source"]["path"]), (&json!("Para começar"), &json!(7), &json!(p)));
    assert_eq!(c.ok("threads", json!({}))["threads"][0]["id"], 1);
    let scopes = c.ok("scopes", json!({ "frameId": top["id"] }));
    let locals = scopes["scopes"].as_array().unwrap().iter().find(|s| s["name"] == "Locais").unwrap()["variablesReference"].clone();
    let vars = c.ok("variables", json!({ "variablesReference": locals }));
    assert_eq!((&vars["variables"][0]["name"], &vars["variables"][0]["value"]), (&json!("o saldo"), &json!("100")));
    assert_eq!(vars["variables"][0]["type"], "um inteiro");
    // O hover pergunta pela palavra.
    let hover = c.ok("evaluate", json!({ "expression": "saldo", "frameId": top["id"], "context": "hover" }));
    assert_eq!(hover["result"], "100");
    let missing = c.request("evaluate", json!({ "expression": "a taxa", "frameId": top["id"], "context": "hover" }));
    assert_eq!(missing["success"], false);

    c.ok("next", json!({ "threadId": 1 }));
    assert_eq!(c.event("stopped")["reason"], "step");
    assert_eq!(c.top()["line"], 8);
    c.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(c.event("exited")["exitCode"], 0);
    c.event("terminated");
    assert_eq!(c.output("stdout"), "110\n");
    c.finish();
}

#[test]
fn a_program_with_errors_does_not_start() {
    let path = program_file("errors", "Para começar:\n    Escrevo o sald.\n");
    let mut c = Client::start();
    c.ok("initialize", json!({ "adapterID": "lavra" }));
    let r = c.request("launch", json!({ "program": path.display().to_string() }));
    assert_eq!(r["success"], false);
    assert!(r["message"].as_str().unwrap().contains("erros"), "{r}");
    let shown = c.output("stderr");
    assert!(shown.contains("[L0334]") && shown.contains("conta.lav:2"), "{shown}");
    c.finish();
}

#[test]
fn a_session_in_the_integrated_terminal() {
    let path = program_file("terminal", PROGRAM);
    let p = path.display().to_string();
    let mut c = Client::start();
    c.ok("initialize", json!({ "adapterID": "lavra", "supportsRunInTerminalRequest": true }));
    c.ok("launch", json!({ "program": p, "console": "integratedTerminal" }));
    c.ok("setBreakpoints", json!({ "source": { "path": p }, "breakpoints": [{ "line": 7 }] }));
    c.ok("configurationDone", json!({}));
    assert_eq!(c.event("stopped")["reason"], "breakpoint");
    assert!(c.terminal.is_some(), "o adaptador pediu o terminal");
    assert_eq!(c.top()["line"], 7);
    c.ok("stepIn", json!({ "threadId": 1 }));
    c.event("stopped");
    assert_eq!(c.top()["name"], "Para render");
    c.ok("continue", json!({ "threadId": 1 }));
    assert_eq!(c.event("exited")["exitCode"], 0);
    // A saída foi para o terminal, não para o console de depuração.
    let mut terminal = c.terminal.take().unwrap();
    let mut printed = String::new();
    terminal.stdout.take().unwrap().read_to_string(&mut printed).unwrap();
    assert_eq!(printed, "110\n");
    assert!(terminal.wait().unwrap().success());
    assert_eq!(c.output("stdout"), "");
    c.finish();
}
