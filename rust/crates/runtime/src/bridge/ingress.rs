//! Session Ingress for receiving events from claude.ai

use std::sync::Arc;
use std::time::Duration;

use crate::bridge::api::BridgeFatalError;
use crate::bridge::types::*;

/// Session ingress error
#[derive(Debug, thiserror::Error)]
pub enum IngressError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Bridge fatal error: {0}")]
    Bridge(#[from] BridgeFatalError),

    #[error("Connection closed")]
    ConnectionClosed,
}

/// Session ingress event
#[derive(Debug, Clone)]
pub enum IngressEvent {
    /// Connected to the ingress endpoint
    Connected,

    /// Disconnected from the ingress endpoint
    Disconnected,

    /// Received a user input message
    UserInput(String),

    /// Received a control command
    ControlCommand(ControlCommand),

    /// Received a permission request
    PermissionRequest(PermissionRequest),

    /// Heartbeat ping
    Ping,
}

/// Control command from claude.ai
#[derive(Debug, Clone)]
pub enum ControlCommand {
    /// Stop the current session
    Stop,

    /// Pause the current session
    Pause,

    /// Resume a paused session
    Resume,

    /// Update session configuration
    UpdateConfig(serde_json::Value),

    /// Custom command
    Custom(String, serde_json::Value),
}

/// Permission request
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub request_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub timeout_ms: Option<u64>,
}

/// Session ingress configuration
#[derive(Debug, Clone)]
pub struct IngressConfig {
    pub ingress_url: String,
    pub session_token: String,
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
    pub ping_interval: Duration,
}

impl Default for IngressConfig {
    fn default() -> Self {
        Self {
            ingress_url: String::new(),
            session_token: String::new(),
            reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(2),
            ping_interval: Duration::from_secs(30),
        }
    }
}

/// Session ingress client
pub struct SessionIngress {
    config: IngressConfig,
    is_connected: bool,
}

impl SessionIngress {
    /// Create a new session ingress
    pub fn new(config: IngressConfig) -> Self {
        Self {
            config,
            is_connected: false,
        }
    }

    /// Connect to the ingress endpoint
    pub async fn connect(&mut self) -> Result<(), IngressError> {
        self.is_connected = true;
        Ok(())
    }

    /// Disconnect from the ingress endpoint
    pub async fn disconnect(&mut self) -> Result<(), IngressError> {
        self.is_connected = false;
        Ok(())
    }

    /// Receive the next event from the ingress
    pub async fn next_event(&mut self) -> Result<IngressEvent, IngressError> {
        Err(IngressError::ConnectionClosed)
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}

/// Ingress message sender for sending events back to claude.ai
pub struct IngressSender {
    session_token: String,
}

impl IngressSender {
    /// Create a new ingress sender
    pub fn new(session_token: String) -> Self {
        Self { session_token }
    }

    /// Send a permission response
    pub async fn send_permission_response(
        &self,
        response: PermissionResponseEvent,
    ) -> Result<(), IngressError> {
        Ok(())
    }

    /// Send a session activity
    pub async fn send_activity(
        &self,
        activity: SessionActivity,
    ) -> Result<(), IngressError> {
        Ok(())
    }
}
