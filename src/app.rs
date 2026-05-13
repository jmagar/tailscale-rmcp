use anyhow::Result;
use serde_json::Value;

use crate::tailscale::TailscaleClient;

/// Business service layer. All tool and CLI logic calls into this.
/// The client is a thin HTTP wrapper; this is where policy lives.
#[derive(Clone)]
pub struct TailscaleService {
    client: TailscaleClient,
    pub allow_destructive: bool,
}

impl TailscaleService {
    pub fn new(client: TailscaleClient, allow_destructive: bool) -> Self {
        Self {
            client,
            allow_destructive,
        }
    }

    // ── read operations ───────────────────────────────────────────────────────

    pub async fn devices(&self) -> Result<Value> {
        self.client.devices().await
    }

    pub async fn device(&self, id: &str) -> Result<Value> {
        self.client.device(id).await
    }

    pub async fn device_routes(&self, id: &str) -> Result<Value> {
        self.client.device_routes(id).await
    }

    pub async fn keys(&self) -> Result<Value> {
        self.client.keys().await
    }

    pub async fn acl(&self) -> Result<Value> {
        self.client.acl().await
    }

    /// Aggregate all DNS info (nameservers, search paths, preferences) into one object.
    pub async fn dns(&self) -> Result<Value> {
        let (nameservers, searchpaths, preferences) = tokio::try_join!(
            self.client.dns_nameservers(),
            self.client.dns_searchpaths(),
            self.client.dns_preferences(),
        )?;
        Ok(serde_json::json!({
            "nameservers": nameservers,
            "searchpaths": searchpaths,
            "preferences": preferences,
        }))
    }

    pub async fn users(&self) -> Result<Value> {
        self.client.users().await
    }

    // ── write operations (non-destructive) ────────────────────────────────────

    pub async fn authorize_device(&self, id: &str) -> Result<Value> {
        self.client.authorize_device(id).await
    }

    // ── destructive operations ────────────────────────────────────────────────

    /// Delete a device. Requires `allow_destructive` to be set on the service
    /// AND `confirm = true` in the caller's arguments.
    pub async fn delete_device(&self, id: &str, confirm: bool) -> Result<Value> {
        if !self.allow_destructive {
            anyhow::bail!(
                "destructive operations are disabled on this server; \
                 set TAILSCALE_ALLOW_DESTRUCTIVE=true to enable"
            );
        }
        if !confirm {
            anyhow::bail!(
                "confirm=true is required to delete a device; \
                 this is a permanent, irreversible action"
            );
        }
        self.client.delete_device(id).await
    }
}
