//! O servidor de linguagem em processo, numa conexão em memória: o que o editor pede e o
//! que recebe.

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{Value, json};

struct Client {
    conn: Connection,
    next: i32,
    notes: Vec<Notification>,
}

impl Client {
    fn start() -> (Client, std::thread::JoinHandle<()>) {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || lavra::lsp::run(&server).expect("servidor"));
        let mut c = Client { conn: client, next: 1, notes: Vec::new() };
        c.request("initialize", json!({ "capabilities": {}, "processId": null, "rootUri": null }));
        c.notify("initialized", json!({}));
        (c, handle)
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = RequestId::from(self.next);
        self.next += 1;
        self.conn.sender.send(Message::Request(Request::new(id.clone(), method.into(), params))).unwrap();
        loop {
            match self.conn.receiver.recv().unwrap() {
                Message::Response(r) if r.id == id => return r.response_result.unwrap_or_else(|e| panic!("erro em {method}: {e:?}")),
                Message::Notification(n) => self.notes.push(n),
                _ => {}
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.conn.sender.send(Message::Notification(Notification::new(method.into(), params))).unwrap();
    }

    /// Espera a próxima publicação de diagnósticos.
    fn diagnostics(&mut self) -> Vec<Value> {
        loop {
            if let Some(i) = self.notes.iter().position(|n| n.method == "textDocument/publishDiagnostics") {
                let n = self.notes.remove(i);
                return n.params["diagnostics"].as_array().unwrap().clone();
            }
            if let Message::Notification(n) = self.conn.receiver.recv().unwrap() {
                self.notes.push(n);
            }
        }
    }

    fn stop(mut self, handle: std::thread::JoinHandle<()>) {
        self.request("shutdown", Value::Null);
        self.notify("exit", Value::Null);
        handle.join().unwrap();
    }
}

const URI: &str = "file:///tmp/conta.lav";

const PROGRAM: &str = "\
Para render a quantia, sendo a quantia um inteiro confiado:
    A quantia cresce 10.

Para começar:
    O saldo começa em 100.
    Rendo o saldo.
    Escrevo o saldo.
    Escrevo o sald.
";

fn pos(line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": URI }, "position": { "line": line, "character": character } })
}

#[test]
fn diagnostics_hover_definition_references_symbols() {
    let (mut c, handle) = Client::start();
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI, "languageId": "lavra", "version": 1, "text": PROGRAM } }));
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert_eq!(diags[0]["code"], "L0334");
    assert_eq!(diags[0]["range"]["start"], json!({ "line": 7, "character": 12 }));
    assert!(diags[0]["codeDescription"]["href"].as_str().unwrap().ends_with("L0334.md"));

    // Hover no "saldo" de "Escrevo o saldo.": variável, inteiro, linha da declaração.
    let h = c.request("textDocument/hover", pos(6, 15));
    let text = h["contents"]["value"].as_str().unwrap();
    assert!(text.contains("**o saldo**: variável, um inteiro") && text.contains("linha 5"), "{text}");
    // Hover no verbo da chamada: a regência.
    let h = c.request("textDocument/hover", pos(5, 5));
    assert!(h["contents"]["value"].as_str().unwrap().contains("rendo QUANTIA"), "{h}");
    // Hover numa palavra da língua.
    let h = c.request("textDocument/hover", pos(4, 13));
    assert!(h["contents"]["value"].as_str().unwrap().contains("Declara um valor que muda"), "{h}");

    // Definição e referências de "o saldo".
    let d = c.request("textDocument/definition", pos(6, 15));
    assert_eq!(d["range"]["start"], json!({ "line": 4, "character": 4 }));
    let mut refs = pos(6, 15);
    refs["context"] = json!({ "includeDeclaration": true });
    assert_eq!(c.request("textDocument/references", refs).as_array().unwrap().len(), 3);
    let hl = c.request("textDocument/documentHighlight", pos(6, 15));
    assert_eq!(hl.as_array().unwrap().len(), 3);

    // Símbolos: a função com a regência.
    let syms = c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": URI } }));
    let s = &syms.as_array().unwrap()[0];
    assert_eq!((s["name"].as_str(), s["detail"].as_str()), (Some("render"), Some("rendo QUANTIA")));

    // Tokens semânticos: o verbo da definição é função; "quantia" é parâmetro confiado.
    let toks = c.request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": URI } }));
    let data: Vec<u64> = toks["data"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let (mut line, mut col) = (0, 0);
    let mut decoded = Vec::new();
    for t in data.chunks(5) {
        if t[0] > 0 {
            col = 0;
        }
        line += t[0];
        col += t[1];
        decoded.push((line, col, t[2], t[3], t[4]));
    }
    let types = lavra::lsp::semantic::TYPES;
    let at = |l: u64, c0: u64| decoded.iter().find(|t| t.0 == l && t.1 == c0).map(|t| (types[t.3 as usize], t.4));
    assert_eq!(at(0, 5).map(|t| t.0), Some("function"));
    let (kind, mods) = at(0, 14).expect("quantia no cabeçalho");
    assert_eq!(kind, "parameter");
    assert!(mods & (1 << 4) != 0, "quantia é confiada");
    assert_eq!(at(5, 4).map(|t| t.0), Some("function"), "Rendo");

    // Formatação: o texto já está na forma, nada a mudar; estragado, volta inteiro.
    let fmt =
        c.request("textDocument/formatting", json!({ "textDocument": { "uri": URI }, "options": { "tabSize": 4, "insertSpaces": true } }));
    assert_eq!(fmt, json!([]));
    let spoiled = PROGRAM.replace("    O saldo começa em 100.", "\tO saldo   começa em 100 .");
    c.notify("textDocument/didChange", json!({ "textDocument": { "uri": URI, "version": 2 }, "contentChanges": [{ "text": spoiled }] }));
    c.diagnostics();
    let fmt =
        c.request("textDocument/formatting", json!({ "textDocument": { "uri": URI }, "options": { "tabSize": 4, "insertSpaces": true } }));
    assert_eq!(fmt[0]["newText"], PROGRAM);

    // Corrigir o erro limpa os diagnósticos.
    let fixed = PROGRAM.replace("o sald.", "o saldo.");
    c.notify("textDocument/didChange", json!({ "textDocument": { "uri": URI, "version": 3 }, "contentChanges": [{ "text": fixed }] }));
    assert!(c.diagnostics().is_empty());
    c.stop(handle);
}

#[test]
fn unknown_request_does_not_take_the_server_down() {
    let (c, handle) = Client::start();
    let id = RequestId::from(99);
    c.conn.sender.send(Message::Request(Request::new(id.clone(), "lavra/naoExiste".into(), json!({})))).unwrap();
    loop {
        if let Message::Response(r) = c.conn.receiver.recv().unwrap()
            && r.id == id
        {
            assert!(r.response_result.is_err());
            break;
        }
    }
    // Pedido sobre documento que não está aberto: erro, e o servidor segue.
    let id = RequestId::from(100);
    c.conn.sender.send(Message::Request(Request::new(id.clone(), "textDocument/hover".into(), pos(0, 0)))).unwrap();
    loop {
        if let Message::Response(r) = c.conn.receiver.recv().unwrap()
            && r.id == id
        {
            assert!(r.response_result.is_err());
            break;
        }
    }
    c.stop(handle);
}
