//! O léxico da Lavra: cada palavra que a língua usa, com o papel e uma linha de
//! documentação. É a fonte única dessas palavras:
//!
//! - o léxico, o parser e a morfologia consultam as tabelas daqui;
//! - as formas que um verbo do usuário não pode ter na 1ª pessoa saem daqui
//!   ([`reserved`]): toda palavra da língua, menos as recusadas com dica;
//! - `lavrac lexicon --json` exporta tudo ([`to_json`]) para as gramáticas dos editores,
//!   o conferidor da morfologia e o site;
//! - o servidor de linguagem mostra a documentação no hover ([`lookup`]).
//!
//! Um teste lê o código do parser e confere que toda palavra que ele consulta está aqui.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::ast::{BinOp, UnOp};
use crate::morphology::{self, Gender, Number};

/// Para que a língua usa a palavra. Os papéis viram os escopos do realce, os tokens
/// semânticos do servidor e as seções da referência.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    Article,
    Contraction,
    Preposition,
    Conjunction,
    /// Verbo da língua que age, na 1ª pessoa: escrevo, leio, tomo.
    Statement,
    /// Verbo da língua que desvia o fluxo: devolvo, desisto, paro, pulo.
    Flow,
    /// Verbo do sujeito que declara: é, são, começa, começam.
    Declaration,
    /// Verbo do sujeito que muda o dado: vira, cresce, diminui, ganha, perde.
    Mutation,
    Control,
    /// Futuro do subjuntivo das condições: for, forem, houver.
    Copula,
    Definition,
    Signature,
    Ownership,
    Type,
    Nominal,
    Operator,
    Comparison,
    Literal,
    Position,
    Anaphora,
    Output,
    Comment,
    /// Forma que a língua não aceita onde o usuário a escreveu, e responde com dica.
    /// Não fica reservada: "guardo" é um bom verbo para o usuário definir.
    Rejected,
    /// Reservada sem uso na sintaxe: não pode ser a 1ª pessoa de um verbo do usuário.
    Reserved,
}

impl Role {
    pub const ALL: &[Role] = &[
        Role::Article,
        Role::Contraction,
        Role::Preposition,
        Role::Conjunction,
        Role::Statement,
        Role::Flow,
        Role::Declaration,
        Role::Mutation,
        Role::Control,
        Role::Copula,
        Role::Definition,
        Role::Signature,
        Role::Ownership,
        Role::Type,
        Role::Nominal,
        Role::Operator,
        Role::Comparison,
        Role::Literal,
        Role::Position,
        Role::Anaphora,
        Role::Output,
        Role::Comment,
        Role::Rejected,
        Role::Reserved,
    ];

    /// Identificador estável, para o JSON e para os escopos dos editores.
    pub fn id(self) -> &'static str {
        match self {
            Role::Article => "article",
            Role::Contraction => "contraction",
            Role::Preposition => "preposition",
            Role::Conjunction => "conjunction",
            Role::Statement => "statement",
            Role::Flow => "flow",
            Role::Declaration => "declaration",
            Role::Mutation => "mutation",
            Role::Control => "control",
            Role::Copula => "copula",
            Role::Definition => "definition",
            Role::Signature => "signature",
            Role::Ownership => "ownership",
            Role::Type => "type",
            Role::Nominal => "nominal",
            Role::Operator => "operator",
            Role::Comparison => "comparison",
            Role::Literal => "literal",
            Role::Position => "position",
            Role::Anaphora => "anaphora",
            Role::Output => "output",
            Role::Comment => "comment",
            Role::Rejected => "rejected",
            Role::Reserved => "reserved",
        }
    }

    /// O papel em português, para o hover e para a referência.
    pub fn describe(self) -> &'static str {
        match self {
            Role::Article => "artigo",
            Role::Contraction => "contração",
            Role::Preposition => "preposição",
            Role::Conjunction => "conjunção",
            Role::Statement => "verbo da língua",
            Role::Flow => "verbo de fluxo",
            Role::Declaration => "declaração",
            Role::Mutation => "mudança do dado",
            Role::Control => "controle",
            Role::Copula => "verbo da condição",
            Role::Definition => "definição",
            Role::Signature => "cabeçalho",
            Role::Ownership => "posse",
            Role::Type => "tipo",
            Role::Nominal => "operação por extenso",
            Role::Operator => "operador",
            Role::Comparison => "comparação",
            Role::Literal => "valor da língua",
            Role::Position => "posição",
            Role::Anaphora => "retomada",
            Role::Output => "saída",
            Role::Comment => "comentário",
            Role::Rejected => "forma recusada",
            Role::Reserved => "palavra reservada",
        }
    }
}

// ------------------------------------------------------------------ tabelas

/// Artigos: forma, definido, gênero, número.
pub const ARTICLES: &[(&str, bool, Gender, Number)] = &[
    ("o", true, Gender::Masculine, Number::Singular),
    ("a", true, Gender::Feminine, Number::Singular),
    ("os", true, Gender::Masculine, Number::Plural),
    ("as", true, Gender::Feminine, Number::Plural),
    ("um", false, Gender::Masculine, Number::Singular),
    ("uma", false, Gender::Feminine, Number::Singular),
    ("uns", false, Gender::Masculine, Number::Plural),
    ("umas", false, Gender::Feminine, Number::Plural),
];

