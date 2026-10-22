//! O servidor de linguagem em processo, numa conexão em memória: o que o editor pede e o
//! que recebe.

use lsp_server::{Connection, Message, Notification, Request, RequestId};
use serde_json::{Value, json};

struct Client {
    conn: Connection,
    next: i32,
    notes: Vec<Notification>,
    /// Os pedidos que o servidor fez ao cliente.
    asked: Vec<Request>,
}

impl Client {
    fn start() -> (Client, std::thread::JoinHandle<()>) {
        Client::start_with(Value::Null)
    }

    /// O servidor, com as opções de inicialização do cliente.
    fn start_with(options: Value) -> (Client, std::thread::JoinHandle<()>) {
        Client::start_full(json!({}), options)
    }

    /// O servidor, com as capacidades e as opções de inicialização do cliente.
    fn start_full(capabilities: Value, options: Value) -> (Client, std::thread::JoinHandle<()>) {
        let (server, client) = Connection::memory();
        let handle = std::thread::spawn(move || lavra::lsp::run(&server).expect("servidor"));
        let mut c = Client { conn: client, next: 1, notes: Vec::new(), asked: Vec::new() };
        c.request(
            "initialize",
            json!({ "capabilities": capabilities, "processId": null, "rootUri": null, "initializationOptions": options }),
        );
        c.notify("initialized", json!({}));
        (c, handle)
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.try_request(method, params).unwrap_or_else(|e| panic!("erro em {method}: {e}"))
    }

