//! A extensão da Lavra no Zed. O realce, o recuo e a estrutura vêm da gramática e das
//! queries (`languages/lavra`); o servidor de linguagem é o próprio `lavrac`, o das
//! configurações do Zed ou o do PATH do projeto, com as opções que o usuário der a ele.
//!
//! Nas configurações do Zed, a chave é o nome do servidor:
//!
//! ```json
//! "lsp": {
//!   "lavrac": {
//!     "binary": { "path": "/caminho/do/lavrac" },
//!     "initialization_options": { "inlayHints": { "types": false } }
//!   }
//! }
//! ```

use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result, serde_json};

struct Lavra;

/// As configurações do usuário para o servidor ("lsp": { "lavrac": ... }), se houver.
fn user_settings(id: &LanguageServerId, worktree: &zed::Worktree) -> LspSettings {
    LspSettings::for_worktree(id.as_ref(), worktree).unwrap_or_default()
}

impl zed::Extension for Lavra {
    fn new() -> Self {
        Lavra
    }

    fn language_server_command(&mut self, id: &LanguageServerId, worktree: &zed::Worktree) -> Result<zed::Command> {
        let binary = user_settings(id, worktree).binary;
        let configured = binary.as_ref().and_then(|b| b.path.clone());
        let command = match configured {
            Some(path) => path,
            None => worktree.which("lavrac").ok_or_else(|| {
                "o Zed não achou o lavrac no PATH: instale-o pela página da última versão \
                 (https://github.com/JoaoHenriqueBarbosa/lavra/releases/latest) ou indique o caminho \
                 em \"lsp\": { \"lavrac\": { \"binary\": { \"path\": ... } } }"
                    .to_string()
            })?,
        };
        let args = binary.as_ref().and_then(|b| b.arguments.clone()).unwrap_or_else(|| vec!["lsp".into()]);
        let mut env = worktree.shell_env();
        env.extend(binary.and_then(|b| b.env).unwrap_or_default());
        Ok(zed::Command { command, args, env })
    }

    /// As opções de inicialização do usuário, como os inlay hints que ele quer. O texto das
    /// bibliotecas (`libraryDocuments`) fica desligado: o Zed só abre arquivos, e a definição
    /// de um verbo de biblioteca não tem para onde ir.
    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(user_settings(id, worktree).initialization_options)
    }

    /// A configuração do usuário ("settings"), que o Zed manda em
    /// `workspace/didChangeConfiguration` quando ela muda.
    fn language_server_workspace_configuration(
        &mut self,
        id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<serde_json::Value>> {
        Ok(user_settings(id, worktree).settings)
    }
}

zed::register_extension!(Lavra);
