//! A extensão da Lavra no Zed. O realce, o recuo e a estrutura vêm da gramática e das
//! queries (`languages/lavra`); o servidor de linguagem é o próprio `lavrac`, o das
//! configurações do Zed ou o do PATH do projeto, com as opções que o usuário der a ele.
//!
//! O depurador é o mesmo `lavrac`, como adaptador de depuração (`lavrac dap`): a extensão
//! diz ao Zed como abri-lo e traduz o pedido de "depurar este arquivo" para o que ele entende.
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
use zed_extension_api::{
    self as zed, DebugAdapterBinary, DebugConfig, DebugRequest, DebugScenario, DebugTaskDefinition, LanguageServerId, Result,
    StartDebuggingRequestArguments, StartDebuggingRequestArgumentsRequest, serde_json,
};

/// O nome do adaptador de depuração, como está no `extension.toml`.
const ADAPTER: &str = "lavrac";

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

    /// Como abrir o adaptador de depuração: o `lavrac` que o usuário indicou, ou o do PATH
    /// do projeto, com o subcomando "dap". A conversa é pela entrada e saída padrão.
    fn get_dap_binary(
        &mut self,
        adapter_name: String,
        config: DebugTaskDefinition,
        user_provided_debug_adapter_path: Option<String>,
        worktree: &zed::Worktree,
    ) -> Result<DebugAdapterBinary, String> {
        if adapter_name != ADAPTER {
            return Err(format!("a extensão da Lavra não conhece o adaptador \"{adapter_name}\""));
        }
        let command = match user_provided_debug_adapter_path {
            Some(path) => path,
            None => worktree
                .which("lavrac")
                .ok_or_else(|| "o Zed não achou o lavrac no PATH: instale-o, ou indique o caminho do adaptador de depuração".to_string())?,
        };
        let request = self.dap_request_kind(adapter_name, serde_json::from_str(&config.config).map_err(|e| e.to_string())?)?;
        Ok(DebugAdapterBinary {
            command: Some(command),
            arguments: vec!["dap".into()],
            envs: worktree.shell_env(),
            cwd: Some(worktree.root_path()),
            connection: None,
            request_args: StartDebuggingRequestArguments { configuration: config.config, request },
        })
    }

    /// O `lavrac dap` só inicia programas: não se liga a um que já está rodando.
    fn dap_request_kind(&mut self, _adapter_name: String, config: serde_json::Value) -> Result<StartDebuggingRequestArgumentsRequest, String> {
        match config.get("request").and_then(|r| r.as_str()) {
            Some("launch") | None => Ok(StartDebuggingRequestArgumentsRequest::Launch),
            Some(other) => Err(format!("o depurador da Lavra inicia o programa (\"launch\"), e o pedido é \"{other}\"")),
        }
    }

    /// "Depurar este arquivo", do painel do Zed, vira a configuração do `lavrac dap`: o
    /// programa, e se para na primeira oração.
    fn dap_config_to_scenario(&mut self, config: DebugConfig) -> Result<DebugScenario, String> {
        let DebugRequest::Launch(launch) = config.request else {
            return Err("o depurador da Lavra inicia o programa, e não se liga a um que já roda".to_string());
        };
        let configuration = serde_json::json!({
            "request": "launch",
            "program": launch.program,
            "stopOnEntry": config.stop_on_entry.unwrap_or(false),
        });
        Ok(DebugScenario { label: config.label, adapter: config.adapter, build: None, config: configuration.to_string(), tcp_connection: None })
    }
}

zed::register_extension!(Lavra);
