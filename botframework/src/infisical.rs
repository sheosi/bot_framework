//! Infisical secrets management client
//! Uses the official Infisical Rust SDK

use infisical::resources::secrets::ListSecretsRequest;
use infisical::{AuthMethod, Client};
use std::collections::HashMap;
use tracing::info;

/// Infisical configuration
#[derive(Debug, Clone)]
pub struct InfisicalConfig {
    pub api_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub project_id: String,
    pub environment: String,
}

impl InfisicalConfig {
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("INFISICAL_CLIENT_ID").expect("INFISICAL_CLIENT_ID needed");
        let client_secret =
            std::env::var("INFISICAL_CLIENT_SECRET").expect("INFISICAL_CLIENT_SECRET needed");
        let project_id =
            std::env::var("INFISICAL_PROJECT_ID").expect("INFISICAL_PROJECT_ID needed");
        let api_url = std::env::var("INFISICAL_API_URL").expect("INFISICAL_API_URL needed");
        let environment =
            std::env::var("INFISICAL_ENVIRONMENT").unwrap_or_else(|_| "dev".to_string());

        Some(Self {
            api_url,
            client_id,
            client_secret,
            project_id,
            environment,
        })
    }
}

/// Load Infisical secrets into a HashMap if configured
pub async fn load_infisical_secrets(config: Option<InfisicalConfig>) -> HashMap<String, String> {
    let Some(config) = config else {
        panic!("Infisical config is needed")
    };
    let mut client = match Client::builder().base_url(config.api_url).build().await {
        Ok(client) => client,
        Err(e) => {
            panic!("Failed to create Infisical client: {}", e);
        }
    };

    // Authenticate using Universal Auth
    let auth_method = AuthMethod::new_universal_auth(config.client_id, config.client_secret);
    if let Err(e) = client.login(auth_method).await {
        panic!("Failed to authenticate with Infisical: {}", e);
    }

    // Fetch secrets
    let request = ListSecretsRequest::builder(&config.project_id, &config.environment).build();

    match client.secrets().list(request).await {
        Ok(secrets) => {
            let secrets_map: HashMap<String, String> = secrets
                .into_iter()
                .map(|s| (s.secret_key, s.secret_value))
                .collect();

            info!(
                "Loaded {} secrets from Infisical (project: {}, env: {})",
                secrets_map.len(),
                config.project_id,
                config.environment
            );

            secrets_map
        }
        Err(e) => {
            panic!("Failed to load secrets from Infisical: {}", e);
        }
    }
}
