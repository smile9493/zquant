//! Lightweight task runtime for desktop-embedded job execution.
//!
//! Provides a state machine for task lifecycle management:
//! `Pending -> Running -> Success | Failed | Cancelled`

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, watch};
use tracing::{debug, info, warn};

/// Task status in the lifecycle state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Success => write!(f, "success"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl TaskStatus {
    /// Check if this status is terminal (no further transitions allowed).
    pub fn is_terminal(self) -> bool {
        matches!(self, TaskStatus::Success | TaskStatus::Failed | TaskStatus::Cancelled)
    }
}

/// Valid state transitions for the task lifecycle.
fn validate_transition(from: TaskStatus, to: TaskStatus) -> bool {
    matches!(
        (from, to),
        (TaskStatus::Pending, TaskStatus::Running)
            | (TaskStatus::Pending, TaskStatus::Cancelled)
            | (TaskStatus::Running, TaskStatus::Success)
            | (TaskStatus::Running, TaskStatus::Failed)
            | (TaskStatus::Running, TaskStatus::Cancelled)
    )
}

/// Unique task identifier.
pub type TaskId = u64;

/// A task entry tracked by the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub id: TaskId,
    pub name: String,
    pub status: TaskStatus,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Event emitted when a task changes state.
#[derive(Debug, Clone)]
pub struct TaskEvent {
    pub id: TaskId,
    pub status: TaskStatus,
    pub message: Option<String>,
}

/// Handle returned when a task is submitted, allowing cancellation.
#[derive(Clone)]
pub struct TaskHandle {
    id: TaskId,
    cancel_tx: watch::Sender<bool>,
}

impl TaskHandle {
    /// Request cancellation of this task.
    pub fn cancel(&self) {
        let _ = self.cancel_tx.send(true);
        debug!(task_id = self.id, "Cancellation requested");
    }

    pub fn id(&self) -> TaskId {
        self.id
    }
}

/// The task runtime manages submission, execution, and lifecycle of tasks.
pub struct TaskRuntime {
    tasks: Arc<Mutex<HashMap<TaskId, TaskEntry>>>,
    cancel_senders: Arc<Mutex<HashMap<TaskId, watch::Sender<bool>>>>,
    next_id: Arc<Mutex<TaskId>>,
    event_tx: mpsc::UnboundedSender<TaskEvent>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<TaskEvent>>>,
}