/// Contrações obrigatórias: forma contraída, preposição, artigo.
pub const CONTRACTIONS: &[(&str, &str, &str)] = &[
    ("do", "de", "o"),
    ("da", "de", "a"),
    ("dos", "de", "os"),
    ("das", "de", "as"),
    ("no", "em", "o"),
    ("na", "em", "a"),
    ("nos", "em", "os"),
    ("nas", "em", "as"),
    ("ao", "a", "o"),
    ("à", "a", "a"),
    ("aos", "a", "os"),
    ("às", "a", "as"),
    ("pelo", "por", "o"),
    ("pela", "por", "a"),
    ("pelos", "por", "os"),
    ("pelas", "por", "as"),
    ("num", "em", "um"),
    ("numa", "em", "uma"),
    ("dum", "de", "um"),
    ("duma", "de", "uma"),
];

/// Preposições que a regência de um verbo pode pedir.
pub const PREPOSITIONS: &[&str] =
    &["a", "de", "em", "por", "com", "para", "sobre", "contra", "entre", "até", "sem", "desde", "após", "ante", "sob"];

/// Verbo da língua, conjugado na 1ª pessoa.
pub struct CoreVerb {
    pub infinitive: &'static str,
    pub first_person: &'static str,
    pub role: Role,
    pub doc: &'static str,
}

pub const CORE_VERBS: &[CoreVerb] = &[
    CoreVerb {
        infinitive: "escrever",
        first_person: "escrevo",
        role: Role::Statement,
        doc: "Escreve um valor na saída e quebra a linha: \"Escrevo o saldo.\"",
    },
    CoreVerb {
        infinitive: "ler",
        first_person: "leio",
        role: Role::Statement,
        doc: "Lê uma linha da entrada para um lugar (\"Leio o nome.\") ou como valor (\"leio um inteiro\").",
    },
    CoreVerb {
        infinitive: "tomar",
        first_person: "tomo",
        role: Role::Statement,
        doc: "Tira um item da coleção, com a posse dele: \"Tomo a última das notas.\" O item fica em \"o resultado\".",
    },
    CoreVerb {
        infinitive: "devolver",
        first_person: "devolvo",
        role: Role::Flow,
        doc: "Devolve o valor da função e sai dela: \"Devolvo o total.\"",
    },
    CoreVerb {
        infinitive: "desistir",
        first_person: "desisto",
        role: Role::Flow,
        doc: "A função falha com uma mensagem: \"Desisto com \"saldo insuficiente\".\" Só em função \"salvo uma falha\".",
    },
    CoreVerb { infinitive: "parar", first_person: "paro", role: Role::Flow, doc: "Sai do laço." },
    CoreVerb { infinitive: "pular", first_person: "pulo", role: Role::Flow, doc: "Passa para a próxima volta do laço." },
    CoreVerb {
        infinitive: "repetir",
        first_person: "repito",
        role: Role::Control,
        doc: "Laço contado (\"Repito 3 vezes:\") ou sem fim (\"Repito sempre:\", que sai com \"Paro\").",
    },
];

/// Verbo cujo sujeito é o dado, conjugado na 3ª pessoa.
pub struct SubjectVerb {
    pub infinitive: &'static str,
    pub role: Role,
    pub doc: &'static str,
}

pub const SUBJECT_VERBS: &[SubjectVerb] = &[
    SubjectVerb { infinitive: "ser", role: Role::Declaration, doc: "Declara um valor fixo: \"O limite é 100.\" O que \"é\" não muda." },
    SubjectVerb {
        infinitive: "começar",
        role: Role::Declaration,
        doc: "Declara um valor que muda: \"O saldo começa em 0.\", \"As notas começam vazias.\"",
    },
    SubjectVerb { infinitive: "virar", role: Role::Mutation, doc: "Dá um valor novo ao dado: \"O saldo vira 10.\"" },
    SubjectVerb { infinitive: "crescer", role: Role::Mutation, doc: "Soma ao dado: \"O saldo cresce 5.\"" },
    SubjectVerb { infinitive: "diminuir", role: Role::Mutation, doc: "Subtrai do dado: \"O saldo diminui 5.\"" },
    SubjectVerb { infinitive: "ganhar", role: Role::Mutation, doc: "Põe um item no fim da coleção: \"As notas ganham 7.\"" },
    SubjectVerb {
        infinitive: "perder",
        role: Role::Mutation,
        doc: "Tira um item da coleção, sem ficar com ele: \"As notas perdem a primeira.\"",
    },
];

/// O que uma operação por extenso calcula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NominalOp {
    Binary(BinOp),
    Unary(UnOp),
    /// "o deslocamento de x por n à esquerda".
    Shift,
}

pub struct Nominal {
    pub word: &'static str,
    pub gender: Gender,
    pub op: NominalOp,
    /// Aceita "entre" ("a soma entre x e y"); só as comutativas.
    pub commutative: bool,
    pub doc: &'static str,
}

