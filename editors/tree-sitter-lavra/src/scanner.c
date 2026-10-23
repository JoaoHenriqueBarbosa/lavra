/**
 * O scanner externo da Lavra: o recuo (INDENT e DEDENT), o fim da linha lógica (NEWLINE),
 * os comentários e o conteúdo dos textos. Segue o léxico do compilador (`src/lexer.rs`):
 *
 * - comentário é a linha cujo conteúdo começa com "Nota:" (sem distinguir maiúsculas) ou
 *   com "--"; o "--" no meio da linha corta o resto dela;
 * - a linha só termina a linha lógica se acabar em ".", ":" ou ";"; senão continua na de
 *   baixo, com recuo livre (enjambement). Aqui isso vira: o NEWLINE só sai onde a gramática
 *   o espera, que é logo depois desses sinais, e só no fim da linha;
 * - linhas em branco e comentários não contam para o recuo; o tab vale quatro espaços.
 *
 * Os tokens de recuo e de fim de linha têm largura zero e ficam no fim da linha que os
 * provoca: o scanner olha adiante para decidir, sem consumir. A indentação da próxima linha
 * de código, calculada quando sai o NEWLINE, fica no estado. Com ela, o DEDENT sai antes dos
 * comentários que já são do bloco de fora, e os comentários alinhados com o bloco ficam
 * dentro dele.
 */

#include "tree_sitter/alloc.h"
#include "tree_sitter/array.h"
#include "tree_sitter/parser.h"

#include <string.h>

enum TokenType {
  NEWLINE,
  INDENT,
  DEDENT,
  STRING_CONTENT,
  COMMENT,
  ERROR_SENTINEL,
};

typedef struct {
  /// Os recuos dos blocos abertos, em colunas; o primeiro é a margem (0).
  Array(uint16_t) indents;
  /// A indentação da próxima linha de código depois do último NEWLINE; -1 enquanto
  /// ninguém a calculou.
  int32_t next_indent;
} Scanner;

static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static inline void skip(TSLexer *lexer) { lexer->advance(lexer, true); }

static inline bool is_quote(int32_t c) { return c == '"' || c == 0x201C || c == 0x201D; }

static inline bool at_line_end(TSLexer *lexer) {
  return lexer->eof(lexer) || lexer->lookahead == '\n' || lexer->lookahead == '\r';
}

/// Consome "nota:" sem distinguir maiúsculas; falso se o texto não for esse.
static bool consume_nota(TSLexer *lexer) {
  static const char *const word = "nota:";
  for (const char *p = word; *p; p++) {
    int32_t c = lexer->lookahead;
    if (c >= 'A' && c <= 'Z') {
      c += 'a' - 'A';
    }
    if (c != *p) {
      return false;
    }
    advance(lexer);
  }
  return true;
}

/// O resto do comentário, até o fim da linha.
static bool finish_comment(TSLexer *lexer) {
  while (!at_line_end(lexer)) {
    advance(lexer);
  }
  lexer->mark_end(lexer);
  lexer->result_symbol = COMMENT;
  return true;
}

/// A partir do fim de uma linha, pula as linhas em branco e as de comentário e devolve a
/// indentação da próxima linha de código (0 no fim do texto).
static int32_t next_code_indent(TSLexer *lexer) {
  for (;;) {
    while (lexer->lookahead == '\r') {
      advance(lexer);
    }
    if (lexer->eof(lexer)) {
      return 0;
    }
    advance(lexer); // a quebra de linha
    int32_t indent = 0;
    while (lexer->lookahead == ' ' || lexer->lookahead == '\t') {
      indent += lexer->lookahead == '\t' ? 4 : 1;
      advance(lexer);
    }
    if (lexer->eof(lexer)) {
      return 0;
    }
    if (lexer->lookahead == '\n' || lexer->lookahead == '\r') {
      continue; // linha em branco
    }
    bool comment = false;
    if (lexer->lookahead == '-') {
      advance(lexer);
      comment = lexer->lookahead == '-';
    } else if (lexer->lookahead == 'n' || lexer->lookahead == 'N') {
      comment = consume_nota(lexer);
    }
    if (!comment) {
      return indent;
    }
    while (!at_line_end(lexer)) {
      advance(lexer);
    }
  }
}

void *tree_sitter_lavra_external_scanner_create(void) {
  Scanner *scanner = ts_calloc(1, sizeof(Scanner));
  array_init(&scanner->indents);
  array_push(&scanner->indents, 0);
  scanner->next_indent = -1;
  return scanner;
}

void tree_sitter_lavra_external_scanner_destroy(void *payload) {
  Scanner *scanner = (Scanner *)payload;
  array_delete(&scanner->indents);
  ts_free(scanner);
}

unsigned tree_sitter_lavra_external_scanner_serialize(void *payload, char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  unsigned size = 0;
  memcpy(buffer, &scanner->next_indent, sizeof(int32_t));
  size += sizeof(int32_t);
  for (uint32_t i = 1; i < scanner->indents.size && size + sizeof(uint16_t) <= TREE_SITTER_SERIALIZATION_BUFFER_SIZE; i++) {
    uint16_t indent = *array_get(&scanner->indents, i);
    memcpy(buffer + size, &indent, sizeof(uint16_t));
    size += sizeof(uint16_t);
  }
  return size;
}

