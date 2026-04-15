mod client;

pub use client::OmlxClient;
use codex_core::config::Config;

pub async fn ensure_oss_ready(config: &Config) -> std::io::Result<()> {
    let omlx_client = OmlxClient::try_from_provider(config).await?;

    if let Some(model) = config.model.as_ref() {
        match omlx_client.fetch_models().await {
            Ok(models) => {
                if !models.iter().any(|m| m == model) {
                    tracing::warn!(
                        "Model '{model}' not found on oMLX server. Available models: {}",
                        models.join(", ")
                    );
                }
            }
            Err(err) => {
                tracing::warn!("Failed to query models from oMLX: {err}");
            }
        }
    }

    Ok(())
}

pub async fn auto_select_model(config: &Config) -> std::io::Result<Option<String>> {
    let omlx_client = OmlxClient::try_from_provider(config).await?;
    fetch_first_model(&omlx_client).await
}

pub async fn auto_select_model_from_defaults() -> std::io::Result<Option<String>> {
    let omlx_client = OmlxClient::try_from_defaults().await?;
    fetch_first_model(&omlx_client).await
}

async fn fetch_first_model(client: &OmlxClient) -> std::io::Result<Option<String>> {
    match client.fetch_models().await {
        Ok(models) => Ok(models.into_iter().next()),
        Err(err) => {
            tracing::warn!("Failed to query models from oMLX: {err}");
            Ok(None)
        }
    }
}
