// In a new file: crates/vault/src/autolock_service.rs
use crate::autolock::{ActivityTracker, AutoLockConfig};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration as TokioDuration, sleep};
use tracing::{debug, info, warn};

#[async_trait]
pub trait AutoLockCallback: Send + Sync {
    async fn on_auto_lock(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct AutoLockService {
    pub activity_tracker: ActivityTracker,
    callback: Arc<dyn AutoLockCallback>,
    is_running: Arc<RwLock<bool>>,
}

impl AutoLockService {
    pub fn new(config: AutoLockConfig, callback: Arc<dyn AutoLockCallback>) -> Self {
        Self {
            activity_tracker: ActivityTracker::new(config),
            callback,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> tokio::task::JoinHandle<()> {
        let activity_tracker = self.activity_tracker.clone();
        let callback = Arc::clone(&self.callback);
        let is_running = Arc::clone(&self.is_running);

        *is_running.write().await = true;

        tokio::spawn(async move {
            let check_interval = TokioDuration::from_secs(activity_tracker.get_config().check_interval_seconds);

            info!("Auto-lock service started");

            while *is_running.read().await {
                if activity_tracker.should_auto_lock().await {
                    info!("Auto-lock triggered due to inactivity");

                    match callback.on_auto_lock().await {
                        Ok(()) => {
                            debug!("Auto-lock callback executed successfully");
                        }
                        Err(e) => {
                            warn!("Auto-lock callback failed: {}", e);
                        }
                    }

                    // Reset activity after locking to prevent immediate re-triggering
                    activity_tracker.update_activity().await;
                }

                sleep(check_interval).await;
            }

            info!("Auto-lock service stopped");
        })
    }

    pub async fn stop(&self) {
        *self.is_running.write().await = false;
    }

    pub async fn update_activity(&self) {
        self.activity_tracker.update_activity().await;
    }

    pub async fn get_time_until_lock(&self) -> Option<chrono::Duration> {
        self.activity_tracker.time_until_lock().await
    }

    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.activity_tracker.get_config().enabled
    }
}
