//! Memory client for connecting to the daemon.
//!
//! Per HOOK-02: Hook handlers call daemon's IngestEvent RPC.

use tonic::transport::Channel;
use tracing::{debug, info};

use memory_service::pb::{
    memory_service_client::MemoryServiceClient,
    Event as ProtoEvent,
    EventRole as ProtoEventRole,
    EventType as ProtoEventType,
    IngestEventRequest,
};
use memory_types::{Event, EventRole, EventType};

use crate::error::ClientError;

/// Default endpoint for the memory daemon.
pub const DEFAULT_ENDPOINT: &str = "http://[::1]:50051";

/// Client for communicating with the memory daemon.
pub struct MemoryClient {
    inner: MemoryServiceClient<Channel>,
}

impl MemoryClient {
    /// Connect to the memory daemon.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The gRPC endpoint (e.g., "http://[::1]:50051")
    ///
    /// # Errors
    ///
    /// Returns `ClientError::Connection` if connection fails.
    pub async fn connect(endpoint: &str) -> Result<Self, ClientError> {
        info!("Connecting to memory daemon at {}", endpoint);
        let inner = MemoryServiceClient::connect(endpoint.to_string())
            .await
            .map_err(ClientError::Connection)?;
        Ok(Self { inner })
    }

    /// Connect to the default endpoint.
    pub async fn connect_default() -> Result<Self, ClientError> {
        Self::connect(DEFAULT_ENDPOINT).await
    }

    /// Ingest an event into the memory system.
    ///
    /// Per HOOK-02: Hook handlers call daemon's IngestEvent RPC.
    ///
    /// # Returns
    ///
    /// Returns the event_id and whether the event was newly created.
    pub async fn ingest(&mut self, event: Event) -> Result<(String, bool), ClientError> {
        debug!("Ingesting event: {}", event.event_id);

        let proto_event = event_to_proto(event);
        let request = tonic::Request::new(IngestEventRequest {
            event: Some(proto_event),
        });

        let response = self.inner.ingest_event(request).await?;
        let resp = response.into_inner();

        if resp.created {
            info!("Event ingested: {}", resp.event_id);
        } else {
            debug!("Event already existed (idempotent): {}", resp.event_id);
        }

        Ok((resp.event_id, resp.created))
    }

    /// Ingest multiple events in sequence.
    ///
    /// # Returns
    ///
    /// Returns the number of events newly created.
    pub async fn ingest_batch(&mut self, events: Vec<Event>) -> Result<usize, ClientError> {
        let mut created_count = 0;
        for event in events {
            let (_, created) = self.ingest(event).await?;
            if created {
                created_count += 1;
            }
        }
        Ok(created_count)
    }
}

/// Convert domain Event to proto Event.
fn event_to_proto(event: Event) -> ProtoEvent {
    let event_type = match event.event_type {
        EventType::SessionStart => ProtoEventType::SessionStart,
        EventType::UserMessage => ProtoEventType::UserMessage,
        EventType::AssistantMessage => ProtoEventType::AssistantMessage,
        EventType::ToolResult => ProtoEventType::ToolResult,
        EventType::AssistantStop => ProtoEventType::AssistantStop,
        EventType::SubagentStart => ProtoEventType::SubagentStart,
        EventType::SubagentStop => ProtoEventType::SubagentStop,
        EventType::SessionEnd => ProtoEventType::SessionEnd,
    };

    let role = match event.role {
        EventRole::User => ProtoEventRole::User,
        EventRole::Assistant => ProtoEventRole::Assistant,
        EventRole::System => ProtoEventRole::System,
        EventRole::Tool => ProtoEventRole::Tool,
    };

    ProtoEvent {
        event_id: event.event_id,
        session_id: event.session_id,
        timestamp_ms: event.timestamp.timestamp_millis(),
        event_type: event_type as i32,
        role: role as i32,
        text: event.text,
        metadata: event.metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_event_to_proto_user_message() {
        let event = Event::new(
            "evt-1".to_string(),
            "session-1".to_string(),
            Utc::now(),
            EventType::UserMessage,
            EventRole::User,
            "Hello!".to_string(),
        );

        let proto = event_to_proto(event);

        assert_eq!(proto.event_id, "evt-1");
        assert_eq!(proto.session_id, "session-1");
        assert_eq!(proto.event_type, ProtoEventType::UserMessage as i32);
        assert_eq!(proto.role, ProtoEventRole::User as i32);
        assert_eq!(proto.text, "Hello!");
    }

    #[test]
    fn test_event_to_proto_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), "value".to_string());

        let event = Event::new(
            "evt-1".to_string(),
            "session-1".to_string(),
            Utc::now(),
            EventType::ToolResult,
            EventRole::Tool,
            "Result".to_string(),
        ).with_metadata(metadata);

        let proto = event_to_proto(event);

        assert_eq!(proto.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_event_to_proto_all_types() {
        // Test all event type mappings
        let types = vec![
            (EventType::SessionStart, ProtoEventType::SessionStart),
            (EventType::UserMessage, ProtoEventType::UserMessage),
            (EventType::AssistantMessage, ProtoEventType::AssistantMessage),
            (EventType::ToolResult, ProtoEventType::ToolResult),
            (EventType::AssistantStop, ProtoEventType::AssistantStop),
            (EventType::SubagentStart, ProtoEventType::SubagentStart),
            (EventType::SubagentStop, ProtoEventType::SubagentStop),
            (EventType::SessionEnd, ProtoEventType::SessionEnd),
        ];

        for (domain_type, proto_type) in types {
            let event = Event::new(
                "evt-1".to_string(),
                "session-1".to_string(),
                Utc::now(),
                domain_type,
                EventRole::System,
                "Test".to_string(),
            );
            let proto = event_to_proto(event);
            assert_eq!(proto.event_type, proto_type as i32);
        }
    }

    #[test]
    fn test_event_to_proto_all_roles() {
        let roles = vec![
            (EventRole::User, ProtoEventRole::User),
            (EventRole::Assistant, ProtoEventRole::Assistant),
            (EventRole::System, ProtoEventRole::System),
            (EventRole::Tool, ProtoEventRole::Tool),
        ];

        for (domain_role, proto_role) in roles {
            let event = Event::new(
                "evt-1".to_string(),
                "session-1".to_string(),
                Utc::now(),
                EventType::UserMessage,
                domain_role,
                "Test".to_string(),
            );
            let proto = event_to_proto(event);
            assert_eq!(proto.role, proto_role as i32);
        }
    }
}
