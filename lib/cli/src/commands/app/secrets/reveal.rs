use super::utils;
use crate::{
    commands::{
        app::util::{get_app_id_from_config, AppIdent, prompt_app_ident},
        AsyncCliCommand,
    },
    opts::{ApiOpts, ListFormatOpts, WasmerEnv},
    utils::render::{ItemFormat, ListFormat},
};
use dialoguer::theme::ColorfulTheme;
use is_terminal::IsTerminal;
use wasmer_api::WasmerClient;
use std::{env::current_dir, path::PathBuf};

/// Reveal the value of an existing secret related to an Edge app.
#[derive(clap::Parser, Debug)]
pub struct CmdAppSecretsReveal {
    /// The name of the secret to get the value of.
    #[clap(name = "name")]
    pub secret_name: Option<String>,

    /// The id of the app the secret is related to.
    pub app_id: Option<AppIdent>,

    /// The path to the directory where the config file for the application will be written to.
    #[clap(long = "app-dir", conflicts_with = "app_id")]
    pub app_dir_path: Option<PathBuf>,

    /// Reveal all the secrets related to an app.
    #[clap(long, conflicts_with = "name")]
    pub all: bool,

    /* --- Common args --- */
    #[clap(flatten)]
    #[allow(missing_docs)]
    pub api: ApiOpts,

    #[clap(flatten)]
    pub env: WasmerEnv,

    /// Don't print any message.
    #[clap(long)]
    pub quiet: bool,

    /// Do not prompt for user input.
    #[clap(long, default_value_t = !std::io::stdin().is_terminal())]
    pub non_interactive: bool,

    #[clap(flatten)]
    pub fmt: Option<ListFormatOpts>,
}

impl CmdAppSecretsReveal {
    async fn get_app_id(&self, client: &WasmerClient) -> anyhow::Result<String> {
        if let Some(app_id) = &self.app_id {
            let app = app_id.resolve(client).await?;
            return Ok(app.id.into_inner());
        }

        let app_dir_path = if let Some(app_dir_path) = &self.app_dir_path {
            app_dir_path.clone()
        } else {
            current_dir()?
        };

        if let Ok(Some(app_id)) = get_app_id_from_config(&app_dir_path).await {
            return Ok(app_id.clone());
        }

        if self.non_interactive {
            anyhow::bail!("No app id given. Use the `--app_id` flag to specify one.")
        } else {
            let id = prompt_app_ident("Enter the name of the app")?;
            let app = id.resolve(client).await?;
            return Ok(app.id.into_inner());
        }
    }

    fn get_secret_name(&self) -> anyhow::Result<String> {
        if let Some(name) = &self.secret_name {
            return Ok(name.clone());
        }

        if self.non_interactive {
            anyhow::bail!("No secret name given. Use the `--name` flag to specify one.")
        } else {
            let theme = ColorfulTheme::default();
            Ok(dialoguer::Input::with_theme(&theme)
                .with_prompt("Enter the name of the secret:")
                .interact_text()?)
        }
    }
}

#[async_trait::async_trait]
impl AsyncCliCommand for CmdAppSecretsReveal {
    type Output = ();

    async fn run_async(self) -> Result<Self::Output, anyhow::Error> {
        let client = self.api.client()?;
        let app_id = self.get_app_id(&client).await?;

        if !self.all {
            let name = self.get_secret_name()?;

            let value = utils::get_secret_value_by_name(&client, &app_id, &name).await?;

            let secret = utils::Secret { name, value };

            if let Some(fmt) = &self.fmt {
                let fmt = match fmt.format {
                    ListFormat::Json => ItemFormat::Json,
                    ListFormat::Yaml => ItemFormat::Yaml,
                    ListFormat::Table => ItemFormat::Table,
                    ListFormat::ItemTable => {
                        anyhow::bail!("The 'item-table' format is not available for single values.")
                    }
                };
                println!("{}", fmt.render(&secret));
            } else {
                print!("{}", secret.value);
            }
        } else {
            let secrets: Vec<utils::Secret> = utils::reveal_secrets(&client, &app_id).await?;

            if let Some(fmt) = &self.fmt {
                println!("{}", fmt.format.render(secrets.as_slice()));
            } else {
                for secret in secrets {
                    println!(
                        "{}=\"{}\"",
                        secret.name,
                        utils::render::sanitize_value(&secret.value)
                    );
                }
            }
        }

        Ok(())
    }
}
