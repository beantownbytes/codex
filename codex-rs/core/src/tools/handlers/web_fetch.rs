use serde::Deserialize;

use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;

pub struct WebFetchHandler;

const DEFAULT_MAX_LENGTH: usize = 100_000;
const REQUEST_TIMEOUT_SECS: u64 = 30;

fn default_max_length() -> usize {
    DEFAULT_MAX_LENGTH
}

#[derive(Deserialize)]
struct WebFetchArgs {
    url: String,
    #[serde(default = "default_max_length")]
    max_length: usize,
}

impl ToolHandler for WebFetchHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let arguments = match &invocation.payload {
            ToolPayload::Function { arguments } => arguments.clone(),
            _ => {
                return Err(FunctionCallError::RespondToModel(
                    "web_fetch handler received unsupported payload".to_string(),
                ));
            }
        };

        let args: WebFetchArgs = parse_arguments(&arguments)?;

        if !args.url.starts_with("http://") && !args.url.starts_with("https://") {
            return Err(FunctionCallError::RespondToModel(
                "url must start with http:// or https://".to_string(),
            ));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let response = client
            .get(&args.url)
            .header("User-Agent", "codex/web_fetch")
            .send()
            .await
            .map_err(|e| {
                FunctionCallError::RespondToModel(format!("request failed: {e}"))
            })?;

        let status = response.status();
        if !status.is_success() {
            return Err(FunctionCallError::RespondToModel(format!(
                "HTTP {status} from {url}",
                url = args.url
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.starts_with("image/")
            || content_type.starts_with("audio/")
            || content_type.starts_with("video/")
            || content_type == "application/octet-stream"
        {
            return Err(FunctionCallError::RespondToModel(format!(
                "binary content type ({content_type}) not supported; web_fetch only returns text"
            )));
        }

        let body = response.text().await.map_err(|e| {
            FunctionCallError::RespondToModel(format!("failed to read response body: {e}"))
        })?;

        let truncated = if body.len() > args.max_length {
            let boundary = body
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= args.max_length)
                .last()
                .unwrap_or(0);
            format!(
                "{}\n\n[truncated — {total} chars total, showing first {shown}]",
                &body[..boundary],
                total = body.len(),
                shown = boundary,
            )
        } else {
            body
        };

        Ok(FunctionToolOutput::from_text(
            truncated,
            Some(true),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_http_urls() {
        let args = r#"{"url": "ftp://example.com"}"#;
        let parsed: WebFetchArgs = serde_json::from_str(args).unwrap();
        assert!(!parsed.url.starts_with("http://") && !parsed.url.starts_with("https://"));
    }

    #[test]
    fn default_max_length_applied() {
        let args: WebFetchArgs = serde_json::from_str(r#"{"url": "https://example.com"}"#).unwrap();
        assert_eq!(args.max_length, DEFAULT_MAX_LENGTH);
    }

    #[test]
    fn custom_max_length() {
        let args: WebFetchArgs =
            serde_json::from_str(r#"{"url": "https://example.com", "max_length": 500}"#).unwrap();
        assert_eq!(args.max_length, 500);
    }
}
