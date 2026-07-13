use std::{error::Error as StdError, fmt, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use reqwest::{Client, RequestBuilder, StatusCode};
use serde_json::Value;

use crate::{config::TailscaleConfig, observability::Counters};

const TIMEOUT: Duration = Duration::from_secs(30);

/// Low-level HTTP client for the Tailscale REST API.
///
/// All methods return raw `serde_json::Value` so the service layer can pass
/// them straight through to callers without any schema coupling.
#[derive(Clone)]
pub struct TailscaleClient {
    client: Client,
    api_key: String,
    pub tailnet: String,
    base_url: String,
    pub counters: Arc<Counters>,
}

impl TailscaleClient {
    pub fn new(config: &TailscaleConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(TIMEOUT)
            .build()
            .context("failed to build reqwest client")?;

        Ok(Self {
            client,
            api_key: config.api_key.clone(),
            tailnet: config.tailnet.clone(),
            base_url: "https://api.tailscale.com/api/v2".into(),
            counters: Arc::new(Counters::new()),
        })
    }

    // ── path helpers ──────────────────────────────────────────────────────────

    fn tailnet_url(&self, path: &str) -> String {
        format!("{}/tailnet/{}/{}", self.base_url, self.tailnet, path)
    }

    fn device_url(&self, device_id: &str, path: &str) -> String {
        if path.is_empty() {
            format!("{}/device/{}", self.base_url, device_id)
        } else {
            format!("{}/device/{}/{}", self.base_url, device_id, path)
        }
    }

    fn auth(&self, req: RequestBuilder) -> RequestBuilder {
        req.bearer_auth(&self.api_key)
    }

    async fn get_json(&self, url: &str, not_found_message: &'static str) -> Result<Value> {
        self.counters.inc_upstream();
        let span = tracing::info_span!("upstream.get", url = %url);
        let _guard = span.enter();

        let result = self
            .auth(self.client.get(url))
            .header("Accept", "application/json")
            .send()
            .await
            .with_context(|| {
                format!(
                    "upstream request failed — is TAILSCALE_API_URL correct? (GET {url})\n\
                     Hint: run `tailscale status` to check server health"
                )
            });

        let resp = match result {
            Ok(r) => r,
            Err(e) => {
                self.counters.inc_upstream_errors();
                tracing::warn!(url = %url, error = %e, "upstream GET failed");
                return Err(e);
            }
        };

        let status = resp.status();
        let text = resp
            .text()
            .await
            .with_context(|| format!("reading body from {url}"))?;

        if !status.is_success() {
            self.counters.inc_upstream_errors();
            let err = map_api_error(status, &text, &self.tailnet, not_found_message);
            tracing::warn!(url = %url, status = %status, "upstream API error");
            return Err(err);
        }

        tracing::debug!(url = %url, "upstream GET ok");
        serde_json::from_str(&text).with_context(|| format!("parsing JSON from {url}: {text}"))
    }

    // ── read endpoints ────────────────────────────────────────────────────────

    /// List all devices in the tailnet.
    pub async fn devices(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.devices");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("devices"), "Tailscale resource not found")
            .await
    }

    /// Get a single device by its stable node ID or legacy device ID.
    pub async fn device(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.device", device_id = %device_id);
        let _guard = span.enter();
        self.get_json(&self.device_url(device_id, ""), "Device not found")
            .await
    }

    /// Get the subnet routes configured for a device.
    pub async fn device_routes(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.device_routes", device_id = %device_id);
        let _guard = span.enter();
        self.get_json(&self.device_url(device_id, "routes"), "Device not found")
            .await
    }

    /// List API keys in the tailnet.
    pub async fn keys(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.keys");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("keys"), "Tailscale resource not found")
            .await
    }

    /// Get the ACL policy for the tailnet.
    /// Uses `Accept: application/json` to avoid HuJSON.
    pub async fn acl(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.acl");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("acl"), "Tailscale resource not found")
            .await
    }

    /// Get DNS nameservers for the tailnet.
    pub async fn dns_nameservers(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_nameservers");
        let _guard = span.enter();
        self.get_json(
            &self.tailnet_url("dns/nameservers"),
            "Tailscale resource not found",
        )
        .await
    }

    /// Get DNS search paths for the tailnet.
    pub async fn dns_searchpaths(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_searchpaths");
        let _guard = span.enter();
        self.get_json(
            &self.tailnet_url("dns/searchpaths"),
            "Tailscale resource not found",
        )
        .await
    }

    /// Get DNS preferences (MagicDNS, etc.) for the tailnet.
    pub async fn dns_preferences(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_preferences");
        let _guard = span.enter();
        self.get_json(
            &self.tailnet_url("dns/preferences"),
            "Tailscale resource not found",
        )
        .await
    }

    /// List users in the tailnet.
    pub async fn users(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.users");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("users"), "Tailscale resource not found")
            .await
    }

    /// Probe reachability — used by /health endpoint.
    /// Returns `Ok(latency_ms)` if the API is reachable, `Err` otherwise.
    pub async fn probe(&self) -> Result<u64> {
        let url = self.tailnet_url("devices?limit=1");
        let started = std::time::Instant::now();
        self.counters.inc_upstream();
        let resp = self
            .auth(self.client.get(&url))
            .header("Accept", "application/json")
            .send()
            .await
            .context("Tailscale API probe failed")?;
        let latency = started.elapsed().as_millis() as u64;
        if resp.status().is_success() || resp.status() == StatusCode::UNAUTHORIZED {
            // 401 means the API is reachable but the key may be wrong — still "up"
            Ok(latency)
        } else {
            self.counters.inc_upstream_errors();
            Err(anyhow::anyhow!(
                "Tailscale API probe returned {}",
                resp.status()
            ))
        }
    }

    // ── write endpoints ───────────────────────────────────────────────────────

    /// Authorize a device (approve it for the tailnet).
    pub async fn authorize_device(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.authorize_device", device_id = %device_id);
        let _guard = span.enter();
        self.counters.inc_upstream();

        let url = self.device_url(device_id, "authorized");
        let resp = self
            .auth(self.client.post(&url))
            .header("Content-Type", "application/json")
            .body(r#"{"authorized":true}"#)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .with_context(|| format!("reading body from {url}"))?;

        if !status.is_success() {
            self.counters.inc_upstream_errors();
            return Err(map_api_error(
                status,
                &text,
                &self.tailnet,
                "Device not found",
            ));
        }

        // 200 with empty body or JSON — handle both
        if text.trim().is_empty() {
            Ok(serde_json::json!({ "ok": true }))
        } else {
            serde_json::from_str(&text).with_context(|| format!("parsing JSON from {url}: {text}"))
        }
    }

    /// Remove a device from the tailnet (destructive).
    pub async fn delete_device(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.delete_device", device_id = %device_id);
        let _guard = span.enter();
        self.counters.inc_upstream();

        let url = self.device_url(device_id, "");
        let resp = self
            .auth(self.client.delete(&url))
            .send()
            .await
            .with_context(|| format!("DELETE {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            self.counters.inc_upstream_errors();
            let text = resp
                .text()
                .await
                .with_context(|| format!("reading body from {url}"))?;
            return Err(map_api_error(
                status,
                &text,
                &self.tailnet,
                "Device not found",
            ));
        }

        Ok(serde_json::json!({ "ok": true, "device_id": device_id, "action": "deleted" }))
    }
}

// ── HTTP status → informative error ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TailscaleApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub hint: Option<String>,
    pub body: String,
}

impl TailscaleApiError {
    fn from_response(
        status: StatusCode,
        body: &str,
        tailnet: &str,
        not_found_message: &'static str,
    ) -> Self {
        let body = body.trim().to_string();
        let (code, message, hint) = match status {
            StatusCode::UNAUTHORIZED => (
                "unauthorized",
                "Tailscale API key is invalid or expired".to_string(),
                Some(
                    "Generate a new key at https://login.tailscale.com/admin/settings/keys"
                        .to_string(),
                ),
            ),
            StatusCode::FORBIDDEN => (
                "forbidden",
                "Tailscale API key is not allowed to perform this request".to_string(),
                Some(
                    "Check the key scopes at https://login.tailscale.com/admin/settings/keys"
                        .to_string(),
                ),
            ),
            StatusCode::NOT_FOUND => (
                "not_found",
                not_found_message.to_string(),
                Some(format!(
                    "If this was a tailnet-level request, check TAILSCALE_TAILNET={tailnet}; \
                     use '-' for personal accounts or your org domain (for example, 'example.com')"
                )),
            ),
            StatusCode::TOO_MANY_REQUESTS => (
                "rate_limited",
                "Tailscale API rate limited the request".to_string(),
                Some("Wait before retrying".to_string()),
            ),
            other => (
                "tailscale_api_error",
                format!("Tailscale API error {other}"),
                None,
            ),
        };

        Self {
            status,
            code,
            message,
            hint,
            body,
        }
    }
}

impl fmt::Display for TailscaleApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (HTTP {})", self.message, self.status.as_u16())?;
        if let Some(hint) = &self.hint {
            write!(f, "\nHint: {hint}")?;
        }
        if !self.body.is_empty() {
            write!(f, "\nBody: {}", self.body)?;
        }
        Ok(())
    }
}

impl StdError for TailscaleApiError {}

fn map_api_error(
    status: StatusCode,
    body: &str,
    tailnet: &str,
    not_found_message: &'static str,
) -> anyhow::Error {
    TailscaleApiError::from_response(status, body, tailnet, not_found_message).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_api_error_is_clear_for_missing_device() {
        let error = map_api_error(
            StatusCode::NOT_FOUND,
            r#"{"message":"missing"}"#,
            "-",
            "Device not found",
        );
        let message = error.to_string();

        assert!(
            message.contains("Device not found"),
            "404 should produce a user-facing device-not-found message, got: {message}"
        );
        assert!(
            message.contains("HTTP 404"),
            "404 should preserve the upstream HTTP status, got: {message}"
        );
    }
}
