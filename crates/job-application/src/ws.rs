use axum::{
    extract::{ws::{WebSocket, Message}, State, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use job_events::bus::{Event, EventBus};
use job_store_pg::JobStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct WsState {
    pub store: Arc<JobStore>,
    pub bus: Arc<dyn EventBus>,
}

#[derive(Serialize, Deserialize)]
struct WsMessage {
    v: u8,
    #[serde(rename = "type")]
    msg_type: String,
    ts: String,
    data: serde_json::Value,
}

#[derive(Deserialize)]
struct ClientMessage {
    #[serde(rename = "type")]
    msg_type: String,
    data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct SubscribeData {
    job_id: String,
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<WsState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WsState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.bus.subscribe();
    let mut subscribed_jobs: HashSet<String> = HashSet::new();
    let conn_id = uuid::Uuid::new_v4().to_string();

    tracing::info!(conn_id = %conn_id, "WebSocket connection established");

    // Send hello
    let hello = WsMessage {
        v: 1,
        msg_type: "hello".to_string(),
        ts: chrono::Utc::now().to_rfc3339(),
        data: serde_json::json!({"server": "job-kernel", "schema_v": "1.0"}),
    };
    if let Ok(msg) = serde_json::to_string(&hello) {
        let _ = sender.send(Message::Text(msg)).await;
    }

    // Send snapshot
    if let Ok(jobs) = state.store.list_jobs().await {
        let summaries: Vec<_> = jobs
            .into_iter()
            .map(|job| {
                serde_json::json!({
                    "job_id": job.job_id,
                    "job_type": job.job_type,
                    "status": format!("{:?}", job.status).to_lowercase(),
                    "stop_requested": job.stop_requested,
                    "created_at": job.created_at,
                    "updated_at": job.updated_at,
                })
            })
            .collect();

        let snapshot = WsMessage {
            v: 1,
            msg_type: "snapshot".to_string(),
            ts: chrono::Utc::now().to_rfc3339(),
            data: serde_json::json!({"health": {"status": "healthy"}, "jobs": summaries}),
        };
        if let Ok(msg) = serde_json::to_string(&snapshot) {
            let _ = sender.send(Message::Text(msg)).await;
        }
    }

    let (tx, mut rx_client) = tokio::sync::mpsc::channel::<String>(100);
    let tx_clone = tx.clone();

    // Event forwarding task
    let forward_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let msg = event_to_message(event);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if tx_clone.send(json).await.is_err() {
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    tracing::warn!("WS client lagged, skipping events");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    loop {
        tokio::select! {
            Some(msg) = rx_client.recv() => {
                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&msg) {
                    if ws_msg.msg_type == "log" {
                        if let Some(job_id) = ws_msg.data.get("job_id").and_then(|v| v.as_str()) {
                            if !subscribed_jobs.contains(job_id) {
                                continue;
                            }
                        }
                    }
                }
                if sender.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
            Some(Ok(msg)) = receiver.next() => {
                if let Message::Text(text) = msg {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        if client_msg.msg_type == "subscribe" {
                            if let Some(data) = client_msg.data {
                                if let Ok(sub) = serde_json::from_value::<SubscribeData>(data) {
                                    subscribed_jobs.insert(sub.job_id);
                                }
                            }
                        }
                    }
                }
            }
            else => break,
        }
    }

    forward_task.abort();
    tracing::info!(conn_id = %conn_id, subscribed_jobs = subscribed_jobs.len(), "WebSocket connection closed");
}

fn event_to_message(event: Event) -> WsMessage {
    let ts = chrono::Utc::now().to_rfc3339();
    match event {
        Event::JobCreated(e) => WsMessage {
            v: 1,
            msg_type: "event".to_string(),
            ts,
            data: serde_json::json!({"kind": "job.created", "payload": e}),
        },
        Event::JobStarted(e) => WsMessage {
            v: 1,
            msg_type: "event".to_string(),
            ts,
            data: serde_json::json!({"kind": "job.started", "payload": e}),
        },
        Event::JobCompleted(e) => WsMessage {
            v: 1,
            msg_type: "event".to_string(),
            ts,
            data: serde_json::json!({"kind": "job.completed", "payload": e}),
        },
        Event::AgentMessageProduced(e) => WsMessage {
            v: 1,
            msg_type: "log".to_string(),
            ts: ts.clone(),
            data: serde_json::json!({
                "job_id": e.job_id,
                "entry": {
                    "timestamp": ts,
                    "level": "info",
                    "message": format!("{}: {}", e.message_type, e.content)
                }
            }),
        },
        _ => WsMessage {
            v: 1,
            msg_type: "event".to_string(),
            ts,
            data: serde_json::json!({"kind": "other"}),
        },
    }
}
