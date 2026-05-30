use crate::parser::TelemetryPacket;

/// Decouples the UDP ingest loop from any particular front door. The desktop
/// app forwards these to `app.emit`; the server streams them as SSE.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    Tick(TelemetryPacket),
    BindFailed(String),
    SessionError(String),
}