    /// O pedido, com a mensagem de erro se o servidor recusar.
    fn try_request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = RequestId::from(self.next);
        self.next += 1;
        self.conn.sender.send(Message::Request(Request::new(id.clone(), method.into(), params))).unwrap();
        loop {
            match self.conn.receiver.recv().unwrap() {
                Message::Response(r) if r.id == id => return r.response_result.map_err(|e| e.message),
                Message::Notification(n) => self.notes.push(n),
                Message::Request(r) => self.asked.push(r),
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
            match self.conn.receiver.recv().unwrap() {
                Message::Notification(n) => self.notes.push(n),
                Message::Request(r) => self.asked.push(r),
                _ => {}
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

/// Os tokens semânticos em posições absolutas: (linha, coluna, tipo, modificadores).
fn semantic_tokens(c: &mut Client, uri: &str) -> Vec<(u64, u64, &'static str, u64)> {
    let toks = c.request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }));
    let data: Vec<u64> = toks["data"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    let (mut line, mut col) = (0, 0);
    let mut out = Vec::new();
    for t in data.chunks(5) {
        if t[0] > 0 {
            col = 0;
        }
        line += t[0];
        col += t[1];
        out.push((line, col, lavra::lsp::semantic::TYPES[t[3] as usize], t[4]));
    }
    out
}

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
    let decoded = semantic_tokens(&mut c, URI);
    let at = |l: u64, c0: u64| decoded.iter().find(|t| t.0 == l && t.1 == c0).map(|t| (t.2, t.3));
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

const URI2: &str = "file:///tmp/mover.lav";

const PROGRAM2: &str = "\
Para mover a quantia para o destino, sendo a quantia um inteiro e o destino um inteiro confiado:
    O destino cresce a quantia.

Para começar:
    O saldo começa em 100.
    A taxa, um inteiro, é 3.
    Movo 5 para o saldo.
    Escrevo o saldo
    Escrevo o saldo mais a taxa.
";

fn at2(line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": URI2 }, "position": { "line": line, "character": character } })
}

fn labels(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|i| i["label"].as_str().unwrap().to_string()).collect()
}

#[test]
fn writing_assistance() {
    let (mut c, handle) = Client::start();
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI2, "languageId": "lavra", "version": 1, "text": PROGRAM2 } }));
    let diags = c.diagnostics();
    assert_eq!(diags.len(), 1, "{diags:?}");

    // Completar no começo da oração: verbos da língua, o verbo do programa com a regência,
    // as orações de controle e o dado como sujeito, em maiúscula.
    let items = c.request("textDocument/completion", at2(6, 4));
    let l = labels(&items);
    for want in ["Escrevo", "Movo", "Se", "Enquanto", "O saldo"] {
        assert!(l.contains(&want.to_string()), "falta {want}: {l:?}");
    }
    let movo = items.as_array().unwrap().iter().find(|i| i["label"] == "Movo").unwrap();
    assert_eq!(movo["insertText"], "Movo ${1:quantia} para ${2:destino}");
    // Depois de "o": os nomes masculinos singulares, as posições e "o resultado".
    let l = labels(&c.request("textDocument/completion", at2(8, 19)));
    assert!(l.contains(&"saldo".to_string()) && l.contains(&"resultado".to_string()) && l.contains(&"primeiro".to_string()), "{l:?}");
    assert!(!l.contains(&"taxa".to_string()), "a taxa é feminina: {l:?}");
    // Na posição de tipo.
    let l = labels(&c.request("textDocument/completion", at2(5, 22)));
    assert!(l.contains(&"inteiro".to_string()) && l.contains(&"real".to_string()) && l.contains(&"texto".to_string()), "{l:?}");

    // Ajuda de assinatura: depois de "para", o parâmetro ativo é o destino.
    let sig = c.request("textDocument/signatureHelp", at2(6, 16));
    assert_eq!(sig["signatures"][0]["label"], "movo QUANTIA para DESTINO");
    assert_eq!(sig["activeParameter"], 1);

    // Inlay hint: o tipo da declaração sem aposto.
    let hints = c.request("textDocument/inlayHint", json!({ "textDocument": { "uri": URI2 }, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 9, "character": 0 } } }));
    let texts: Vec<&str> = hints.as_array().unwrap().iter().map(|h| h["label"].as_str().unwrap()).collect();
    assert!(texts.contains(&", um inteiro,"), "{texts:?}");
    assert!(texts.contains(&"confiado:"), "o argumento confiado: {texts:?}");

    // Ação rápida: o ponto final que faltou.
    let actions = c.request(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": URI2 }, "range": diags[0]["range"], "context": { "diagnostics": diags } }),
    );
    let a = &actions.as_array().unwrap()[0];
    assert_eq!(a["title"], "Pôr o ponto final");
    let edit = &a["edit"]["changes"][URI2][0];
    assert_eq!((edit["newText"].as_str(), &edit["range"]["start"]), (Some("."), &json!({ "line": 7, "character": 19 })));

    // Dobra: o corpo da função e o de "Para começar".
    let folds = c.request("textDocument/foldingRange", json!({ "textDocument": { "uri": URI2 } }));
    let spans: Vec<(u64, u64)> =
        folds.as_array().unwrap().iter().map(|f| (f["startLine"].as_u64().unwrap(), f["endLine"].as_u64().unwrap())).collect();
    assert!(spans.contains(&(0, 1)) && spans.contains(&(3, 8)), "{spans:?}");

    // Seleção crescente: a palavra, a oração, o bloco, o texto.
    let sel =
        c.request("textDocument/selectionRange", json!({ "textDocument": { "uri": URI2 }, "positions": [{ "line": 6, "character": 18 }] }));
    let mut depth = 0;
    let mut r = &sel[0];
    while !r.is_null() {
        depth += 1;
        r = &r["parent"];
    }
    assert!(depth >= 4, "{sel}");
    c.stop(handle);
}

/// Aplica as edições de um `WorkspaceEdit` (linhas e colunas em UTF-16) ao texto.
fn apply_workspace_edit(text: &str, edit: &Value, uri: &str) -> String {
    let lines = lavra::span::LineIndex::new(text);
    let offset = |p: &Value| {
        let line = p["line"].as_u64().unwrap() as usize;
        let start = lines.line_start(line, text);
        let col = p["character"].as_u64().unwrap() as usize;
        let mut units = 0;
        let mut at = start;
        for c in text[start..].chars() {
            if units >= col {
                break;
            }
            units += c.len_utf16();
            at += c.len_utf8();
        }
        at
    };
    let mut edits: Vec<(usize, usize, String)> = edit["changes"][uri]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (offset(&e["range"]["start"]), offset(&e["range"]["end"]), e["newText"].as_str().unwrap().to_string()))
        .collect();
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    let mut out = text.to_string();
    for (s, e, t) in edits {
        out.replace_range(s..e, &t);
    }
    out
}

