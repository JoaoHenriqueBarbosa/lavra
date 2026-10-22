//! O núcleo do depurador, sem o protocolo: o programa roda numa thread sob o
//! `Debugger`, e o teste conversa com ele por comandos e eventos.

use lavra::debug::{Command, Event, Sink, spawn};
use serde_json::Value;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Session {
    commands: Sender<Command>,
    events: Receiver<Event>,
    output: String,
    seq: i64,
}

impl Session {
    fn start(src: &str, input: &str, breakpoints: &[u32], stop_on_entry: bool) -> Session {
        let a = lavra::analyze(src);
        assert!(!a.has_errors(), "{:?}", a.diagnostics);
        let (tx, rx) = channel();
        let sink: Sink = Arc::new(Mutex::new(move |e| {
            let _ = tx.send(e);
        }));
        let input = Box::new(std::io::Cursor::new(input.as_bytes().to_vec()));
        let (commands, _) = spawn(a, "/tmp/conta.lav".into(), src.into(), sink, input).unwrap();
        commands.send(Command::SetBreakpoints { lines: breakpoints.to_vec() }).unwrap();
        commands.send(Command::Start { stop_on_entry }).unwrap();
        Session { commands, events: rx, output: String::new(), seq: 0 }
    }

    /// O próximo evento que não é saída do programa (a saída fica guardada).
    fn next(&mut self) -> Event {
        loop {
            match self.events.recv_timeout(Duration::from_secs(10)).expect("o depurador não respondeu") {
                Event::Output { text, category } if category == "stdout" => self.output.push_str(&text),
                e => return e,
            }
        }
    }

    fn stopped(&mut self) -> (String, Option<String>) {
        match self.next() {
            Event::Stopped { reason, text } => (reason, text),
            e => panic!("esperava uma parada, veio {e:?}"),
        }
    }

    fn ask(&mut self, make: impl FnOnce(i64) -> Command) -> Value {
        self.seq += 1;
        let seq = self.seq;
        self.commands.send(make(seq)).unwrap();
        match self.next() {
            Event::Reply { seq: s, body: Some(b), .. } if s == seq => b,
            e => panic!("esperava a resposta {seq}, veio {e:?}"),
        }
    }

