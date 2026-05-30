use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use super::ServerState;
use crate::event::ServerEvent;

pub async fn events(
    State(s): State<ServerState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = s.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| {
        let ev = res.ok()?;
        let event = match ev {
            ServerEvent::Tick(pkt) => Event::default().event("telemetry_tick").json_data(&pkt).ok()?,
            ServerEvent::BindFailed(msg) => Event::default().event("udp_bind_failed").data(msg),
            ServerEvent::SessionError(msg) => Event::default().event("session_error").data(msg),
        };
        Some(Ok(event))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn broadcast_roundtrip() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<ServerEvent>(8);
        tx.send(ServerEvent::SessionError("hello".into())).unwrap();
        let got = rx.recv().await.unwrap();
        assert!(matches!(got, ServerEvent::SessionError(_)));
    }
}
