//! Application state management

use optima_ops_core::{AppConfig, Environment, SSHClient};
use std::sync::{Arc, RwLock};
use tokio::sync::Mutex;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    config: AppConfig,
    current_env: RwLock<Environment>,
    /// SSH client for container management (lazy initialized)
    ssh_client: Mutex<Option<SSHClient>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let env = config.get_environment();
        Self {
            inner: Arc::new(AppStateInner {
                config,
                current_env: RwLock::new(env),
                ssh_client: Mutex::new(None),
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
        // Reset SSH client when environment changes
        if let Ok(mut client) = self.inner.ssh_client.try_lock() {
            *client = None;
        }
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

    /// Get or create SSH client for current environment
    pub async fn get_ssh_client(&self) -> Result<tokio::sync::MutexGuard<'_, Option<SSHClient>>, String> {
        let mut guard = self.inner.ssh_client.lock().await;

        if guard.is_none() {
            let env = self.current_environment();
            let client = SSHClient::new(self.config(), Some(env));
            *guard = Some(client);
        }

        Ok(guard)
    }
}
