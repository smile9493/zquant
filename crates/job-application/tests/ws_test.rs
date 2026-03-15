use futures::{SinkExt, StreamExt};
use job_application::WsState;
use job_events::bus::{Event, EventBus, InMemoryEventBus};
use job_events::types::{AgentMessageProduced, JobCreated};
use job_store_pg::JobStore;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};

async fn setup_test_env() -> (PgPool, Arc<JobStore>, Arc<dyn EventBus>) {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/zquant_test".to_string());
    let pool = PgPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();

    let store = Arc::new(JobStore::new(pool.clone()));
    let bus = Arc::new(InMemoryEventBus::new(100)) as Arc<dyn EventBus>;

    (pool, store, bus)
}

#[tokio::test]
async fn test_ws_hello_and_snapshot() {
    let (_pool, store, bus) = setup_test_env().await;

    let ws_state = WsState {
        store: store.clone(),
        bus: bus.clone(),
    };

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(job_application::ws_handler))
        .with_state(ws_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (_write, mut read) = ws_stream.split();

    if let Some(Ok(TungsteniteMessage::Text(text))) = read.next().await {
        let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(msg["type"], "hello");
        assert_eq!(msg["v"], 1);
    } else {
        panic!("Expected hello message");
    }

    if let Some(Ok(TungsteniteMessage::Text(text))) = read.next().await {
        let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(msg["type"], "snapshot");
        assert!(msg["data"]["jobs"].is_array());
    } else {
        panic!("Expected snapshot message");
    }
}

#[tokio::test]
async fn test_ws_job_created_event() {
    let (_pool, store, bus) = setup_test_env().await;

    let ws_state = WsState {
        store: store.clone(),
        bus: bus.clone(),
    };

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(job_application::ws_handler))
        .with_state(ws_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (_write, mut read) = ws_stream.split();

    read.next().await;
    read.next().await;

    bus.publish(Event::JobCreated(JobCreated {
        job_id: "test_job_1".to_string(),
        job_type: "test".to_string(),
        created_at: chrono::Utc::now(),
    }));

    tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
        if let Some(Ok(TungsteniteMessage::Text(text))) = read.next().await {
            let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(msg["type"], "event");
            assert_eq!(msg["data"]["kind"], "job.created");
            assert_eq!(msg["data"]["payload"]["job_id"], "test_job_1");
        } else {
            panic!("Expected job.created event");
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn test_ws_subscribe_job_logs() {
    let (_pool, store, bus) = setup_test_env().await;

    let ws_state = WsState {
        store: store.clone(),
        bus: bus.clone(),
    };

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(job_application::ws_handler))
        .with_state(ws_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let url = format!("ws://{}/ws", addr);
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    read.next().await;
    read.next().await;

    let subscribe_msg = json!({
        "type": "subscribe",
        "data": {
            "job_id": "job_1"
        }
    });
    write.send(TungsteniteMessage::Text(subscribe_msg.to_string())).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    bus.publish(Event::AgentMessageProduced(AgentMessageProduced {
        job_id: "job_1".to_string(),
        agent_id: "agent_1".to_string(),
        message_type: "info".to_string(),
        content: json!("test message"),
        ts: chrono::Utc::now(),
    }));

    tokio::time::timeout(tokio::time::Duration::from_secs(2), async {
        if let Some(Ok(TungsteniteMessage::Text(text))) = read.next().await {
            let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
            assert_eq!(msg["type"], "log");
            assert_eq!(msg["data"]["job_id"], "job_1");
        } else {
            panic!("Expected log message");
        }
    })
    .await
    .unwrap();

    bus.publish(Event::AgentMessageProduced(AgentMessageProduced {
        job_id: "job_2".to_string(),
        agent_id: "agent_2".to_string(),
        message_type: "info".to_string(),
        content: json!("test message 2"),
        ts: chrono::Utc::now(),
    }));

    let result = tokio::time::timeout(tokio::time::Duration::from_millis(500), read.next()).await;
    assert!(result.is_err(), "Should not receive log for unsubscribed job");
}
