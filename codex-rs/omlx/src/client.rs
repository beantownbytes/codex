use codex_core::config::Config;
use codex_model_provider_info::DEFAULT_OMLX_PORT;
use codex_model_provider_info::OMLX_OSS_PROVIDER_ID;
use codex_model_provider_info::WireApi;
use codex_model_provider_info::create_oss_provider;
use std::io;

#[derive(Clone)]
pub struct OmlxClient {
    client: reqwest::Client,
    base_url: String,
}

const OMLX_CONNECTION_ERROR: &str =
    "oMLX server is not responding. Start it with: `omlx serve`";

impl OmlxClient {
    pub async fn try_from_provider(config: &Config) -> io::Result<Self> {
        let provider = config
            .model_providers
            .get(OMLX_OSS_PROVIDER_ID)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Built-in provider {OMLX_OSS_PROVIDER_ID} not found"),
                )
            })?;
        let base_url = provider.base_url.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "oss provider must have a base_url",
            )
        })?;
        Self::try_from_base_url(base_url).await
    }

    pub async fn try_from_defaults() -> io::Result<Self> {
        let provider = create_oss_provider(DEFAULT_OMLX_PORT, WireApi::Responses);
        let base_url = provider.base_url.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "oss provider must have a base_url",
            )
        })?;
        Self::try_from_base_url(base_url).await
    }

    async fn try_from_base_url(base_url: &str) -> io::Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let omlx_client = OmlxClient {
            client,
            base_url: base_url.to_string(),
        };
        omlx_client.check_server().await?;

        Ok(omlx_client)
    }

    async fn check_server(&self) -> io::Result<()> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let response = self.client.get(&url).send().await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "Server returned error: {} {OMLX_CONNECTION_ERROR}",
                    resp.status()
                )))
            }
        } else {
            Err(io::Error::other(OMLX_CONNECTION_ERROR))
        }
    }

    pub async fn fetch_models(&self) -> io::Result<Vec<String>> {
        let url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| io::Error::other(format!("Request failed: {e}")))?;

        if response.status().is_success() {
            let json: serde_json::Value = response.json().await.map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("JSON parse error: {e}"))
            })?;
            let models = json["data"]
                .as_array()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "No 'data' array in response")
                })?
                .iter()
                .filter_map(|model| model["id"].as_str())
                .map(std::string::ToString::to_string)
                .collect();
            Ok(models)
        } else {
            Err(io::Error::other(format!(
                "Failed to fetch models: {}",
                response.status()
            )))
        }
    }

    #[cfg(test)]
    fn from_host_root(host_root: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: host_root.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[tokio::test]
    async fn test_fetch_models_happy_path() {
        if std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/models"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_raw(
                    serde_json::json!({
                        "object": "list",
                        "data": [
                            {"id": "qwen3-30b", "object": "model", "owned_by": "omlx"},
                            {"id": "llama-3b", "object": "model", "owned_by": "omlx"},
                        ]
                    })
                    .to_string(),
                    "application/json",
                ),
            )
            .mount(&server)
            .await;

        let client = OmlxClient::from_host_root(server.uri());
        let models = client.fetch_models().await.expect("fetch models");
        assert!(models.contains(&"qwen3-30b".to_string()));
        assert!(models.contains(&"llama-3b".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_models_no_data_array() {
        if std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/models"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_raw(serde_json::json!({}).to_string(), "application/json"),
            )
            .mount(&server)
            .await;

        let client = OmlxClient::from_host_root(server.uri());
        let result = client.fetch_models().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No 'data' array in response")
        );
    }

    #[tokio::test]
    async fn test_fetch_models_server_error() {
        if std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/models"))
            .respond_with(wiremock::ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = OmlxClient::from_host_root(server.uri());
        let result = client.fetch_models().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to fetch models: 500")
        );
    }

    #[tokio::test]
    async fn test_check_server_happy_path() {
        if std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/models"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = OmlxClient::from_host_root(server.uri());
        client
            .check_server()
            .await
            .expect("server check should pass");
    }

    #[tokio::test]
    async fn test_check_server_error() {
        if std::env::var(codex_core::spawn::CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR).is_ok() {
            return;
        }

        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/models"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = OmlxClient::from_host_root(server.uri());
        let result = client.check_server().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Server returned error: 404")
        );
    }

    #[test]
    fn test_from_host_root() {
        let client = OmlxClient::from_host_root("http://localhost:8000");
        assert_eq!(client.base_url, "http://localhost:8000");
    }
}
