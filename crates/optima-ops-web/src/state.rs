//! Application state management

use optima_ops_core::{AppConfig, Environment};
use std::sync::{Arc, RwLock};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: AppConfig,
    current_env: RwLock<Environment>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let env = config.get_environment();
        Self {
            inner: Arc::new(AppStateInner {
                config,
                current_env: RwLock::new(env),
            }),
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    pub fn current_environment(&self) -> Environment {
        *self.inner.current_env.read().unwrap()
    }

    pub fn set_environment(&self, env: Environment) {
        *self.inner.current_env.write().unwrap() = env;
    }

    /// Get all available environments
    pub fn available_environments() -> Vec<(&'static str, &'static str)> {
        vec![
            ("production", "Production"),
            ("stage", "Stage"),
            ("shared", "Shared"),
            ("development", "Development"),
        ]
    }
}