    /// Os nomes e valores de uma referência: [("o saldo", "100", referência)].
    fn variables(&mut self, reference: u64) -> Vec<(String, String, u64)> {
        let v = self.ask(|seq| Command::Variables { seq, reference: reference as u32 });
        v["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (x["name"].as_str().unwrap().into(), x["value"].as_str().unwrap().into(), x["variablesReference"].as_u64().unwrap()))
            .collect()
    }

    fn top(&mut self) -> (String, u64, usize) {
        let st = self.ask(|seq| Command::StackTrace { seq });
        let f = &st["stackFrames"][0];
        (f["name"].as_str().unwrap().into(), f["line"].as_u64().unwrap(), f["id"].as_u64().unwrap() as usize)
    }

    fn scope(&mut self, frame: usize, name: &str) -> u64 {
        let s = self.ask(|seq| Command::Scopes { seq, frame });
        s["scopes"].as_array().unwrap().iter().find(|x| x["name"] == name).unwrap_or_else(|| panic!("sem o escopo {name}: {s}"))["variablesReference"]
            .as_u64()
            .unwrap()
    }

    fn exited(&mut self) -> u8 {
        match self.next() {
            Event::Exited { code } => code,
            e => panic!("esperava o fim, veio {e:?}"),
        }
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

#[test]
fn breakpoint_stack_variables_and_steps() {
    let mut s = Session::start(PROGRAM, "", &[7], false);
    assert_eq!(s.stopped().0, "breakpoint");
    let (name, line, frame) = s.top();
    assert_eq!((name.as_str(), line), ("Para começar", 7));
    let locals = s.scope(frame, "Locais");
    let vars = s.variables(locals);
    assert_eq!((vars[0].0.as_str(), vars[0].1.as_str()), ("o saldo", "100"));
    let notas = vars.iter().find(|v| v.0 == "as notas").expect("as notas");
    assert_eq!(notas.1, "[7; 8; 9]");
    let items = s.variables(notas.2);
    assert_eq!(
        items.iter().map(|v| (v.0.as_str(), v.1.as_str())).collect::<Vec<_>>(),
        [("o item 1", "7"), ("o item 2", "8"), ("o item 3", "9")]
    );

    // Entrar na função: dois quadros, e o parâmetro confiado com o valor.
    s.commands.send(Command::StepIn).unwrap();
    assert_eq!(s.stopped().0, "step");
    let st = s.ask(|seq| Command::StackTrace { seq });
    let frames = st["stackFrames"].as_array().unwrap();
    assert_eq!((frames[0]["name"].as_str(), frames[0]["line"].as_u64()), (Some("Para render"), Some(2)));
    assert_eq!((frames[1]["name"].as_str(), frames[1]["line"].as_u64()), (Some("Para começar"), Some(7)));
    let params = s.scope(frames[0]["id"].as_u64().unwrap() as usize, "Parâmetros");
    assert_eq!(s.variables(params)[0].1, "100");

    // Sair: volta ao "Para começar" na oração seguinte, com o saldo rendido.
    s.commands.send(Command::StepOut).unwrap();
    assert_eq!(s.stopped().0, "step");
    assert_eq!(s.top().1, 8);
    let v = s.ask(|seq| Command::Evaluate { seq, text: "o saldo".into(), frame: None });
    assert_eq!(v["result"], "110");
    let v = s.ask(|seq| Command::Evaluate { seq, text: "saldo".into(), frame: None });
    assert_eq!(v["result"], "110");

    s.commands.send(Command::Continue).unwrap();
    assert_eq!(s.exited(), 0);
    assert_eq!(s.output, "110\n");
}

#[test]
fn next_steps_over_calls_and_entry_stops_first() {
    let mut s = Session::start(PROGRAM, "", &[], true);
    assert_eq!(s.stopped().0, "entry");
    assert_eq!(s.top().1, 5);
    for want in [6, 7, 8] {
        s.commands.send(Command::Next).unwrap();
        assert_eq!(s.stopped().0, "step");
        assert_eq!(s.top().1, want, "o \"next\" passa por cima da chamada");
    }
    s.commands.send(Command::Continue).unwrap();
    assert_eq!(s.exited(), 0);
}

#[test]
fn runtime_error_stops_where_it_happened() {
    let src = "Para começar:\n    As notas são 7 e 8.\n    Escrevo o item 5 das notas.\n";
    let mut s = Session::start(src, "", &[], false);
    let (reason, text) = s.stopped();
    assert_eq!(reason, "exception");
    assert!(text.unwrap().contains("5"), "a mensagem da falha");
    assert_eq!(s.top().1, 3);
    s.commands.send(Command::Continue).unwrap();
    match s.next() {
        Event::Output { text, category } => {
            assert_eq!(category, "stderr");
            assert!(text.contains("Erro de execução") && text.contains("conta.lav:3"), "{text}");
        }
        e => panic!("{e:?}"),
    }
    assert_eq!(s.exited(), 101);
}

#[test]
fn pause_stops_an_endless_loop_and_terminate_ends_it() {
    let src = "Para começar:\n    O contador começa em 0.\n    Repito sempre:\n        O contador cresce 1.\n";
    let mut s = Session::start(src, "", &[], false);
    std::thread::sleep(Duration::from_millis(50));
    s.commands.send(Command::Pause).unwrap();
    assert_eq!(s.stopped().0, "pause");
    assert_eq!(s.top().1, 4);
    s.commands.send(Command::Terminate).unwrap();
    assert_eq!(s.exited(), 0);
}

#[test]
fn input_comes_from_the_given_reader() {
    let src = "Para começar:\n    Leio um inteiro e escrevo o resultado vezes 2.\n";
    let mut s = Session::start(src, "21\n", &[], false);
    assert_eq!(s.exited(), 0);
    assert_eq!(s.output, "42\n");
}