#[test]
fn rename_and_fix_all() {
    let (mut c, handle) = Client::start();
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI, "languageId": "lavra", "version": 1, "text": PROGRAM } }));
    c.diagnostics();
    // O editor propõe o nome com o artigo, e marca só a palavra.
    let p = c.request("textDocument/prepareRename", pos(6, 15));
    assert_eq!(p["placeholder"], "o saldo");
    assert_eq!(
        (&p["range"]["start"], &p["range"]["end"]),
        (&json!({ "line": 6, "character": 14 }), &json!({ "line": 6, "character": 19 }))
    );
    // Outro gênero: o artigo vai junto, com a maiúscula do começo da oração.
    let mut params = pos(6, 15);
    params["newName"] = json!("a reserva");
    let edit = c.request("textDocument/rename", params);
    let renamed = apply_workspace_edit(PROGRAM, &edit, URI);
    assert!(
        renamed.contains("    A reserva começa em 100.\n    Rendo a reserva.\n    Escrevo a reserva.\n    Escrevo o sald."),
        "{renamed}"
    );
    // O verbo, pelo uso na 1ª pessoa.
    let mut params = pos(5, 5);
    params["newName"] = json!("aplicar");
    let renamed = apply_workspace_edit(PROGRAM, &c.request("textDocument/rename", params), URI);
    assert!(renamed.starts_with("Para aplicar a quantia,") && renamed.contains("    Aplico o saldo."), "{renamed}");
    // Recusado, com o motivo.
    let mut params = pos(6, 15);
    params["newName"] = json!("valor");
    let e = c.try_request("textDocument/rename", params).unwrap_err();
    assert!(e.contains("gênero"), "{e}");
    assert!(c.try_request("textDocument/prepareRename", pos(6, 5)).is_err(), "\"Escrevo\" é da língua");

    // "Corrigir tudo": o ponto final e a comparação, numa ação só.
    let broken = "Para começar:\n    A nota é 7\n    Se a nota for maiores que 5: escrevo \"ok\".\n";
    c.notify("textDocument/didChange", json!({ "textDocument": { "uri": URI, "version": 2 }, "contentChanges": [{ "text": broken }] }));
    c.diagnostics();
    let range = json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } });
    let actions = c.request(
        "textDocument/codeAction",
        json!({ "textDocument": { "uri": URI }, "range": range, "context": { "diagnostics": [], "only": ["source.fixAll"] } }),
    );
    let all = &actions.as_array().unwrap()[0];
    assert_eq!((all["title"].as_str(), all["kind"].as_str()), (Some("Corrigir tudo o que é seguro"), Some("source.fixAll")));
    let fixed = apply_workspace_edit(broken, &all["edit"], URI);
    assert_eq!(fixed, "Para começar:\n    A nota é 7.\n    Se a nota for maior que 5: escrevo \"ok\".\n");
    c.stop(handle);
}

const URI3: &str = "file:///tmp/textos.lav";

const PROGRAM3: &str = "\
Uso a biblioteca de textos.

Para começar:
    A frase é \"a,b\".
    Divido a frase por \",\" e escrevo o resultado.
    Se a frase contiver \"a\": escrevo \"sim\".
";

fn at3(line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": URI3 }, "position": { "line": line, "character": character } })
}

