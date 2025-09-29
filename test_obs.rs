// Test if observability module compiles
mod observability {
    use serde::{Deserialize, Serialize};
    use thiserror::Error;
    use tracing::{info, Level};
    use tracing_subscriber::{EnvFilter, fmt};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HealthStatus {
        pub status: String,
        pub checks: std::collections::HashMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ObservabilityConfig {
        pub log_level: String,
        pub json_logs: bool,
        pub enable_tracing: bool,
        pub enable_metrics: bool,
        pub service_name: String,
        pub service_version: String,
    }

    #[derive(Debug)]
    pub struct ObservabilityManager {
        pub config: ObservabilityConfig,
    }

    impl ObservabilityManager {
        pub fn new(config: ObservabilityConfig) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self { config })
        }
    }
}

fn main() {
    use observability::*;
    let config = ObservabilityConfig {
        log_level: "info".to_string(),
        json_logs: false,
        enable_tracing: true,
        enable_metrics: true,
        service_name: "test".to_string(),
        service_version: "1.0.0".to_string(),
    };
    let _manager = ObservabilityManager::new(config).unwrap();
    println!("Observability module test successful");
}
