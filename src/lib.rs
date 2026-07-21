use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

struct ClassicAspExtension;

impl zed::Extension for ClassicAspExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let binary_settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.binary);

        if let Some(path) = binary_settings.as_ref().and_then(|b| b.path.clone()) {
            return Ok(zed::Command {
                command: path,
                args: binary_settings
                    .and_then(|b| b.arguments)
                    .unwrap_or_default(),
                env: Default::default(),
            });
        }

        let path = worktree.which("asp-ls").ok_or_else(|| {
            "asp-ls not found in PATH. Build it with `cargo install --path server` \
             or set `lsp.asp-ls.binary.path` in your Zed settings."
                .to_string()
        })?;

        Ok(zed::Command {
            command: path,
            args: vec![],
            env: Default::default(),
        })
    }

    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.initialization_options))
    }

    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        Ok(LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .ok()
            .and_then(|settings| settings.settings))
    }
}

zed::register_extension!(ClassicAspExtension);