/// O aceite das bibliotecas: os verbos de uma biblioteca aparecem no editor como os do
/// programa, sem nada no servidor que saiba de bibliotecas.
#[test]
fn a_library_is_served_like_the_program() {
    let (mut c, handle) = Client::start();
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI3, "languageId": "lavra", "version": 1, "text": PROGRAM3 } }));
    assert!(c.diagnostics().is_empty());
    // Completar no começo da oração: os verbos da biblioteca, com a regência.
    let items = c.request("textDocument/completion", at3(4, 4));
    let l = labels(&items);
    for want in ["Divido", "Junto", "Aparo"] {
        assert!(l.contains(&want.to_string()), "falta {want}: {l:?}");
    }
    let divido = items.as_array().unwrap().iter().find(|i| i["label"] == "Divido").unwrap();
    assert_eq!(divido["insertText"], "Divido ${1:texto} por ${2:separador}");
    // Hover: a regência e de onde vem.
    let h = c.request("textDocument/hover", at3(4, 5));
    let text = h["contents"]["value"].as_str().unwrap();
    assert!(text.contains("divido TEXTO por SEPARADOR") && text.contains("Vem da biblioteca de textos"), "{text}");
    // Ajuda de assinatura depois do verbo.
    let sig = c.request("textDocument/signatureHelp", at3(4, 11));
    assert_eq!(sig["signatures"][0]["label"], "divido TEXTO por SEPARADOR");
    // A definição fica na biblioteca: sem um cliente que saiba abrir o texto dela, nada.
    assert!(c.request("textDocument/definition", at3(4, 5)).is_null());
    // Tokens semânticos: o verbo e o predicado da biblioteca são funções.
    let found: Vec<(u64, u64, &str)> = semantic_tokens(&mut c, URI3).into_iter().map(|t| (t.0, t.1, t.2)).collect();
    assert!(found.contains(&(4, 4, "function")) && found.contains(&(5, 15, "function")), "{found:?}");
    // O rename de um verbo da biblioteca é recusado, com o motivo.
    let mut params = at3(4, 5);
    params["newName"] = json!("partir");
    let e = c.try_request("textDocument/rename", params).unwrap_err();
    assert!(e.contains("vem da biblioteca de textos"), "{e}");
    c.stop(handle);
}

/// No cliente que sabe abrir o texto das bibliotecas, a definição de um verbo de biblioteca
/// leva ao cabeçalho dele, e o texto vem do pedido `lavra/library`.
#[test]
fn the_definition_of_a_library_verb_opens_the_library() {
    let (mut c, handle) = Client::start_with(json!({ "libraryDocuments": true }));
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI3, "languageId": "lavra", "version": 1, "text": PROGRAM3 } }));
    assert!(c.diagnostics().is_empty());
    let loc = c.request("textDocument/definition", at3(4, 5));
    assert_eq!(loc["uri"], "lavra-library:/textos.lav");
    let source = c.request("lavra/library", json!({ "uri": "lavra-library:/textos.lav" }))["text"].as_str().unwrap().to_string();
    assert_eq!(source, lavra::builtins::LIBRARIES[0].source);
    let line = loc["range"]["start"]["line"].as_u64().unwrap() as usize;
    let (start, end) =
        (loc["range"]["start"]["character"].as_u64().unwrap() as usize, loc["range"]["end"]["character"].as_u64().unwrap() as usize);
    let header = source.lines().nth(line).unwrap();
    assert!(header.starts_with("Para dividir"), "{header}");
    assert_eq!(&header[start..end], "dividir");
    // Um endereço que não é de biblioteca é recusado.
    let e = c.try_request("lavra/library", json!({ "uri": "lavra-library:/nada.lav" })).unwrap_err();
    assert!(e.contains("não há biblioteca"), "{e}");
    c.stop(handle);
}

const URI4: &str = "file:///tmp/posse.lav";

const PROGRAM4: &str = "\
Para arquivar as cartas, sendo as cartas uma lista de textos:
    Escrevo as cartas.

Para começar:
    As cartas começam vazias.
    As cartas ganham \"Querida Ana\".
    Os rascunhos são \"nova\" e \"outra\".
    Arquivo as cartas.
    As cartas viram os rascunhos.
    Escrevo as cartas.
";

fn at4(line: u32, character: u32) -> Value {
    json!({ "textDocument": { "uri": URI4 }, "position": { "line": line, "character": character } })
}