pub const NOMINALS: &[Nominal] = &[
    Nominal {
        word: "soma",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::Add),
        commutative: true,
        doc: "\"a soma de x e y\": x mais y.",
    },
    Nominal {
        word: "diferença",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::Sub),
        commutative: false,
        doc: "\"a diferença de x e y\": x menos y.",
    },
    Nominal {
        word: "produto",
        gender: Gender::Masculine,
        op: NominalOp::Binary(BinOp::Mul),
        commutative: true,
        doc: "\"o produto de x e y\": x vezes y.",
    },
    Nominal {
        word: "quociente",
        gender: Gender::Masculine,
        op: NominalOp::Binary(BinOp::Div),
        commutative: false,
        doc: "\"o quociente de x e y\": x dividido por y.",
    },
    Nominal {
        word: "resto",
        gender: Gender::Masculine,
        op: NominalOp::Binary(BinOp::Rem),
        commutative: false,
        doc: "\"o resto de x e y\": o resto da divisão de x por y.",
    },
    Nominal {
        word: "potência",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::Pow),
        commutative: false,
        doc: "\"a potência de x e y\": x elevado a y.",
    },
    Nominal {
        word: "interseção",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::BitAnd),
        commutative: true,
        doc: "\"a interseção de x e y\": os bits ligados nos dois.",
    },
    Nominal {
        word: "união",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::BitOr),
        commutative: true,
        doc: "\"a união de x e y\": os bits ligados em qualquer um.",
    },
    Nominal {
        word: "disjunção",
        gender: Gender::Feminine,
        op: NominalOp::Binary(BinOp::BitXor),
        commutative: true,
        doc: "\"a disjunção de x e y\": os bits ligados em só um dos dois.",
    },
    Nominal {
        word: "negativo",
        gender: Gender::Masculine,
        op: NominalOp::Unary(UnOp::Neg),
        commutative: false,
        doc: "\"o negativo de x\": x com o sinal trocado.",
    },
    Nominal {
        word: "raiz",
        gender: Gender::Feminine,
        op: NominalOp::Unary(UnOp::Sqrt),
        commutative: false,
        doc: "\"a raiz de x\": a raiz quadrada de um real.",
    },
    Nominal {
        word: "tamanho",
        gender: Gender::Masculine,
        op: NominalOp::Unary(UnOp::Len),
        commutative: false,
        doc: "\"o tamanho de x\": quantos itens tem a lista, ou quantas letras tem o texto.",
    },
    Nominal {
        word: "inverso",
        gender: Gender::Masculine,
        op: NominalOp::Unary(UnOp::BitNot),
        commutative: false,
        doc: "\"o inverso de x\": os bits de x trocados.",
    },
    Nominal {
        word: "módulo",
        gender: Gender::Masculine,
        op: NominalOp::Unary(UnOp::Abs),
        commutative: false,
        doc: "\"o módulo de x\": o valor absoluto. Entre dois números, \"x módulo y\" é o resto.",
    },
    Nominal {
        word: "deslocamento",
        gender: Gender::Masculine,
        op: NominalOp::Shift,
        commutative: false,
        doc: "\"o deslocamento de x por n à esquerda\" (ou à direita): os bits de x andam n casas.",
    },
];

/// Substantivo de tipo da língua.
pub struct TypeNoun {
    pub singular: &'static str,
    pub plural: &'static str,
    pub gender: Gender,
    pub doc: &'static str,
}

pub const TYPE_NOUNS: &[TypeNoun] = &[
    TypeNoun {
        singular: "inteiro",
        plural: "inteiros",
        gender: Gender::Masculine,
        doc: "Número inteiro com sinal, de 64 bits (ou \"de 8/16/32 bits\").",
    },
    TypeNoun {
        singular: "natural",
        plural: "naturais",
        gender: Gender::Masculine,
        doc: "Número inteiro sem sinal, de 64 bits (ou \"de 8/16/32 bits\").",
    },
    TypeNoun { singular: "real", plural: "reais", gender: Gender::Masculine, doc: "Número com vírgula, de 64 bits (ou \"de 32 bits\")." },
    TypeNoun { singular: "byte", plural: "bytes", gender: Gender::Masculine, doc: "Natural de 8 bits." },
    TypeNoun { singular: "lógico", plural: "lógicos", gender: Gender::Masculine, doc: "Verdadeiro ou falso." },
    TypeNoun { singular: "caractere", plural: "caracteres", gender: Gender::Masculine, doc: "Uma letra, entre aspas simples: 'a'." },
    TypeNoun { singular: "texto", plural: "textos", gender: Gender::Masculine, doc: "Texto entre aspas: \"olá\"." },
    TypeNoun {
        singular: "lista",
        plural: "listas",
        gender: Gender::Feminine,
        doc: "Coleção de valores do mesmo tipo: \"uma lista de inteiros\".",
    },
];

/// Radicais dos ordinais e a posição de cada um; "último" tem posição 0.
pub const ORDINALS: &[(&str, u32)] = &[
    ("primeir", 1),
    ("segund", 2),
    ("terceir", 3),
    ("quart", 4),
    ("quint", 5),
    ("sext", 6),
    ("sétim", 7),
    ("oitav", 8),
    ("non", 9),
    ("décim", 10),
    ("últim", 0),
];

/// Locuções juntadas antes de desfazer as contrações: "pelo menos" não é "por o menos".
pub const PROTECTED_LOCUTIONS: &[(&str, &str)] = &[("pelo", "menos"), ("no", "máximo"), ("à", "esquerda"), ("à", "direita")];

/// Locuções cuja palavra final pode vir contraída ("igual ao", "seguido do").
pub const LOCUTIONS: &[(&str, &str)] = &[
    ("maior", "que"),
    ("menor", "que"),
    ("igual", "a"),
    ("diferente", "de"),
    ("maiores", "que"),
    ("menores", "que"),
    ("iguais", "a"),
    ("diferentes", "de"),
    ("dividido", "por"),
    ("seguido", "de"),
    ("seguida", "de"),
];

