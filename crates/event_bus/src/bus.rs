use crate::{Event, EventEnvelope};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Trait for event subscribers
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    async fn on_event(&self, event: &EventEnvelope);
}

/// Type-erased subscriber
type BoxedSubscriber = Arc<dyn EventSubscriber>;

/// The global event bus - singleton managing all pub/sub
pub struct EventBus {
    subscribers: RwLock<HashMap<String, Vec<BoxedSubscriber>>>,
    event_history_sender: Option<mpsc::UnboundedSender<EventEnvelope>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            event_history_sender: None,
        }
    }

    /// Connect to event history persistence
    pub fn connect_to_history(&mut self, sender: mpsc::UnboundedSender<EventEnvelope>) {
        self.event_history_sender = Some(sender);
    }

    /// Subscribe to events of a specific type
    pub fn subscribe(&self, event_type: &str, subscriber: BoxedSubscriber) {
        let mut subs = self.subscribers.write();
        subs.entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(subscriber);
    }

    /// Publish an event to all subscribers
    pub async fn publish<E: Event + serde::Serialize>(&self, event: &E) {
        let event_type = event.event_type();
        
        // Create envelope
        let payload = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
        let envelope = EventEnvelope::new(
            event_type.to_string(),
            "system".to_string(),
            payload,
        );

        // Store in history if connected
        if let Some(sender) = &self.event_history_sender {
            let _ = sender.send(envelope.clone());
        }

        // Notify subscribers
        let subscribers = {
            let subs = self.subscribers.read();
            subs.get(event_type).cloned()
        };

        if let Some(subscribers) = subscribers {
            for subscriber in subscribers {
                subscriber.on_event(&envelope).await;
            }
        }
    }

    /// Publish a raw envelope (for replaying history)
    pub async fn publish_envelope(&self, envelope: EventEnvelope) {
        let subscribers = {
            let subs = self.subscribers.read();
            subs.get(&envelope.event_type).cloned()
        };

        if let Some(subscribers) = subscribers {
            for subscriber in subscribers {
                subscriber.on_event(&envelope).await;
            }
        }
    }

    /// Get count of subscribers for a given event type
    pub fn subscriber_count(&self, event_type: &str) -> usize {
        let subs = self.subscribers.read();
        subs.get(event_type).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Global static event bus instance
static mut EVENT_BUS: Option<Arc<EventBus>> = None;
static INIT: std::sync::Once = std::sync::Once::new();

pub fn get_event_bus() -> Arc<EventBus> {
    unsafe {
        INIT.call_once(|| {
            EVENT_BUS = Some(Arc::new(EventBus::new()));
        });
        EVENT_BUS.as_ref().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestSubscriber {
        pub received: Arc<RwLock<Vec<String>>>,
    }
    
    #[async_trait]
    impl EventSubscriber for TestSubscriber {
        async fn on_event(&self, event: &EventEnvelope) {
            self.received.write().push(event.event_type.clone());
        }
    }
    
    #[tokio::test]
    async fn test_pub_sub() {
        let bus = EventBus::new();
        let received = Arc::new(RwLock::new(Vec::new()));
        let subscriber = Arc::new(TestSubscriber {
            received: received.clone(),
        });
        
        bus.subscribe("test_event", subscriber);
        
        // Create a test event (we'll need to implement a simple one)
        // For now, just verify the structure works
        assert_eq!(bus.subscriber_count("test_event"), 1);
    }
}

