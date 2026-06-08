//! Supabase client for fleet registry integration.
//!
//! Reads credentials from environment:
//!   - SUPABASE_URL     (e.g. https://project.supabase.co)
//!   - SUPABASE_SERVICE_KEY  (service_role key)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

/// A single row from the `repos` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRow {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A single row from the `fleet_budgets` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetBudget {
    pub agent_id: String,
    pub total_budget: f64,
    pub gamma: f64,
    pub eta: f64,
}

/// A single row from the `fleet_events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetEvent {
    pub agent_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

/// Supabase REST client.
pub struct SupabaseClient {
    url: String,
    key: String,
    client: reqwest::blocking::Client,
}

impl SupabaseClient {
    /// Create a new client from environment variables.
    /// Returns `Ok(None)` if env vars are not set.
    pub fn from_env() -> Result<Option<Self>> {
        let url = match env::var("SUPABASE_URL") {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let key = match env::var("SUPABASE_SERVICE_KEY") {
            Ok(v) => v,
            Err(_) => return Ok(None),
        };
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(Some(SupabaseClient { url, key, client }))
    }

    /// Create a client directly (useful for tests).
    pub fn new(url: impl Into<String>, key: impl Into<String>) -> Result<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to build HTTP client")?;
        Ok(SupabaseClient {
            url: url.into(),
            key: key.into(),
            client,
        })
    }

    fn api_url(&self, table: &str) -> String {
        format!("{}/rest/v1/{}", self.url.trim_end_matches('/'), table)
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "apikey",
            reqwest::header::HeaderValue::from_str(&self.key).unwrap(),
        );
        headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.key)).unwrap(),
        );
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Prefer",
            reqwest::header::HeaderValue::from_static("resolution=merge-duplicates"),
        );
        headers
    }

    /// Upsert a repo row into the `repos` table.
    pub fn upsert_repo(
        &self,
        name: &str,
        description: Option<&str>,
        language: Option<&str>,
        url: Option<&str>,
    ) -> Result<()> {
        let row = RepoRow {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            language: language.map(|s| s.to_string()),
            url: url.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(self.api_url("repos"))
            .headers(self.headers())
            .json(&row)
            .send()
            .context("Failed to POST to repos table")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Supabase repos upsert failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Query all rows from the `fleet_budgets` table.
    pub fn get_fleet_budgets(&self) -> Result<Vec<FleetBudget>> {
        let response = self
            .client
            .get(self.api_url("fleet_budgets"))
            .headers(self.headers())
            .send()
            .context("Failed to GET fleet_budgets")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Supabase fleet_budgets query failed ({}): {}", status, body);
        }

        let budgets: Vec<FleetBudget> = response
            .json()
            .context("Failed to parse fleet_budgets JSON")?;
        Ok(budgets)
    }

    /// Insert a row into the `fleet_events` table.
    pub fn insert_fleet_event(
        &self,
        agent_id: &str,
        event_type: &str,
        payload: Option<serde_json::Value>,
    ) -> Result<()> {
        let event = FleetEvent {
            agent_id: agent_id.to_string(),
            event_type: event_type.to_string(),
            payload,
        };

        let response = self
            .client
            .post(self.api_url("fleet_events"))
            .headers(self.headers())
            .json(&event)
            .send()
            .context("Failed to POST to fleet_events")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            anyhow::bail!("Supabase fleet_events insert failed ({}): {}", status, body);
        }

        Ok(())
    }

    /// Check whether conservation holds for every budget row.
    pub fn check_conservation(&self) -> Result<Vec<(String, bool)>> {
        let budgets = self.get_fleet_budgets()?;
        let mut results = Vec::new();
        for b in &budgets {
            let valid = (b.gamma + b.eta - b.total_budget).abs() < 1e-9;
            results.push((b.agent_id.clone(), valid));
        }
        Ok(results)
    }
}