/// Particípios e adjetivos da língua, que concordam com o nome: base masculina singular.
pub const PARTICIPLES: &[(&str, Role, &str)] = &[
    ("emprestado", Role::Ownership, "Parâmetro emprestado: a função lê o valor, não o muda nem fica com ele (como &T)."),
    ("confiado", Role::Ownership, "Parâmetro confiado: a função pode mudar o valor, que volta para quem chamou (como &mut T)."),
    ("vazio", Role::Literal, "Coleção sem itens: \"As notas começam vazias.\""),
];

/// As demais palavras da língua: forma (locuções com espaço), papel, documentação.
pub struct Keyword {
    pub form: &'static str,
    pub role: Role,
    pub doc: &'static str,
}

pub const KEYWORDS: &[Keyword] = &[
    // Conjunções.
    Keyword {
        form: "e",
        role: Role::Conjunction,
        doc: "Liga orações (\"Escrevo x e paro.\"), termos de uma operação e os itens de uma lista.",
    },
    Keyword {
        form: "ou",
        role: Role::Conjunction,
        doc: "Liga condições e casos (\"Sendo um círculo ou um quadrado:\"); \", ou VALOR\" trata a falha de uma chamada.",
    },
    // Controle.
    Keyword { form: "se", role: Role::Control, doc: "Condição: \"Se o saldo for maior que 0:\"." },
    Keyword { form: "senão", role: Role::Control, doc: "O caso contrário de um \"Se\"; \"Senão, se\" encadeia outra condição." },
    Keyword { form: "enquanto", role: Role::Control, doc: "Laço com condição: \"Enquanto o saldo for positivo:\"." },
    Keyword {
        form: "cada",
        role: Role::Control,
        doc: "\"Para cada nota das notas:\" percorre a coleção; \"Para cada i de 1 até 10:\" conta.",
    },
    Keyword { form: "até", role: Role::Control, doc: "O fim do intervalo em \"Para cada i de 1 até 10:\"." },
    Keyword { form: "sempre", role: Role::Control, doc: "\"Repito sempre:\": laço sem fim, que sai com \"Paro\"." },
    Keyword { form: "vezes", role: Role::Control, doc: "\"Repito 3 vezes:\" conta as voltas; entre dois números, multiplica." },
    Keyword {
        form: "conforme",
        role: Role::Control,
        doc: "Examina uma enumeração caso a caso: \"Conforme a figura:\" seguido de \"Sendo ...:\".",
    },
    Keyword {
        form: "sendo",
        role: Role::Control,
        doc: "Um caso de \"Conforme\" (\"Sendo um círculo:\"); no cabeçalho, dá os tipos (\", sendo a quantia um real\").",
    },
    // Condição.
    Keyword { form: "for", role: Role::Copula, doc: "\"ser\" no futuro do subjuntivo, nas condições: \"Se o saldo for maior que 0\"." },
    Keyword { form: "forem", role: Role::Copula, doc: "\"ser\" no futuro do subjuntivo, no plural: \"Se as notas forem iguais\"." },
    Keyword { form: "houver", role: Role::Copula, doc: "\"Se houver apelido:\": o talvez tem valor, que vira um nome dentro do bloco." },
    // Definição.
    Keyword {
        form: "para",
        role: Role::Definition,
        doc: "Define uma função (\"Para transferir a quantia ...:\"), abre o programa (\"Para começar:\") ou um laço (\"Para cada\").",
    },
    Keyword { form: "começar", role: Role::Definition, doc: "\"Para começar:\" é onde o programa começa." },
    Keyword {
        form: "registro",
        role: Role::Definition,
        doc: "\"Uma conta é um registro de:\" define um registro, com os campos recuados.",
    },
    Keyword { form: "destas", role: Role::Definition, doc: "\"Uma forma é uma destas:\" define uma enumeração, com os casos recuados." },
    Keyword { form: "destes", role: Role::Definition, doc: "\"Um sinal é um destes:\" define uma enumeração de nome masculino." },
    Keyword {
        form: "com",
        role: Role::Definition,
        doc: "Os campos de um registro novo (\"uma conta com o titular \"Ana\" e o saldo 0\"), a coleção que começa com itens, a mensagem de \"Desisto com\".",
    },
    // Cabeçalho.
    Keyword {
        form: "devolvendo",
        role: Role::Signature,
        doc: "No cabeçalho, o tipo do valor que a função devolve: \", devolvendo um real\".",
    },
    Keyword { form: "salvo", role: Role::Signature, doc: "\", salvo uma falha\": a função pode desistir, e quem chama tem de tratar." },
    Keyword { form: "falha", role: Role::Signature, doc: "\", salvo uma falha\": a função pode desistir, e quem chama tem de tratar." },
    Keyword { form: "ambos", role: Role::Signature, doc: "Na tipagem, os dois regidos de uma vez: \", sendo ambos inteiros\"." },
    Keyword { form: "ambas", role: Role::Signature, doc: "Na tipagem, as duas regidas de uma vez: \", sendo ambas listas de reais\"." },
    // Posse.
    Keyword { form: "cópia", role: Role::Ownership, doc: "\"uma cópia de X\": entrega uma cópia, e o original continua com o dono." },
    Keyword { form: "talvez", role: Role::Ownership, doc: "Tipo que pode não ter valor: \"talvez um texto\", que começa em \"nada\"." },
    // Tipos.
    Keyword { form: "bits", role: Role::Type, doc: "O tamanho de um número: \"um inteiro de 32 bits\"." },
    // Operadores.
    Keyword { form: "mais", role: Role::Operator, doc: "Soma: \"x mais 1\"." },
    Keyword { form: "menos", role: Role::Operator, doc: "Subtração: \"x menos 1\"." },
    Keyword { form: "dividido por", role: Role::Operator, doc: "Divisão: \"x dividido por 2\"." },
    Keyword {
        form: "seguido de",
        role: Role::Operator,
        doc: "Junta textos: \"\"Olá, \" seguido do nome\". Liga mais frouxo que os argumentos de uma chamada.",
    },
    Keyword { form: "seguida de", role: Role::Operator, doc: "\"seguido de\" concordando com um nome feminino." },
    Keyword {
        form: "como",
        role: Role::Operator,
        doc: "Converte: \"x como real\", \"o saldo como texto\". Falha se o valor não cabe no tipo.",
    },
    Keyword { form: "não", role: Role::Operator, doc: "Nega a condição: \"Se o saldo não for positivo\", \"Se não houver apelido\"." },
    Keyword { form: "esquerda", role: Role::Operator, doc: "\"à esquerda\": a direção do deslocamento de bits." },
    Keyword { form: "direita", role: Role::Operator, doc: "\"à direita\": a direção do deslocamento de bits." },
    // Comparações.
    Keyword { form: "maior que", role: Role::Comparison, doc: "Comparação: \"Se o saldo for maior que 0\"." },
    Keyword { form: "maiores que", role: Role::Comparison, doc: "\"maior que\" no plural." },
    Keyword { form: "menor que", role: Role::Comparison, doc: "Comparação: \"Se o saldo for menor que 0\"." },
    Keyword { form: "menores que", role: Role::Comparison, doc: "\"menor que\" no plural." },
    Keyword { form: "igual a", role: Role::Comparison, doc: "Igualdade: \"Se o saldo for igual a 0\" (contrai: \"igual ao teto\")." },
    Keyword { form: "iguais a", role: Role::Comparison, doc: "\"igual a\" no plural." },
    Keyword { form: "diferente de", role: Role::Comparison, doc: "Desigualdade: \"Se o nome for diferente de \"Ana\"\"." },
    Keyword { form: "diferentes de", role: Role::Comparison, doc: "\"diferente de\" no plural." },
    Keyword { form: "pelo menos", role: Role::Comparison, doc: "Maior ou igual: \"Se a nota for pelo menos 7\"." },
    Keyword { form: "no máximo", role: Role::Comparison, doc: "Menor ou igual: \"Se a nota for no máximo 10\"." },
    // Valores.
    Keyword { form: "verdadeiro", role: Role::Literal, doc: "O lógico verdadeiro." },
    Keyword { form: "falso", role: Role::Literal, doc: "O lógico falso." },
    Keyword { form: "nada", role: Role::Literal, doc: "A falta de valor de um talvez: \"O apelido, talvez um texto, começa em nada.\"" },
    // Posições e retomadas.
    Keyword { form: "item", role: Role::Position, doc: "\"o item 2 das notas\": a posição, contando de 1." },
    Keyword {
        form: "resultado",
        role: Role::Anaphora,
        doc: "\"o resultado\": o valor da oração anterior; \"o resultado de dividir ...\" chama uma função como valor.",
    },
    Keyword { form: "fato", role: Role::Anaphora, doc: "\"o fato de o saldo ser positivo\": uma condição vista como lógico." },
    // Saída e comentários.
    Keyword {
        form: "sem quebrar a linha",
        role: Role::Output,
        doc: "\"Escrevo x sem quebrar a linha.\": a próxima escrita continua na mesma linha.",
    },
    Keyword {
        form: "nota",
        role: Role::Comment,
        doc: "\"Nota:\" no começo da linha faz dela um comentário; \"--\" comenta até o fim da linha.",
    },
    // Formas recusadas, com a dica que a língua dá.
    Keyword {
        form: "seja",
        role: Role::Rejected,
        doc: "Quem se declara é o sujeito: \"O saldo é 0.\" (fixo) ou \"O saldo começa em 0.\" (variável).",
    },
    Keyword {
        form: "sejam",
        role: Role::Rejected,
        doc: "Quem se declara é o sujeito: \"O saldo é 0.\" (fixo) ou \"O saldo começa em 0.\" (variável).",
    },
    Keyword { form: "fosse", role: Role::Rejected, doc: "Numa condição o verbo vai no futuro do subjuntivo: \"for\"." },
    Keyword { form: "fossem", role: Role::Rejected, doc: "Numa condição o verbo vai no futuro do subjuntivo: \"forem\"." },
    Keyword { form: "está", role: Role::Rejected, doc: "Numa condição o verbo vai no futuro do subjuntivo: \"for\"." },
    Keyword { form: "estão", role: Role::Rejected, doc: "Numa condição o verbo vai no futuro do subjuntivo: \"forem\"." },
    Keyword { form: "há", role: Role::Rejected, doc: "Numa condição, \"houver\": \"Se houver apelido\"." },
    Keyword {
        form: "ir",
        role: Role::Rejected,
        doc: "Predicado com \"ir\" não pode: no subjuntivo ele dá \"for\", igual a \"ser\", que a língua usa nas comparações.",
    },
    Keyword {
        form: "vez", role: Role::Rejected, doc: "\"Repito\" conta em \"vezes\", no plural; para uma vez só, não precisa de laço."
    },
    Keyword { form: "guardo", role: Role::Rejected, doc: "Para mudar um valor, o dado é o sujeito: \"O saldo vira 10.\"" },
    Keyword { form: "acrescento", role: Role::Rejected, doc: "Para somar ao valor, o dado é o sujeito: \"O saldo cresce 10.\"" },
    Keyword { form: "subtraio", role: Role::Rejected, doc: "Para tirar do valor, o dado é o sujeito: \"O saldo diminui 10.\"" },
    // Reservadas sem uso na sintaxe.
    Keyword {
        form: "caso",
        role: Role::Reserved,
        doc: "Nome que a língua dá aos itens de uma enumeração; não pode ser a 1ª pessoa de um verbo do usuário.",
    },
];

