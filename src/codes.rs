//! Os códigos dos diagnósticos: estáveis, num espaço só ("L0001" em diante) e
//! independentes da categoria. Um erro que muda de etapa ou de categoria guarda o código;
//! um código nunca é reaproveitado para outro erro.
//!
//! Cada ponto que cria um diagnóstico passa o código (`Diagnostic::new(L0334, ...)`); a
//! categoria vem daqui. `lavrac explain L0334` mostra o texto de `src/explain/L0334.md`,
//! com um exemplo errado e o certo; o catálogo do site sai desta lista.
//!
//! As faixas só dão ordem à atribuição: L00xx léxico, L01xx e L02xx parser, L03xx
//! análise semântica, L04xx posse e empréstimo, L05xx avisos, L09xx execução.
//!
//! O runtime dos executáveis inclui este arquivo, por isso ele só depende de
//! `diagnostics`.

use crate::diagnostics::Category;

/// Um código de diagnóstico.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Code {
    pub id: &'static str,
    pub category: Category,
    /// Uma linha que nomeia o problema, para o catálogo.
    pub title: &'static str,
}

impl Code {
    /// O número do código ("L0904" dá 904), que o executável nativo recebe.
    pub fn number(self) -> u32 {
        self.id[1..].parse().unwrap_or(0)
    }
}

macro_rules! codes {
    ($($name:ident = $cat:ident, $title:literal;)*) => {
        $(pub const $name: Code = Code { id: stringify!($name), category: Category::$cat, title: $title };)*
        /// Todos os códigos, na ordem do registro.
        pub const ALL: &[Code] = &[$($name),*];
    };
}

