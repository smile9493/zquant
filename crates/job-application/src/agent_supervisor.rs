use chrono::Utc;
use job_events::{
    bus::{Event, EventBus},
    types::{AgentMessageProduced, AgentSpawnRequested, AgentTaskScheduled},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;

const DEFAULT_AGENT_CHANNEL_CAPACITY: usize = 64;
const DEFAULT_MAX_PENDING_TASKS_PER_AGENT: usize = 32;

struct AgentHandle {
    task_tx: mpsc::Sender<AgentTaskScheduled>,
}

pub struct AgentSupervisor {
    bus: Arc<dyn EventBus>,
    rx: broadcast::Receiver<Event>,
    agents: HashMap<String, AgentHandle>,
    pending: HashMap<String, VecDeque<AgentTaskScheduled>>,
    agent_channel_capacity: usize,
    max_pending_tasks_per_agent: usize,
}

impl AgentSupervisor {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        let rx = bus.subscribe();
        Self {
            bus,
            rx,
            agents: HashMap::new(),
            pending: HashMap::new(),
            agent_channel_capacity: DEFAULT_AGENT_CHANNEL_CAPACITY,
            max_pending_tasks_per_agent: DEFAULT_MAX_PENDING_TASKS_PER_AGENT,
        }
    }

    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(Event::AgentSpawnRequested(evt)) => self.on_spawn(evt),
                Ok(Event::AgentTaskScheduled(evt)) => self.on_schedule(evt),
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "AgentSupervisor lagged while receiving events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    }

    fn on_spawn(&mut self, evt: AgentSpawnRequested) {
        if self.agents.contains_key(&evt.agent_id) {
            tracing::warn!(agent_id = %evt.agent_id, "Agent already exists; ignoring spawn");
            return;
        }

        tracing::info!(
            agent_id = %evt.agent_id,
            job_id = %evt.job_id,
            agent_kind = %evt.agent_kind,
            "Spawning agent"
        );

        let (task_tx, task_rx) = mpsc::channel(self.agent_channel_capacity);
        let bus = self.bus.clone();
        let agent_id = evt.agent_id.clone();
        let job_id = evt.job_id.clone();

        tokio::spawn(async move {
            run_agent(agent_id, job_id, bus, task_rx).await;
        });

        self.agents
            .insert(evt.agent_id.clone(), AgentHandle { task_tx });

        self.flush_pending(&evt.agent_id);
    }

    fn on_schedule(&mut self, evt: AgentTaskScheduled) {
        let agent_id = evt.agent_id.clone();

        let Some(handle) = self.agents.get(&agent_id) else {
            let q = self.pending.entry(agent_id.clone()).or_default();
            if q.len() >= self.max_pending_tasks_per_agent {
                tracing::warn!(
                    agent_id = %agent_id,
                    max_pending = self.max_pending_tasks_per_agent,
                    "Dropping scheduled task: agent not spawned and pending queue full"
                );
                return;
            }

            tracing::warn!(
                agent_id = %agent_id,
                task_id = %evt.task_id,
                "Buffering scheduled task: agent not spawned yet"
            );
            q.push_back(evt);
            return;
        };

        let task_tx = handle.task_tx.clone();
        match task_tx.try_send(evt) {
            Ok(()) => {}
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!(agent_id = %agent_id, "Dropping scheduled task: agent channel full");
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(evt)) => {
                tracing::warn!(agent_id = %agent_id, "Agent channel closed; removing agent and buffering task");
                self.agents.remove(&agent_id);
                self.on_schedule(evt);
            }
        }
    }

    fn flush_pending(&mut self, agent_id: &str) {
        let Some(handle) = self.agents.get(agent_id) else {
            return;
        };
        let task_tx = handle.task_tx.clone();

        let Some(q) = self.pending.get_mut(agent_id) else {
            return;
        };

        while let Some(task) = q.pop_front() {
            match task_tx.try_send(task) {
                Ok(()) => continue,
                Err(tokio::sync::mpsc::error::TrySendError::Full(task)) => {
                    q.push_front(task);
                    tracing::warn!(agent_id = %agent_id, "Agent channel full; leaving pending tasks queued");
                    break;
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_task)) => {
                    tracing::warn!(agent_id = %agent_id, "Agent channel closed; removing agent and dropping pending tasks");
                    self.agents.remove(agent_id);
                    q.clear();
                    break;
                }
            }
        }

        if q.is_empty() {
            self.pending.remove(agent_id);
        }
    }
}

async fn run_agent(
    agent_id: String,
    job_id: String,
    bus: Arc<dyn EventBus>,
    mut task_rx: mpsc::Receiver<AgentTaskScheduled>,
) {
    while let Some(task) = task_rx.recv().await {
        let content = serde_json::json!({
            "task_id": task.task_id,
            "task_payload": task.task_payload,
            "deadline": task.deadline,
        });

        bus.publish(Event::AgentMessageProduced(AgentMessageProduced {
            agent_id: agent_id.clone(),
            job_id: job_id.clone(),
            message_type: "task_completed".to_string(),
            content,
            ts: Utc::now(),
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use job_events::bus::InMemoryEventBus;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn agent_spawn_schedule_produces_message() {
        let bus = Arc::new(InMemoryEventBus::new(64));
        let bus_dyn = bus.clone() as Arc<dyn EventBus>;

        let mut rx = bus.subscribe();
        let supervisor_task = tokio::spawn(AgentSupervisor::new(bus_dyn.clone()).run());

        bus_dyn.publish(Event::AgentSpawnRequested(AgentSpawnRequested {
            agent_id: "agent-1".to_string(),
            job_id: "job-1".to_string(),
            agent_kind: "test".to_string(),
            init_payload: serde_json::json!({"init": true}),
        }));

        bus_dyn.publish(Event::AgentTaskScheduled(AgentTaskScheduled {
            agent_id: "agent-1".to_string(),
            task_id: "task-1".to_string(),
            task_payload: serde_json::json!({"hello": "world"}),
            deadline: None,
        }));

        let produced = timeout(Duration::from_secs(2), async {
            loop {
                match rx.recv().await {
                    Ok(Event::AgentMessageProduced(e)) => break e,
                    Ok(_) => continue,
                    Err(_) => continue,
                }
            }
        })
        .await
        .expect("timeout waiting for AgentMessageProduced");

        assert_eq!(produced.agent_id, "agent-1");
        assert_eq!(produced.job_id, "job-1");
        assert_eq!(produced.message_type, "task_completed");
        assert_eq!(produced.content["task_id"], "task-1");

        supervisor_task.abort();
    }
}