// ------------------------------------------------------------------ consultas

/// Verbo da língua pelo infinitivo (inclui os verbos do sujeito): não se redefine.
pub fn is_core_infinitive(w: &str) -> bool {
    CORE_VERBS.iter().any(|v| v.infinitive == w) || SUBJECT_VERBS.iter().any(|v| v.infinitive == w)
}

/// Verbo da língua na 1ª pessoa que abre uma oração depois de "e" ou vírgula.
pub fn is_clause_verb(w: &str) -> bool {
    CORE_VERBS.iter().any(|v| v.first_person == w && v.role != Role::Control)
}

/// Os verbos da língua que abrem oração, pelo infinitivo (para as dicas de conjugação).
pub fn clause_verb_infinitives() -> impl Iterator<Item = &'static str> {
    CORE_VERBS.iter().filter(|v| v.role != Role::Control).map(|v| v.infinitive)
}

/// Os mesmos, na 1ª pessoa.
pub fn clause_verb_forms() -> impl Iterator<Item = &'static str> {
    CORE_VERBS.iter().filter(|v| v.role != Role::Control).map(|v| v.first_person)
}

pub fn nominal(w: &str) -> Option<&'static Nominal> {
    NOMINALS.iter().find(|n| n.word == w)
}

pub fn type_noun(singular: &str) -> Option<&'static TypeNoun> {
    TYPE_NOUNS.iter().find(|t| t.singular == singular)
}

