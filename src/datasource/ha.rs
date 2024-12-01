use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HomeAssistantClient {
    client: Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub state: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub last_updated: String,
    pub last_changed: String,
}

impl HomeAssistantClient {
    pub fn new(host: String, token: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", token))?,
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()?;

        let base_url = format!("{}/api", host.trim_end_matches('/'));

        Ok(Self {
            client,
            base_url,
        })
    }

    /// Get the state of an entity
    pub async fn get_state(&self, entity_id: &str) -> Result<State> {
        let url = format!("{}/states/{}", self.base_url, entity_id);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get state: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        Ok(response.json().await?)
    }

    /// Call a service
    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()> {
        let url = format!("{}/services/{}/{}", self.base_url, domain, service);
        let response = self.client
            .post(&url)
            .json(&data.unwrap_or(serde_json::json!({})))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to call service: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        Ok(())
    }

    /// Get all states
    pub async fn get_states(&self) -> Result<Vec<State>> {
        let url = format!("{}/states", self.base_url);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get states: {} - {}",
                response.status(),
                response.text().await?
            ));
        }

        Ok(response.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_get_state() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/api/states/sensor.temperature")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{
                "state": "23.5",
                "attributes": {"unit_of_measurement": "°C"},
                "last_updated": "2024-01-01T00:00:00Z",
                "last_changed": "2024-01-01T00:00:00Z"
            }"#)
            .create();

        let client = HomeAssistantClient::new(
            server.url(),
            "fake_token".to_string(),
        ).unwrap();

        let state = client.get_state("sensor.temperature").await.unwrap();
        assert_eq!(state.state, Some("23.5".to_string()));
    }
}