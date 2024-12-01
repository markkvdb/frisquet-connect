use crate::config;
use crate::datasource::externaltemperature::ExternalTemperatureErr;
use crate::datasource::ha;

impl From<reqwest::Error> for ExternalTemperatureErr {
    fn from(value: reqwest::Error) -> Self {
        ExternalTemperatureErr::from(value.to_string())
    }
}

pub async fn get_ha_temperature(config: &mut config::HAConfig) -> Result<f32, ExternalTemperatureErr> {
    let ha_client = ha::HomeAssistantClient::new(config.url.clone(), config.token.clone())
        .map_err(|e| ExternalTemperatureErr::from(format!("Failed to create HA client: {}", e)))?;

    let response = ha_client.get_state(&config.entity_id).await
        .map_err(|e| ExternalTemperatureErr::from(format!("Failed to get state for {}: {}", config.entity_id, e)))?;

    let temperature_str = if config.temperature_field.is_none() {
        // If no temperature field is specified, use the state directly
        response.state
            .ok_or_else(|| ExternalTemperatureErr::from(format!("No state value for {}", config.entity_id)))?
    } else {
        // If temperature field is specified, look in the attributes
        let attributes = response.attributes
            .ok_or_else(|| ExternalTemperatureErr::from(format!("No attributes for {}", config.entity_id)))?;
        
        let temp_field = config.temperature_field.as_ref().unwrap();
        attributes.get(temp_field)
            .ok_or_else(|| ExternalTemperatureErr::from(format!("No field {} in attributes", temp_field)))?
            .as_str()
            .ok_or_else(|| ExternalTemperatureErr::from(format!("Field {} is not a string", temp_field)))?
            .to_string()
    };

    temperature_str
        .parse::<f32>()
        .map_err(|e| ExternalTemperatureErr::from(format!("Cannot parse temperature '{}': {}", temperature_str, e)))
}


#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;

    #[tokio::test]
    async fn test_get_ha_temperature() {
        // Start a mock server
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        
        // Create test config pointing to mock server
        let mut config = config::HAConfig {
            url: uri.to_string(),
            token: "fake_token".to_string(),
            entity_id: "sensor.temperature".to_string(),
            temperature_field: None,
        };

        // Mock the HA API response
        Mock::given(method("GET"))
            .and(path("/api/states/sensor.temperature"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "state": "21.5",
                    "attributes": {
                        "unit_of_measurement": "°C",
                        "friendly_name": "Temperature"
                    },
                    "last_updated": "2024-01-01T00:00:00Z",
                    "last_changed": "2024-01-01T00:00:00Z"
                })))
            .mount(&mock_server)
            .await;

        let temperature = get_ha_temperature(&mut config).await.unwrap();
        assert_eq!(temperature, 21.5);
    }

    #[tokio::test]
    async fn test_get_ha_temperature_with_temperature_field() {
        let mock_server = MockServer::start().await;
        let uri = mock_server.uri();
        
        let mut config = config::HAConfig {
            url: uri.to_string(),
            token: "fake_token".to_string(),
            entity_id: "sensor.weather".to_string(),
            temperature_field: Some("temperature".to_string()),
        };

        Mock::given(method("GET"))
            .and(path("/api/states/sensor.weather"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(json!({
                    "state": "sunny",
                    "attributes": {
                        "temperature": "23.5",
                        "humidity": "45"
                    },
                    "last_updated": "2024-01-01T00:00:00Z",
                    "last_changed": "2024-01-01T00:00:00Z"
                })))
            .mount(&mock_server)
            .await;

        let temperature = get_ha_temperature(&mut config).await.unwrap();
        assert_eq!(temperature, 23.5);
    }
}