/// Plural de um tipo da língua → o tipo.
pub fn type_noun_by_plural(plural: &str) -> Option<&'static TypeNoun> {
    TYPE_NOUNS.iter().find(|t| t.plural == plural)
}

/// A dica que a língua dá para uma forma recusada.
pub fn rejected_hint(w: &str) -> Option<&'static str> {
    KEYWORDS.iter().find(|k| k.role == Role::Rejected && k.form == w).map(|k| k.doc)
}

// ------------------------------------------------------------------ entradas

/// Uma forma escrita da língua, com o papel e a documentação.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Em minúsculas; locuções com um espaço entre as palavras.
    pub form: String,
    /// A palavra de dicionário: o infinitivo de um verbo, o singular masculino.
    pub lemma: String,
    pub role: Role,
    pub doc: String,
}

/// Todas as formas da língua, na ordem das tabelas.
pub fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(build_entries)
}

fn build_entries() -> Vec<Entry> {
    let mut out = Vec::new();
    let mut push = |form: &str, lemma: &str, role: Role, doc: String| {
        out.push(Entry { form: form.to_string(), lemma: lemma.to_string(), role, doc });
    };
    for &(form, definite, g, n) in ARTICLES {
        let kind = if definite { "definido" } else { "indefinido" };
        let doc = if definite {
            format!("Artigo {kind} {}: marca o nome de um valor (\"{form} saldo\").", morphology::describe_gn(g, n))
        } else {
            format!("Artigo {kind} {}: apresenta um tipo ou um caso (\"{form} inteiro\").", morphology::describe_gn(g, n))
        };
        push(form, form, Role::Article, doc);
    }
    for &(form, prep, art) in CONTRACTIONS {
        push(form, form, Role::Contraction, format!("\"{prep}\" + \"{art}\": a contração é obrigatória antes de nome com artigo."));
    }
    for &p in PREPOSITIONS {
        push(
            p,
            p,
            Role::Preposition,
            "Preposição: rege um argumento no cabeçalho e na chamada (\"transferir a quantia para o destino\").".into(),
        );
    }
    for v in CORE_VERBS {
        push(v.first_person, v.infinitive, v.role, v.doc.into());
        push(v.infinitive, v.infinitive, v.role, v.doc.into());
    }
    for v in SUBJECT_VERBS {
        for n in [Number::Singular, Number::Plural] {
            if let Some(f) = morphology::third_person(v.infinitive, n) {
                push(&f, v.infinitive, v.role, v.doc.into());
            }
        }
        push(v.infinitive, v.infinitive, v.role, v.doc.into());
    }
    for t in TYPE_NOUNS {
        push(t.singular, t.singular, Role::Type, t.doc.into());
        push(t.plural, t.singular, Role::Type, t.doc.into());
    }
    for n in NOMINALS {
        push(n.word, n.word, Role::Nominal, n.doc.into());
    }
    for &(stem, value) in ORDINALS {
        let lemma = format!("{stem}o");
        let doc = if value == 0 {
            "\"a última das notas\": o último item da coleção.".to_string()
        } else {
            format!("\"a {stem}a das notas\": o item {value} da coleção.")
        };
        for end in ["o", "a", "os", "as"] {
            push(&format!("{stem}{end}"), &lemma, Role::Position, doc.clone());
        }
    }
    for &(base, role, doc) in PARTICIPLES {
        for g in [Gender::Masculine, Gender::Feminine] {
            for n in [Number::Singular, Number::Plural] {
                push(&morphology::inflect_adjective(base, Gender::Masculine, g, n), base, role, doc.into());
            }
        }
    }
    for k in KEYWORDS {
        push(k.form, k.form, k.role, k.doc.into());
    }
    out
}

