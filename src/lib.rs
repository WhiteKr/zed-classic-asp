use std::fs;

use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result, Worktree};

const REPO: &str = "WhiteKr/zed-classic-asp";

#[derive(Default)]
struct ClassicAspExtension {
    cached_binary_path: Option<String>,
}

impl ClassicAspExtension {
    /// Downloads the prebuilt asp-ls from GitHub Releases, reusing an already
    /// extracted copy when the latest release is the one we have.
    fn download_binary(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(path) = self.cached_binary_path.as_ref() {
            if fs::metadata(path).is_ok() {
                return Ok(path.clone());
            }
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let (os, arch) = zed::current_platform();
        let target = match (os, arch) {
            (zed::Os::Windows, zed::Architecture::X8664) => "x86_64-pc-windows-msvc",
            (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
            (zed::Os::Mac, zed::Architecture::Aarch64) => "aarch64-apple-darwin",
            _ => return Err("no prebuilt asp-ls for this platform".to_string()),
        };
        let (asset_name, file_type) = match os {
            zed::Os::Windows => (
                format!("asp-ls-{target}.zip"),
                zed::DownloadedFileType::Zip,
            ),
            _ => (
                format!("asp-ls-{target}.tar.gz"),
                zed::DownloadedFileType::GzipTar,
            ),
        };

        let release = zed::latest_github_release(
            REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset named {asset_name} in release {}", release.version))?;

        let version_dir = format!("asp-ls-{}", release.version.trim_start_matches('v'));
        let binary_path = match os {
            zed::Os::Windows => format!("{version_dir}/asp-ls.exe"),
            _ => format!("{version_dir}/asp-ls"),
        };

        if fs::metadata(&binary_path).is_err() {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            zed::download_file(&asset.download_url, &version_dir, file_type)?;
            if !matches!(os, zed::Os::Windows) {
                zed::make_file_executable(&binary_path)?;
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for ClassicAspExtension {
    fn new() -> Self {
        Self::default()
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

        let path = match worktree.which("asp-ls") {
            Some(path) => path,
            None => self.download_binary(language_server_id)?,
        };

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