codes! {
    // Léxico.
    L0001 = Lexical, "Sinal que não faz parte da Lavra";
    L0002 = Lexical, "Texto sem aspas de fechamento";
    L0003 = Lexical, "Caractere sem aspas simples de fechamento";
    L0004 = Lexical, "Caractere com mais de uma letra";
    L0005 = Lexical, "Barra invertida no fim da linha";
    L0006 = Lexical, "Escape desconhecido";
    L0007 = Lexical, "Número grande demais";
    L0008 = Lexical, "Hexadecimal sem dígitos";
    L0009 = Lexical, "Hexadecimal com sinal";
    L0010 = Lexical, "Número real malformado";
    L0011 = Lexical, "Número colado numa palavra";
    L0012 = Lexical, "Recuo inconsistente";

    // Parser: períodos, orações e condições.
    L0100 = Syntax, "Faltou o ponto final";
    L0101 = Syntax, "Esperava outra palavra ou sinal";
    L0102 = Syntax, "Dois \"Para começar\"";
    L0103 = Syntax, "Faltou o bloco recuado depois dos dois-pontos";
    L0104 = Agreement, "\"Repito\" com \"vez\" no singular";
    L0105 = Morphology, "\"Para cada\" com nome igual no singular e no plural";
    L0106 = Syntax, "\"Senão\" sem \"Se\"";
    L0107 = Syntax, "Faltou o nome do item em \"Para cada\"";
    L0108 = Syntax, "Artigo depois de \"cada\"";
    L0109 = Syntax, "Caso de \"Conforme\" sem \"Sendo\"";
    L0110 = Syntax, "\"Senão se\" sem vírgula";
    L0111 = Syntax, "Nome depois de \"houver\" com artigo ou faltando";
    L0112 = Morphology, "\"há\" numa condição";
    L0113 = Agreement, "Verbo que não concorda com o sujeito";
    L0114 = Morphology, "Verbo da condição fora do futuro do subjuntivo";
    L0115 = Agreement, "Adjetivo do predicado que não concorda";
    L0116 = Agreement, "Comparação que não concorda";
    L0117 = Regency, "Comparação com a preposição errada";
    L0118 = Scope, "Campo que nenhum registro tem";
    L0119 = Syntax, "\"Desisto\" sem o motivo";
    L0120 = Morphology, "Verbo na 3ª pessoa onde o programa age";
    L0121 = Morphology, "Verbo no infinitivo onde o programa age";
    L0122 = Syntax, "Verbo desconhecido";
    L0123 = Syntax, "Sujeito sem verbo";
    L0124 = Syntax, "Declaração de campo ou item";
    L0125 = Syntax, "Aposto fora da declaração";
    L0126 = Syntax, "\"talvez talvez\"";
    L0127 = Syntax, "Tipo sem artigo indefinido";
    L0128 = Agreement, "Artigo que não concorda com o tipo";
    L0129 = Type, "Inteiro com tamanho inválido";
    L0130 = Morphology, "Contração obrigatória";
    L0131 = Syntax, "Faltou o nome depois do artigo";
    L0132 = Syntax, "Tipo com artigo depois de \"como\"";
    L0133 = Syntax, "Operação por extenso misturada com outra conta sem parênteses";
    L0134 = Morphology, "\"de\" contraído antes de verbo";
    L0135 = Morphology, "Contração antes do sujeito de \"o fato de\"";
    L0136 = Agreement, "\"cópia\" no masculino";

    // Parser: cabeçalhos e chamadas.
    L0150 = Morphology, "Predicado com \"ir\"";
    L0151 = Morphology, "Verbo da própria língua redefinido";
    L0152 = Morphology, "1ª pessoa que já é palavra da língua";
    L0153 = Morphology, "1ª pessoa igual a verbo da língua";
    L0154 = Morphology, "Dois verbos com a mesma 1ª pessoa";
    L0155 = Scope, "Mesma regência definida duas vezes";
    L0156 = Morphology, "Cabeçalho sem verbo no infinitivo";
    L0157 = Morphology, "Verbo defectivo sem 1ª pessoa";
    L0158 = Syntax, "Tipo devolvido sem artigo indefinido";
    L0159 = Syntax, "\"salvo uma falha\" malformado";
    L0160 = Scope, "Nome tipado que não está no cabeçalho";
    L0161 = Type, "Predicado que não devolve lógico";
    L0162 = Syntax, "\"Ambos\" sem dois parâmetros";
    L0163 = Agreement, "\"Ambos\" que não concorda";
    L0164 = Agreement, "Tipagem que não concorda com o cabeçalho";
    L0165 = Agreement, "Vários parâmetros com tipo no singular";
    L0167 = Agreement, "Particípio que não concorda com o tipo";
    L0168 = Type, "Tipo desconhecido";
    L0169 = Agreement, "Tipo no plural depois de artigo singular";
    L0170 = Type, "Real com tamanho inválido";
    L0171 = Regency, "Chamada fora da regência";
    L0172 = Type, "Parâmetro sem tipo";

    // Parser: registros, enumerações, lugares e operações por extenso.
    L0200 = Syntax, "Faltou o nome do tipo";
    L0201 = Agreement, "Tipo definido no plural";
    L0202 = Scope, "Tipo definido duas vezes";
    L0203 = Agreement, "\"registro\" no feminino";
    L0204 = Scope, "Campo repetido na definição";
    L0205 = Morphology, "Campo com nome de operação da língua";
    L0206 = Agreement, "\"destas\" ou \"destes\" que não concorda";
    L0207 = Syntax, "Caso sem artigo indefinido";
    L0208 = Syntax, "Faltou o nome do caso";
    L0209 = Scope, "Caso com nome já usado";
    L0210 = Syntax, "Campo de caso sem artigo";
    L0211 = Syntax, "Posição sem a coleção";
    L0212 = Agreement, "Ordinal que não concorda com o artigo";
    L0213 = Syntax, "Posição desconhecida";
    L0214 = Agreement, "\"vazio\" que não concorda";
    L0215 = Agreement, "Operação por extenso com o artigo errado";
    L0216 = Syntax, "Deslocamento sem direção";
    L0217 = Regency, "\"entre\" em operação não comutativa";
    L0218 = Scope, "Caso que nenhuma enumeração tem";
    L0219 = Agreement, "Caso sem artigo indefinido";
    L0220 = Agreement, "Caso com o artigo do gênero errado";

    // Análise semântica.
    L0300 = Syntax, "Programa sem \"Para começar\"";
    L0301 = Type, "Função que pode terminar sem devolver";
    L0302 = Type, "Tipo que contém a si mesmo";
    L0303 = Type, "Registro recursivo por lista";
    L0304 = Type, "Tipo ainda não suportado";
    L0305 = Type, "Lista sem o tipo dos itens";
    L0306 = Type, "\"Repito\" com contagem que não é inteira";
    L0307 = Type, "Intervalo de \"Para cada\" que não é de inteiros";
    L0308 = Scope, "\"Para cada\" sem a coleção declarada";
    L0309 = Type, "\"Conforme\" sobre valor que não é enumeração";
    L0310 = Syntax, "Caso tratado duas vezes";
    L0311 = Type, "\"Conforme\" sem todos os casos";
    L0312 = Scope, "Nome declarado duas vezes no bloco";
    L0313 = Scope, "Nome igual a constante do programa";
    L0314 = Morphology, "Nome com uma letra só";
    L0315 = Type, "Vários valores com aposto que não é lista";
    L0316 = Type, "\"vazio\" para quem não é lista nem texto";
    L0317 = Agreement, "Nome no plural com um valor só";
    L0318 = Agreement, "Nome no singular com uma coleção";
    L0319 = Type, "\"ganha\" em quem não é lista";
    L0320 = Type, "\"perde\" ou \"Tomo\" em quem não é lista";
    L0321 = Type, "\"cresce\" ou \"diminui\" em quem não é número";
    L0322 = Type, "Valor de função que não devolve nada";
    L0323 = Type, "Leitura de um tipo que não se lê";
    L0324 = Syntax, "\"Paro\" ou \"Pulo\" fora de laço";
    L0325 = Type, "\"Devolvo\" com valor em função que não devolve";
    L0326 = Type, "\"Devolvo\" sem valor em função que devolve";
    L0327 = Syntax, "\", ou ...\" em chamada que não falha";
    L0328 = Type, "Chamada que pode falhar sem tratamento";
    L0329 = Borrow, "Valor solto onde o parâmetro é confiado";
    L0330 = Type, "Comparação de ordem entre valores sem ordem";
    L0331 = Type, "Caso perguntado de valor que não é enumeração";
    L0332 = Syntax, "Condição ainda não suportada";
    L0333 = Type, "Caso de outra enumeração";
    L0334 = Scope, "Nome não declarado";
    L0335 = Type, "\"houver\" de valor que não é talvez";
    L0336 = Agreement, "Artigo que não concorda com a declaração";
    L0337 = Type, "Campo de valor que não é registro";
    L0338 = Scope, "Campo que o registro não tem";
    L0339 = Agreement, "Artigo do campo que não concorda";
    L0340 = Type, "Item de coleção de tipo ainda desconhecido";
    L0341 = Type, "Item de valor que não é coleção";
    L0342 = Mutability, "Mudança de valor fixo";
    L0343 = Type, "Texto mudado letra por letra";
    L0344 = Type, "Número que não cabe no tipo";
    L0345 = Scope, "\"o resultado\" sem oração anterior com valor";
    L0346 = Type, "Chamada sem valor usada como valor";
    L0347 = Type, "Conversão impossível";
    L0348 = Type, "\"nada\" onde não se espera talvez";
    L0349 = Type, "Tamanho de quem não é lista nem texto";
    L0350 = Type, "Operação com tipo errado";
    L0351 = Syntax, "Campo repetido na construção";
    L0352 = Type, "Construção sem todos os campos";
    L0353 = Syntax, "Expressão ainda não suportada";
    L0354 = Type, "Valor de tipo diferente do esperado";
    L0355 = Type, "Valor de reserva para função que não devolve";
    L0356 = Syntax, "\", ou desisto\" com motivo";
    L0357 = Type, "Desistir em função que não falha";
    L0358 = Type, "Texto somado";
    L0359 = Type, "Conta com quem não é número";
    L0360 = Type, "Operação de bits com quem não é inteiro";
    L0361 = Type, "Conta entre tipos diferentes";
    L0362 = Syntax, "Ação no nível de fora";

    // Posse e empréstimo.
    L0400 = Ownership, "Valor usado depois de entregue";
    L0401 = Borrow, "Valor mudado enquanto está emprestado";
    L0402 = Ownership, "Constante entregue";
    L0403 = Ownership, "Valor tirado de dentro de registro ou lista";
    L0404 = Ownership, "Vista emprestada entregue";
    L0405 = Borrow, "Valor entregue enquanto está emprestado";
    L0406 = Borrow, "O mesmo valor duas vezes na mesma chamada";

    // Avisos.
    L0500 = Warning, "\"começa\" que nunca muda";
    L0501 = Warning, "\"Senão\" que nunca acontece";
    L0502 = Warning, "Função que nunca é chamada";
    L0503 = Warning, "Nome que nunca é usado";

    // Execução.
    L0900 = Runtime, "Estouro de inteiro";
    L0901 = Runtime, "Estouro na potência";
    L0902 = Runtime, "Estouro no módulo";
    L0903 = Runtime, "Deslocamento fora da faixa";
    L0904 = Runtime, "Divisão por zero";
    L0905 = Runtime, "Resto por zero";
    L0906 = Runtime, "Potência com expoente negativo";
    L0907 = Runtime, "Raiz de número negativo";
    L0908 = Runtime, "Conversão que não cabe";
    L0909 = Runtime, "Posição fora da coleção";
    L0910 = Runtime, "\"o último\" de coleção vazia";
    L0911 = Runtime, "Tirar item de coleção vazia";
    L0912 = Runtime, "A entrada acabou";
    L0913 = Runtime, "Entrada que não é do tipo pedido";
    L0914 = Runtime, "Chamadas demais, uma dentro da outra";
}

/// O código pelo identificador ("L0334") ou pelo número (334).
pub fn find(id: &str) -> Option<Code> {
    let id = id.trim().to_uppercase();
    let id = if id.starts_with('L') { id } else { format!("L{:0>4}", id) };
    ALL.iter().copied().find(|c| c.id == id)
}

/// O código pelo número, para o runtime ("904" dá L0904).
pub fn by_number(n: u32) -> Option<Code> {
    ALL.iter().copied().find(|c| c.number() == n)
}
