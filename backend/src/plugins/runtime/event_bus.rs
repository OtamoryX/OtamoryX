use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SubscriptionId = u64;
pub type EventHandlerFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub type EventHandler = Arc<dyn Fn(PluginEvent) -> EventHandlerFuture + Send + Sync + 'static>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Any,
    ArchiveImported,
    ArchiveUpdated,
    ArchiveDeleted,
    MetadataRequested,
    ScriptTriggered,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEvent {
    pub event_type: EventType,
    #[serde(default)]
    pub source_plugin_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
    pub emitted_at: DateTime<Utc>,
}

impl PluginEvent {
    pub fn new(event_type: EventType, payload: Value) -> Self {
        Self {
            event_type,
            source_plugin_id: None,
            payload,
            emitted_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventFilterSet {
    #[serde(default)]
    pub equals: HashMap<String, Value>,
}

impl EventFilterSet {
    pub fn matches(&self, event: &PluginEvent) -> bool {
        if self.equals.is_empty() {
            return true;
        }

        let payload = match &event.payload {
            Value::Object(map) => map,
            _ => return false,
        };

        self.equals
            .iter()
            .all(|(key, expected)| payload.get(key) == Some(expected))
    }
}

#[derive(Clone)]
struct SubscriberEntry {
    id: SubscriptionId,
    plugin_id: String,
    filters: EventFilterSet,
    handler: EventHandler,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginEventBusError {
    #[error("订阅不存在: {0}")]
    SubscriptionNotFound(SubscriptionId),
}

pub struct PluginEventBus {
    next_subscription_id: AtomicU64,
    subscribers: RwLock<HashMap<EventType, Vec<SubscriberEntry>>>,
}

impl Default for PluginEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginEventBus {
    pub fn new() -> Self {
        Self {
            next_subscription_id: AtomicU64::new(1),
            subscribers: RwLock::new(HashMap::new()),
        }
    }

    pub async fn subscribe(
        &self,
        plugin_id: impl Into<String>,
        event_type: EventType,
        filters: EventFilterSet,
        handler: EventHandler,
    ) -> SubscriptionId {
        let subscription_id = self.next_subscription_id.fetch_add(1, Ordering::Relaxed);
        let entry = SubscriberEntry {
            id: subscription_id,
            plugin_id: plugin_id.into(),
            filters,
            handler,
        };

        let mut guard = self.subscribers.write().await;
        guard.entry(event_type).or_default().push(entry);
        subscription_id
    }

    pub async fn unsubscribe(
        &self,
        subscription_id: SubscriptionId,
    ) -> Result<(), PluginEventBusError> {
        let mut guard = self.subscribers.write().await;
        for entries in guard.values_mut() {
            if let Some(index) = entries.iter().position(|entry| entry.id == subscription_id) {
                entries.remove(index);
                return Ok(());
            }
        }

        Err(PluginEventBusError::SubscriptionNotFound(subscription_id))
    }

    pub async fn unsubscribe_plugin(&self, plugin_id: &str) -> usize {
        let mut removed = 0;
        let mut guard = self.subscribers.write().await;

        for entries in guard.values_mut() {
            let before = entries.len();
            entries.retain(|entry| entry.plugin_id != plugin_id);
            removed += before - entries.len();
        }

        removed
    }

    pub async fn publish(&self, event: PluginEvent) -> usize {
        let mut handlers: Vec<EventHandler> = Vec::new();
        {
            let guard = self.subscribers.read().await;
            if let Some(entries) = guard.get(&event.event_type) {
                for entry in entries {
                    if entry.filters.matches(&event) {
                        handlers.push(entry.handler.clone());
                    }
                }
            }
            if let Some(entries) = guard.get(&EventType::Any) {
                for entry in entries {
                    if entry.filters.matches(&event) {
                        handlers.push(entry.handler.clone());
                    }
                }
            }
        }

        // TODO(P7-8): add backpressure, retry strategy, and dispatch result tracking.
        for handler in &handlers {
            let callback = handler.clone();
            let dispatch_event = event.clone();
            tokio::spawn(async move {
                (callback)(dispatch_event).await;
            });
        }

        handlers.len()
    }
}