fn index() -> &'static HashMap<String, Vec<usize>> {
    static INDEX: OnceLock<HashMap<String, Vec<usize>>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, e) in entries().iter().enumerate() {
            map.entry(e.form.clone()).or_default().push(i);
        }
        map
    })
}

/// As entradas de uma forma escrita (uma palavra pode ter vários papéis).
pub fn lookup(form: &str) -> Vec<&'static Entry> {
    index().get(form).map(|v| v.iter().map(|&i| &entries()[i]).collect()).unwrap_or_default()
}

/// Se a palavra é da língua e não pode ser a 1ª pessoa de um verbo do usuário, a
/// entrada que a reserva. Vale a palavra inteira e cada palavra de uma locução; as
/// formas recusadas com dica não reservam nada.
pub fn reserved(word: &str) -> Option<&'static Entry> {
    static WORDS: OnceLock<HashMap<String, usize>> = OnceLock::new();
    let words = WORDS.get_or_init(|| {
        let mut map = HashMap::new();
        for (i, e) in entries().iter().enumerate() {
            if e.role == Role::Rejected {
                continue;
            }
            for w in e.form.split(' ') {
                map.entry(w.to_string()).or_insert(i);
            }
        }
        map
    });
    words.get(word).map(|&i| &entries()[i])
}

// ------------------------------------------------------------------ exportação

fn gender_id(g: Gender) -> &'static str {
    match g {
        Gender::Masculine => "masculine",
        Gender::Feminine => "feminine",
    }
}

fn number_id(n: Number) -> &'static str {
    match n {
        Number::Singular => "singular",
        Number::Plural => "plural",
    }
}