impl TaskRuntime {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            cancel_senders: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
        }
    }

    /// Submit a task for execution. The provided async closure is the task body.
    /// Returns a TaskHandle for cancellation.
    pub async fn submit<F, Fut>(&self, name: &str, task_fn: F) -> TaskHandle
    where
        F: FnOnce(watch::Receiver<bool>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
    {
        let id = {
            let mut next = self.next_id.lock().await;
            let id = *next;
            *next += 1;
            id
        };

        let now = Utc::now();
        let entry = TaskEntry {
            id,
            name: name.to_string(),
            status: TaskStatus::Pending,
            message: None,
            created_at: now,
            updated_at: now,
        };

        {
            let mut tasks = self.tasks.lock().await;
            tasks.insert(id, entry);
        }

        let _ = self.event_tx.send(TaskEvent {
            id,
            status: TaskStatus::Pending,
            message: None,
        });

        info!(task_id = id, name, "Task submitted");

        let (cancel_tx, cancel_rx) = watch::channel(false);

        // Register cancel sender so cancel(id) can route the signal
        {
            let mut senders = self.cancel_senders.lock().await;
            senders.insert(id, cancel_tx.clone());
        }

        let tasks = self.tasks.clone();
        let event_tx = self.event_tx.clone();
        let cancel_senders = self.cancel_senders.clone();

        tokio::spawn(async move {
            // Transition to Running
            Self::update_status(&tasks, &event_tx, id, TaskStatus::Running, None).await;

            // Execute the task
            let result = task_fn(cancel_rx).await;

            // Clean up cancel sender (task is finishing)
            {
                let mut senders = cancel_senders.lock().await;
                senders.remove(&id);
            }

            // Check if cancelled during execution
            let current_status = {
                let t = tasks.lock().await;
                t.get(&id).map(|e| e.status)
            };

            if current_status == Some(TaskStatus::Cancelled) {
                return; // Already cancelled
            }

            match result {
                Ok(msg) => {
                    Self::update_status(&tasks, &event_tx, id, TaskStatus::Success, Some(msg)).await;
                }
                Err(e) => {
                    Self::update_status(&tasks, &event_tx, id, TaskStatus::Failed, Some(format!("{e:#}"))).await;
                }
            }
        });

        TaskHandle { id, cancel_tx }
    }

    /// Cancel a task by ID. Sends cancellation signal to the running task
    /// and updates the status to Cancelled.
    pub async fn cancel(&self, id: TaskId) -> bool {
        let mut tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&id) {
            if entry.status == TaskStatus::Running || entry.status == TaskStatus::Pending {
                // Send cancellation signal to the task's execution context
                {
                    let mut senders = self.cancel_senders.lock().await;
                    if let Some(tx) = senders.remove(&id) {
                        let _ = tx.send(true);
                        debug!(task_id = id, "Cancel signal sent to task");
                    }
                }

                entry.status = TaskStatus::Cancelled;
                entry.updated_at = Utc::now();
                entry.message = Some("Cancelled by user".to_string());
                let _ = self.event_tx.send(TaskEvent {
                    id,
                    status: TaskStatus::Cancelled,
                    message: Some("Cancelled by user".to_string()),
                });
                info!(task_id = id, "Task cancelled");
                return true;
            }
        }
        false
    }

    /// Get a snapshot of all tasks.
    pub async fn list_tasks(&self) -> Vec<TaskEntry> {
        let tasks = self.tasks.lock().await;
        let mut entries: Vec<_> = tasks.values().cloned().collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries
    }

    /// Get a specific task by ID.
    pub async fn get_task(&self, id: TaskId) -> Option<TaskEntry> {
        let tasks = self.tasks.lock().await;
        tasks.get(&id).cloned()
    }

    /// Drain pending events (non-blocking).
    pub async fn drain_events(&self) -> Vec<TaskEvent> {
        let mut rx = self.event_rx.lock().await;
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        events
    }

    async fn update_status(
        tasks: &Arc<Mutex<HashMap<TaskId, TaskEntry>>>,
        event_tx: &mpsc::UnboundedSender<TaskEvent>,
        id: TaskId,
        new_status: TaskStatus,
        message: Option<String>,
    ) {
        let mut tasks = tasks.lock().await;
        if let Some(entry) = tasks.get_mut(&id) {
            if !validate_transition(entry.status, new_status) {
                warn!(
                    task_id = id,
                    from = %entry.status,
                    to = %new_status,
                    "Invalid state transition, ignoring"
                );
                return;
            }
            entry.status = new_status;
            entry.updated_at = Utc::now();
            entry.message = message.clone();

            debug!(task_id = id, status = %new_status, "Task status updated");

            let _ = event_tx.send(TaskEvent {
                id,
                status: new_status,
                message,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_display() {
        assert_eq!(TaskStatus::Pending.to_string(), "pending");
        assert_eq!(TaskStatus::Running.to_string(), "running");
        assert_eq!(TaskStatus::Success.to_string(), "success");
        assert_eq!(TaskStatus::Failed.to_string(), "failed");
        assert_eq!(TaskStatus::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn terminal_states() {
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(TaskStatus::Success.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
    }

    #[test]
    fn valid_transitions() {
        assert!(validate_transition(TaskStatus::Pending, TaskStatus::Running));
        assert!(validate_transition(TaskStatus::Pending, TaskStatus::Cancelled));
        assert!(validate_transition(TaskStatus::Running, TaskStatus::Success));
        assert!(validate_transition(TaskStatus::Running, TaskStatus::Failed));
        assert!(validate_transition(TaskStatus::Running, TaskStatus::Cancelled));
    }

    #[test]
    fn invalid_transitions() {
        assert!(!validate_transition(TaskStatus::Pending, TaskStatus::Success));
        assert!(!validate_transition(TaskStatus::Pending, TaskStatus::Failed));
        assert!(!validate_transition(TaskStatus::Running, TaskStatus::Pending));
        assert!(!validate_transition(TaskStatus::Success, TaskStatus::Running));
        assert!(!validate_transition(TaskStatus::Failed, TaskStatus::Running));
        assert!(!validate_transition(TaskStatus::Cancelled, TaskStatus::Running));
    }

    #[tokio::test]
    async fn submit_and_complete_task() {
        let rt = TaskRuntime::new();

        let handle = rt.submit("test-task", |_cancel_rx| async {
            Ok("done".to_string())
        }).await;

        // Wait for task to complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let task = rt.get_task(handle.id()).await.unwrap();
        assert_eq!(task.status, TaskStatus::Success);
        assert_eq!(task.message.as_deref(), Some("done"));
    }

    #[tokio::test]
    async fn submit_failing_task() {
        let rt = TaskRuntime::new();

        let handle = rt.submit("fail-task", |_cancel_rx| async {
            anyhow::bail!("something went wrong")
        }).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let task = rt.get_task(handle.id()).await.unwrap();
        assert_eq!(task.status, TaskStatus::Failed);
        assert!(task.message.as_deref().unwrap().contains("something went wrong"));
    }

    #[tokio::test]
    async fn cancel_pending_task() {
        let rt = TaskRuntime::new();

        // Submit a task that waits
        let handle = rt.submit("slow-task", |mut cancel_rx| async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {},
                    _ = cancel_rx.changed() => {
                        return Ok("cancelled".to_string());
                    }
                }
            }
        }).await;

        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let cancelled = rt.cancel(handle.id()).await;
        assert!(cancelled);

        let task = rt.get_task(handle.id()).await.unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn list_tasks_returns_all() {
        let rt = TaskRuntime::new();

        rt.submit("task-1", |_| async { Ok("ok".to_string()) }).await;
        rt.submit("task-2", |_| async { Ok("ok".to_string()) }).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let tasks = rt.list_tasks().await;
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn drain_events_returns_lifecycle_events() {
        let rt = TaskRuntime::new();

        rt.submit("event-task", |_| async { Ok("ok".to_string()) }).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let events = rt.drain_events().await;
        // Should have: Pending, Running, Success
        assert!(events.len() >= 3, "Expected at least 3 events, got {}", events.len());
    }

    #[tokio::test]
    async fn cannot_cancel_completed_task() {
        let rt = TaskRuntime::new();

        let handle = rt.submit("quick-task", |_| async { Ok("ok".to_string()) }).await;

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let cancelled = rt.cancel(handle.id()).await;
        assert!(!cancelled, "Should not be able to cancel a completed task");
    }

    #[tokio::test]
    async fn cancel_running_task_propagates_signal_and_stops() {
        let rt = TaskRuntime::new();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel::<bool>();

        let handle = rt.submit("cancellable-task", |mut cancel_rx| async move {
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                    let _ = done_tx.send(false); // should not reach here
                    Ok("completed normally".to_string())
                }
                _ = cancel_rx.changed() => {
                    let _ = done_tx.send(true); // signal was received
                    Ok("got cancel signal".to_string())
                }
            }
        }).await;

        // Wait for task to start running
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        let cancelled = rt.cancel(handle.id()).await;
        assert!(cancelled, "cancel() should return true for running task");

        // Verify the task closure actually received the cancel signal
        let signal_received = tokio::time::timeout(
            std::time::Duration::from_millis(200),
            done_rx,
        ).await;
        assert!(
            matches!(signal_received, Ok(Ok(true))),
            "Task closure should have received cancel signal"
        );

        let task = rt.get_task(handle.id()).await.unwrap();
        assert_eq!(task.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancelled_task_does_not_emit_success_after_cancel() {
        let rt = TaskRuntime::new();

        let handle = rt.submit("race-task", |mut cancel_rx| async move {
            // Wait for cancel signal, then return Ok — should NOT override Cancelled
            let _ = cancel_rx.changed().await;
            Ok("should be ignored".to_string())
        }).await;

        tokio::time::sleep(std::time::Duration::from_millis(30)).await;

        rt.cancel(handle.id()).await;

        // Give spawned task time to finish its closure
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let task = rt.get_task(handle.id()).await.unwrap();
        assert_eq!(
            task.status,
            TaskStatus::Cancelled,
            "Status must remain Cancelled even if closure returned Ok"
        );
    }
}
