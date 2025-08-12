use crate::manager::{BackupManager, VaultOperations};
use chamber_vault::BackupConfig;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
pub struct BackgroundService<V: VaultOperations + Send + 'static> {
    backup_manager: Arc<Mutex<BackupManager<V>>>,
    running: Arc<Mutex<bool>>,
}

impl<V: VaultOperations + Send + 'static> BackgroundService<V> {
    pub fn new(vault: V, config: BackupConfig) -> Self {
        let backup_manager = Arc::new(Mutex::new(BackupManager::new(vault, config)));
        let running = Arc::new(Mutex::new(false));

        Self {
            backup_manager,
            running,
        }
    }

    /// Starts the backup monitoring thread if it is not already running.
    ///
    /// This function initiates a thread that periodically checks whether a backup
    /// operation is needed. It acquires a lock on `self.running` to ensure that
    /// only one monitoring thread can run at a time. Once started, the thread will
    /// perform the following steps in a loop:
    /// 1. Check the current state of the `running` flag to determine if the thread
    ///    should continue execution.
    /// 2. Attempt to acquire a lock on the `backup_manager` to access the backup logic.
    /// 3. If the backup manager determines a backup is needed, attempt to execute the
    ///    backup. If the backup operation fails, an error message will be printed to
    ///    standard error. (Optional: Notification logic can be implemented here.)
    /// 4. Sleep for one hour before the next backup check.
    ///
    /// ## Remarks
    /// - The function uses `thread::spawn` to create a separate thread for backup
    ///   checks, enabling it to run independently of the main thread.
    /// - The sleep interval is hardcoded to 1 hour (`3600` seconds).
    /// - Errors during locking (`self.running` or `backup_manager`) are handled with
    ///   `.expect` to provide immediate feedback when locks fail to acquire.
    /// - Clippy warnings for the use of `.expect` are explicitly disabled, as they
    ///   are considered acceptable in this context to handle critical locking issues.
    ///
    /// ## Panics
    /// The function may panic in the following cases:
    /// - If acquiring the lock on `self.running` or `self.backup_manager` fails.
    /// - If any lock operation encounters a thread-poisoning error.
    ///
    /// ## Thread Safety
    /// - The function ensures proper synchronization by using a `Mutex` for the
    ///   `running` flag and `backup_manager`.
    /// - The `running` and `backup_manager` are wrapped in `Arc` (Atomically Reference Counted),
    ///   allowing shared ownership across threads.
    ///
    /// ## Lifetime
    /// - The thread will run continuously, checking the backup condition every hour, until
    ///   the `running` flag is set to `false`.
    ///
    /// ## Dependencies
    /// - `std::thread` for spawning the thread and sleeping.
    /// - `std::sync::{Arc, Mutex}` for synchronization.
    /// - `std::time::Duration` for specifying the sleep duration.
    ///
    /// ## Notes
    /// - Before calling `start`, ensure that the `BackupService` instance has been correctly
    ///   initialized with a `backup_manager` capable of handling the `backup_if_needed` logic.
    /// ```
    pub fn start(&self) {
        {
            #[allow(clippy::expect_used)]
            let mut running = self.running.lock().expect("Unable to acquire the lock for running");
            if *running {
                return; // Already running
            }
            *running = true;
        }

        let backup_manager = Arc::clone(&self.backup_manager);
        let running = Arc::clone(&self.running);

        #[allow(clippy::expect_used)]
        thread::spawn(move || {
            while *running.lock().expect("Unable to acquire the lock for running") {
                // Check for backup every hour
                if let Ok(mut manager) = backup_manager.lock() {
                    if let Err(e) = manager.backup_if_needed() {
                        eprintln!("Backup failed: {e}");
                        // Could send notification here
                    }
                }

                // Sleep for 1 hour
                thread::sleep(Duration::from_secs(3600));
            }
        });
    }

    /// Stops the current process or thread associated with the instance by setting the `running` flag
    /// to `false`. This effectively signals that the process or thread should terminate its execution.
    ///
    /// # Behavior
    /// - Acquires a lock on the `self.running` mutex to safely modify the shared `running` state.
    /// - If the lock cannot be acquired, it will panic with the message "Unable to get the lock (running)".
    /// - Updates the `running` boolean to `false` to indicate that the process or thread should stop.
    ///
    /// # Panics
    /// - This method will panic if it fails to acquire the lock for `self.running`. This panic may
    ///   occur due to a poisoned lock or other concurrency issues.
    ///
    /// # Allowances
    /// - The `clippy::expect_used` lint is explicitly allowed to permit the use of `.expect()` for
    ///   descriptive error handling in the context of this critical locking operation.
    #[allow(clippy::expect_used)]
    pub fn stop(&self) {
        let mut running = self.running.lock().expect("Unable to acquire the lock for running");
        *running = false;
    }
}