fn hover_text(c: &mut Client, at: Value) -> String {
    c.request("textDocument/hover", at)["contents"]["value"].as_str().unwrap_or_default().to_string()
}

/// A posse no editor: o hover diz onde o valor é entregue e onde o nome volta a ter um, e
/// o nome que entrega o valor leva o modificador `moved`.
#[test]
fn ownership_in_hover_and_tokens() {
    let (mut c, handle) = Client::start();
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI4, "languageId": "lavra", "version": 1, "text": PROGRAM4 } }));
    assert!(c.diagnostics().is_empty());
    let given = hover_text(&mut c, at4(7, 16));
    assert!(given.contains("As cartas são entregues a \"arquivar\" aqui: daqui em diante, não se usam mais."), "{given}");
    let back = hover_text(&mut c, at4(8, 8));
    assert!(back.contains("As cartas foram entregues a \"arquivar\" na linha 8; aqui ganham um valor novo."), "{back}");
    let passed = hover_text(&mut c, at4(8, 24));
    assert!(passed.contains("Os rascunhos passam para \"as cartas\" aqui: daqui em diante, não se usam mais."), "{passed}");
    let later = hover_text(&mut c, at4(9, 16));
    assert!(later.contains("**as cartas**") && !later.contains("entregue"), "{later}");

    let moved = 1u64 << lavra::lsp::semantic::MODIFIERS.iter().position(|m| *m == "moved").unwrap();
    let toks = semantic_tokens(&mut c, URI4);
    let mods =
        |l: u64, col: u64| toks.iter().find(|t| t.0 == l && t.1 == col).map(|t| t.3).unwrap_or_else(|| panic!("sem token em {l}:{col}"));
    assert!(mods(7, 15) & moved != 0, "\"cartas\" entrega o valor em \"Arquivo as cartas.\"");
    assert!(mods(8, 23) & moved != 0, "\"rascunhos\" passa o valor para \"as cartas\"");
    assert!(mods(8, 7) & moved == 0 && mods(9, 15) & moved == 0, "só o ponto da entrega leva o modificador");
    c.stop(handle);
}

fn hint_labels(c: &mut Client) -> Vec<String> {
    let range = json!({ "start": { "line": 0, "character": 0 }, "end": { "line": 9, "character": 0 } });
    let hints = c.request("textDocument/inlayHint", json!({ "textDocument": { "uri": URI }, "range": range }));
    hints.as_array().unwrap().iter().map(|h| h["label"].as_str().unwrap().to_string()).collect()
}

/// O editor escolhe os inlay hints, nas opções de inicialização ou na configuração; quando a
/// escolha muda, o servidor pede ao editor que busque as dicas de novo.
#[test]
fn inlay_hints_follow_the_configuration() {
    let capabilities = json!({ "workspace": { "inlayHint": { "refreshSupport": true } } });
    let (mut c, handle) = Client::start_full(capabilities, json!({ "inlayHints": { "modes": false } }));
    c.notify("textDocument/didOpen", json!({ "textDocument": { "uri": URI, "languageId": "lavra", "version": 1, "text": PROGRAM } }));
    c.diagnostics();
    let l = hint_labels(&mut c);
    assert!(l.contains(&", um inteiro,".to_string()) && !l.contains(&"confiado:".to_string()), "{l:?}");

    let settings = json!({ "settings": { "lavra": { "inlayHints": { "modes": true, "types": false } } } });
    c.notify("workspace/didChangeConfiguration", settings.clone());
    let l = hint_labels(&mut c);
    assert!(l.contains(&"confiado:".to_string()) && !l.contains(&", um inteiro,".to_string()), "{l:?}");
    let refreshes = |c: &Client| c.asked.iter().filter(|r| r.method == "workspace/inlayHint/refresh").count();
    assert_eq!(refreshes(&c), 1);

    // A mesma escolha de novo não muda nada, e não há o que buscar.
    c.notify("workspace/didChangeConfiguration", settings);
    hint_labels(&mut c);
    assert_eq!(refreshes(&c), 1);
    c.stop(handle);
}