void tree_sitter_lavra_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  array_clear(&scanner->indents);
  array_push(&scanner->indents, 0);
  scanner->next_indent = -1;
  if (length < sizeof(int32_t)) {
    return;
  }
  memcpy(&scanner->next_indent, buffer, sizeof(int32_t));
  for (unsigned i = sizeof(int32_t); i + sizeof(uint16_t) <= length; i += sizeof(uint16_t)) {
    uint16_t indent;
    memcpy(&indent, buffer + i, sizeof(uint16_t));
    array_push(&scanner->indents, indent);
  }
}

bool tree_sitter_lavra_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;
  // Na recuperação de erro tudo vale ao mesmo tempo, até este token que nenhuma regra usa.
  bool recovery = valid_symbols[ERROR_SENTINEL];

  // Dentro de um texto, só o conteúdo: o "--" e o "Nota:" ali são texto.
  if (valid_symbols[STRING_CONTENT] && !recovery) {
    bool any = false;
    while (!at_line_end(lexer) && !is_quote(lexer->lookahead) && lexer->lookahead != '\\') {
      advance(lexer);
      any = true;
    }
    if (any) {
      lexer->result_symbol = STRING_CONTENT;
    }
    return any;
  }

  // Os tokens de largura zero terminam aqui, no fim do token anterior.
  lexer->mark_end(lexer);

  // O resto da linha atual.
  uint32_t skipped = 0;
  while (lexer->lookahead == ' ' || lexer->lookahead == '\t' || lexer->lookahead == '\r') {
    skip(lexer);
    skipped++;
  }
  if (!lexer->eof(lexer) && lexer->lookahead != '\n') {
    // Ainda na mesma linha: um comentário, ou código, que é do léxico interno. A linha só
    // começa aqui no começo do texto.
    if (lexer->lookahead == '-') {
      advance(lexer);
      return lexer->lookahead == '-' && valid_symbols[COMMENT] && finish_comment(lexer);
    }
    if ((lexer->lookahead == 'n' || lexer->lookahead == 'N') && valid_symbols[COMMENT] &&
        lexer->get_column(lexer) == skipped && consume_nota(lexer)) {
      return finish_comment(lexer);
    }
    return false;
  }

  // O fim da linha. Logo depois de ".", ":" ou ";" a gramática espera o NEWLINE: a linha
  // lógica termina aqui, e já se calcula o recuo da próxima linha de código.
  if (valid_symbols[NEWLINE] && !recovery) {
    scanner->next_indent = next_code_indent(lexer);
    lexer->result_symbol = NEWLINE;
    return true;
  }

  // Depois do NEWLINE, ou no meio de uma linha lógica que continua: a próxima linha que
  // não está em branco.
  int32_t indent = 0;
  while (!lexer->eof(lexer)) {
    if (lexer->lookahead == '\n') {
      indent = 0;
    } else if (lexer->lookahead == ' ') {
      indent += 1;
    } else if (lexer->lookahead == '\t') {
      indent += 4;
    } else if (lexer->lookahead != '\r') {
      break;
    }
    skip(lexer);
  }
  uint16_t top = *array_back(&scanner->indents);
  if (lexer->eof(lexer)) {
    if (valid_symbols[DEDENT] && scanner->indents.size > 1) {
      (void)array_pop(&scanner->indents);
      lexer->result_symbol = DEDENT;
      return true;
    }
    return false;
  }

  bool comment_line = false;
  if (lexer->lookahead == '-') {
    advance(lexer);
    comment_line = lexer->lookahead == '-';
  } else if (lexer->lookahead == 'n' || lexer->lookahead == 'N') {
    comment_line = consume_nota(lexer);
  }

  if (comment_line) {
    // O recuo que vem antes do comentário: o bloco que fecha antes dele (o comentário está
    // menos recuado que o bloco, e a próxima linha de código também), ou o bloco novo que o
    // contém (a próxima linha de código está mais recuada).
    if (!recovery && scanner->next_indent >= 0) {
      if (valid_symbols[DEDENT] && scanner->next_indent < top && indent < top) {
        (void)array_pop(&scanner->indents);
        lexer->result_symbol = DEDENT;
        return true;
      }
      if (valid_symbols[INDENT] && scanner->next_indent > top) {
        array_push(&scanner->indents, (uint16_t)scanner->next_indent);
        lexer->result_symbol = INDENT;
        return true;
      }
    }
    return valid_symbols[COMMENT] && finish_comment(lexer);
  }

  // Uma linha de código.
  if (valid_symbols[INDENT] && indent > top) {
    array_push(&scanner->indents, (uint16_t)indent);
    lexer->result_symbol = INDENT;
    return true;
  }
  if (valid_symbols[DEDENT] && indent < top && scanner->indents.size > 1) {
    (void)array_pop(&scanner->indents);
    lexer->result_symbol = DEDENT;
    return true;
  }
  return false;
}
