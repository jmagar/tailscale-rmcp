use std::{sync::Arc, time::Duration};

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

    async fn get_json(&self, url: &str) -> Result<Value> {
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
            let err = map_api_error(status, &text, &self.tailnet);
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
        self.get_json(&self.tailnet_url("devices")).await
    }

    /// Get a single device by its stable node ID or legacy device ID.
    pub async fn device(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.device", device_id = %device_id);
        let _guard = span.enter();
        self.get_json(&self.device_url(device_id, "")).await
    }

    /// Get the subnet routes configured for a device.
    pub async fn device_routes(&self, device_id: &str) -> Result<Value> {
        let span = tracing::info_span!("upstream.device_routes", device_id = %device_id);
        let _guard = span.enter();
        self.get_json(&self.device_url(device_id, "routes")).await
    }

    /// List API keys in the tailnet.
    pub async fn keys(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.keys");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("keys")).await
    }

    /// Get the ACL policy for the tailnet.
    /// Uses `Accept: application/json` to avoid HuJSON.
    pub async fn acl(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.acl");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("acl")).await
    }

    /// Get DNS nameservers for the tailnet.
    pub async fn dns_nameservers(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_nameservers");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("dns/nameservers")).await
    }

    /// Get DNS search paths for the tailnet.
    pub async fn dns_searchpaths(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_searchpaths");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("dns/searchpaths")).await
    }

    /// Get DNS preferences (MagicDNS, etc.) for the tailnet.
    pub async fn dns_preferences(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.dns_preferences");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("dns/preferences")).await
    }

    /// List users in the tailnet.
    pub async fn users(&self) -> Result<Value> {
        let span = tracing::info_span!("upstream.users");
        let _guard = span.enter();
        self.get_json(&self.tailnet_url("users")).await
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
            return Err(map_api_error(status, &text, &self.tailnet));
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
            return Err(map_api_error(status, &text, &self.tailnet));
        }

        Ok(serde_json::json!({ "ok": true, "device_id": device_id, "action": "deleted" }))
    }
}

// ── HTTP status → informative error ──────────────────────────────────────────

fn map_api_error(status: StatusCode, body: &str, tailnet: &str) -> anyhow::Error {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => anyhow::anyhow!(
            "TAILSCALE_API_KEY invalid or expired (HTTP {status})\n\
             Hint: generate a new key at https://login.tailscale.com/admin/settings/keys\n\
             Body: {body}"
        ),
        StatusCode::NOT_FOUND => anyhow::anyhow!(
            "Resource not found (HTTP 404)\n\
             If this is a tailnet-level request: TAILSCALE_TAILNET={tailnet} not found — \
             use '-' for personal accounts or your org domain (e.g. 'example.com')\n\
             Body: {body}"
        ),
        StatusCode::TOO_MANY_REQUESTS => anyhow::anyhow!(
            "Tailscale API rate limited (HTTP 429) — wait before retrying\n\
             Body: {body}"
        ),
        other => anyhow::anyhow!("Tailscale API error {other}: {body}"),
    }
}