/// O léxico inteiro em JSON, para as gramáticas dos editores, o conferidor da
/// morfologia e o site. O formato é estável: só ganha campos.
pub fn to_json() -> String {
    use serde_json::{Value, json};
    let subject: Vec<Value> = SUBJECT_VERBS
        .iter()
        .map(|v| {
            json!({
                "infinitive": v.infinitive,
                "singular": morphology::third_person(v.infinitive, Number::Singular),
                "plural": morphology::third_person(v.infinitive, Number::Plural),
                "role": v.role.id(),
                "doc": v.doc,
            })
        })
        .collect();
    let participles: Vec<Value> = PARTICIPLES
        .iter()
        .map(|&(base, role, doc)| {
            let forms: Vec<String> = [Gender::Masculine, Gender::Feminine]
                .iter()
                .flat_map(|&g| [Number::Singular, Number::Plural].map(|n| morphology::inflect_adjective(base, Gender::Masculine, g, n)))
                .collect();
            json!({ "base": base, "forms": forms, "role": role.id(), "doc": doc })
        })
        .collect();
    let value = json!({
        "version": 1,
        "roles": Role::ALL.iter().map(|r| json!({ "id": r.id(), "name": r.describe() })).collect::<Vec<_>>(),
        "articles": ARTICLES.iter().map(|&(form, definite, g, n)| json!({
            "form": form, "definite": definite, "gender": gender_id(g), "number": number_id(n),
        })).collect::<Vec<_>>(),
        "contractions": CONTRACTIONS.iter().map(|&(form, prep, art)| json!({
            "form": form, "preposition": prep, "article": art,
        })).collect::<Vec<_>>(),
        "prepositions": PREPOSITIONS,
        "coreVerbs": CORE_VERBS.iter().map(|v| json!({
            "infinitive": v.infinitive, "firstPerson": v.first_person, "role": v.role.id(), "doc": v.doc,
        })).collect::<Vec<_>>(),
        "subjectVerbs": subject,
        "types": TYPE_NOUNS.iter().map(|t| json!({
            "singular": t.singular, "plural": t.plural, "gender": gender_id(t.gender), "doc": t.doc,
        })).collect::<Vec<_>>(),
        "nominals": NOMINALS.iter().map(|n| json!({
            "word": n.word,
            "gender": gender_id(n.gender),
            "arity": match n.op { NominalOp::Binary(_) => 2, NominalOp::Unary(_) => 1, NominalOp::Shift => 2 },
            "commutative": n.commutative,
            "doc": n.doc,
        })).collect::<Vec<_>>(),
        "ordinals": ORDINALS.iter().map(|&(stem, position)| json!({ "stem": stem, "position": position })).collect::<Vec<_>>(),
        "participles": participles,
        "locutions": {
            "protected": PROTECTED_LOCUTIONS.iter().map(|&(a, b)| [a, b]).collect::<Vec<_>>(),
            "contractible": LOCUTIONS.iter().map(|&(a, b)| [a, b]).collect::<Vec<_>>(),
        },
        "keywords": KEYWORDS.iter().map(|k| json!({ "form": k.form, "role": k.role.id(), "doc": k.doc })).collect::<Vec<_>>(),
        "words": entries().iter().map(|e| json!({
            "form": e.form, "lemma": e.lemma, "role": e.role.id(), "doc": e.doc,
        })).collect::<Vec<_>>(),
    });
    let mut s = serde_json::to_string_pretty(&value).expect("o léxico vira JSON");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os literais de palavra que um arquivo do parser consulta: argumentos de
    /// `at_word`, `eat_word`, `expect_word` e `is_word`, comparações `w == "..."`,
    /// `adjective_form(_, "...")` e padrões de `match` e `matches!`.
    fn consulted_words(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        let is_word =
            |s: &str| !s.is_empty() && s.chars().all(|c| c.is_lowercase() || c == ' ') && !s.starts_with(' ') && !s.ends_with(' ');
        let literal_after = |line: &str, start: usize| -> Option<String> {
            let rest = &line[start..];
            let end = rest.find('"')?;
            Some(rest[..end].to_string())
        };
        for line in src.lines() {
            let code = line.split("//").next().unwrap_or("");
            for marker in [
                "at_word(\"",
                "eat_word(\"",
                "expect_word(\"",
                "is_word(\"",
                "w == \"",
                "n == \"",
                "sing == \"",
                "word == \"",
                "infinitive == \"",
                "adjective_form(",
            ] {
                let mut from = 0;
                while let Some(i) = code[from..].find(marker) {
                    let at = from + i + marker.len();
                    let lit = if marker == "adjective_form(" {
                        code[at..].find(", \"").and_then(|j| literal_after(code, at + j + 3))
                    } else {
                        literal_after(code, at)
                    };
                    if let Some(l) = lit.filter(|l| is_word(l)) {
                        out.push(l);
                    }
                    from = at;
                }
            }
            // Padrões: `"a" | "b" =>` no começo da linha, e `matches!(w, "a" | "b")`.
            let trimmed = code.trim_start();
            let pattern = if trimmed.starts_with('"') && trimmed.contains("=>") {
                Some(&trimmed[..trimmed.find("=>").unwrap()])
            } else if let Some(i) = code.find("matches!(") {
                let inner = &code[i..];
                inner.find(", \"").map(|j| {
                    let tail = &inner[j + 2..];
                    &tail[..tail.find(')').unwrap_or(tail.len())]
                })
            } else {
                None
            };
            if let Some(p) = pattern {
                for part in p.split('|') {
                    let part = part.trim();
                    if let Some(l) = part.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
                        && is_word(l)
                    {
                        out.push(l.to_string());
                    }
                }
            }
        }
        out
    }

    #[test]
    fn the_parser_only_consults_words_of_the_lexicon() {
        let files = [
            ("parser/mod.rs", include_str!("parser/mod.rs")),
            ("parser/header.rs", include_str!("parser/header.rs")),
            ("parser/data.rs", include_str!("parser/data.rs")),
        ];
        let mut missing = Vec::new();
        let mut seen = 0;
        for (name, src) in files {
            for w in consulted_words(src) {
                seen += 1;
                // Vale a forma inteira, ou cada palavra dela como parte de uma locução.
                if lookup(&w).is_empty() && !w.split(' ').all(|p| !lookup(p).is_empty() || reserved(p).is_some()) {
                    missing.push(format!("{name}: \"{w}\""));
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(seen > 100, "a varredura do parser achou poucas palavras ({seen})");
        assert!(missing.is_empty(), "o parser consulta palavras que não estão no léxico:\n{}", missing.join("\n"));
    }

    #[test]
    fn reserved_forms_cover_the_language() {
        // As formas que sempre foram reservadas continuam reservadas.
        for w in [
            "como",
            "caso",
            "pelo",
            "salvo",
            "para",
            "se",
            "senão",
            "enquanto",
            "cada",
            "mais",
            "menos",
            "vezes",
            "módulo",
            "talvez",
            "nada",
            "sempre",
            "quarto",
            "quinto",
            "oitavo",
            "segundo",
            "terceiro",
            "sexto",
            "sétimo",
            "nono",
            "décimo",
            "último",
            "primeiro",
            "falso",
            "verdadeiro",
            "inteiro",
            "natural",
            "real",
            "texto",
            "registro",
            "resto",
            "produto",
            "quociente",
            "tamanho",
            "negativo",
            "inverso",
            "deslocamento",
            "item",
            "resultado",
            "fato",
        ] {
            assert!(reserved(w).is_some(), "\"{w}\" deixou de ser reservada");
        }
        // As recusadas com dica ficam livres para o usuário.
        for w in ["guardo", "acrescento", "subtraio", "seja", "há", "vez"] {
            assert!(reserved(w).is_none(), "\"{w}\" não deve ser reservada");
        }
    }

    #[test]
    fn every_entry_is_documented() {
        for e in entries() {
            assert!(
                e.doc.ends_with('.') || e.doc.ends_with('"') || e.doc.ends_with(')'),
                "\"{}\": a documentação termina em ponto",
                e.form
            );
            assert!(!e.form.is_empty() && e.form == e.form.to_lowercase(), "\"{}\": forma em minúsculas", e.form);
        }
    }

    #[test]
    fn exported_lexicon_is_current() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/editors/lexicon.json");
        let now = to_json();
        if std::env::var_os("LAVRA_BLESS").is_some() {
            std::fs::write(path, &now).unwrap();
            return;
        }
        let saved = std::fs::read_to_string(path).unwrap_or_default();
        assert!(saved == now, "editors/lexicon.json está desatualizado: rode `LAVRA_BLESS=1 cargo test --lib lexicon` e revise o diff");
    }
}
