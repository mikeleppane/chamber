// In a new file: crates/vault/src/autolock.rs or in existing config module
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoLockConfig {
    pub enabled: bool,
    pub inactivity_timeout_minutes: u64,
    pub check_interval_seconds: u64,
}

impl Default for AutoLockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            inactivity_timeout_minutes: 5,
            check_interval_seconds: 15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActivityTracker {
    last_activity: Arc<RwLock<DateTime<Utc>>>,
    config: AutoLockConfig,
}

impl ActivityTracker {
    pub fn new(config: AutoLockConfig) -> Self {
        Self {
            last_activity: Arc::new(RwLock::new(Utc::now())),
            config,
        }
    }

    pub async fn update_activity(&self) {
        let mut last_activity = self.last_activity.write().await;
        *last_activity = Utc::now();
    }

    pub async fn get_last_activity(&self) -> DateTime<Utc> {
        *self.last_activity.read().await
    }

    #[allow(clippy::cast_possible_wrap)]
    pub async fn should_auto_lock(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        let last_activity = self.get_last_activity().await;
        let timeout_duration = Duration::minutes(self.config.inactivity_timeout_minutes as i64);

        Utc::now() - last_activity > timeout_duration
    }

    pub const fn get_config(&self) -> &AutoLockConfig {
        &self.config
    }

    #[allow(clippy::cast_possible_wrap)]
    pub async fn time_until_lock(&self) -> Option<Duration> {
        if !self.config.enabled {
            return None;
        }

        let last_activity = self.get_last_activity().await;
        let timeout_duration = Duration::minutes(self.config.inactivity_timeout_minutes as i64);
        let time_since_activity = Utc::now() - last_activity;

        if time_since_activity >= timeout_duration {
            Some(Duration::zero())
        } else {
            Some(timeout_duration - time_since_activity)
        }
    }
}
