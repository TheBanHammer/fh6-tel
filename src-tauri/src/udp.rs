use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::broadcast;

use crate::{
    db,
    event::ServerEvent,
    parser,
    session::SessionAction,
    AppState,
};

pub async fn run(state: Arc<AppState>, port: u16, tx: broadcast::Sender<ServerEvent>) {
    let addr = format!("0.0.0.0:{port}");
    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[udp] failed to bind {addr}: {e}");
            let _ = tx.send(ServerEvent::BindFailed(format!("Cannot bind port {port}: {e}")));
            return;
        }
    };
    println!("[udp] listening on {addr}");

    let mut buf = vec![0u8; 1024];
    let mut prev_in_event = false;
    let mut debug_logged = false;
    // Grace period before closing session — prevents pause-menu from splitting a run.
    // At ~30 packets/s, 150 ≈ 5 seconds of tolerance.
    let mut close_pending: u32 = 0;
    const CLOSE_GRACE: u32 = 150;

    loop {
        let (len, _) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[udp] recv error: {e}");
                continue;
            }
        };

        let raw = &buf[..len];

        if !debug_logged {
            debug_logged = true;
            println!("[udp] first packet: {len} bytes");
            if raw.len() >= 323 {
                let speed = f32::from_le_bytes(raw[256..260].try_into().unwrap_or([0; 4]));
                let thr = raw[315];
                let brk = raw[316];
                let gear = raw[319];
                let pos = raw[314];
                let tire_f_raw = f32::from_le_bytes(raw[268..272].try_into().unwrap_or([0; 4]));
                println!("[udp] speed={speed:.2}m/s thr={thr} brk={brk} gear={gear} race_pos={pos} tire_fl_raw={tire_f_raw:.1}°F");
            }
        }

        let pkt = match parser::parse(raw) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Always emit live data regardless of session state
        let _ = tx.send(ServerEvent::Tick(pkt.clone()));

        // Record whenever a lap is being timed: races/Rivals (race_position > 0)
        // and Time Trial (race_position 0 but the lap clock runs). Free-roam has
        // no lap timer so it stays unrecorded. Grace period stops pause-menu
        // packets from splitting a session.
        // Only the live lap clock means "timing now". last_lap / lap_number
        // persist as stale values from the previous race while you're in the
        // pre-race / menu screen, which spuriously spawned tiny sessions.
        let timed_lap = pkt.current_lap > 0.0;
        let raw_in_event = pkt.is_race_on && (pkt.race_position > 0 || timed_lap);
        if raw_in_event {
            close_pending = 0;
        } else {
            close_pending = close_pending.saturating_add(1);
        }
        let in_event = raw_in_event || close_pending < CLOSE_GRACE;

        handle_session(&state, &tx, &pkt, raw, prev_in_event, in_event);
        prev_in_event = in_event;
    }
}

fn handle_session(
    state: &AppState,
    tx: &broadcast::Sender<ServerEvent>,
    pkt: &parser::TelemetryPacket,
    raw: &[u8],
    prev_in_event: bool,
    in_event: bool,
) {
    let mut sm = state.session_manager.lock().unwrap();
    let db = state.db.lock().unwrap();

    let now_ms: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Progress timer for rewind detection. Races/Rivals report a cumulative
    // current_race_time; Time Trial leaves it at 0 and only the lap clock
    // advances — fall back to that so rewinds still stitch the session.
    let progress = if pkt.current_race_time > 0.0 {
        pkt.current_race_time
    } else {
        pkt.current_lap
    };

    // Apply event transition before inserting so the opening packet is captured
    let action = sm.on_race_on_change(prev_in_event, in_event, pkt.car_ordinal, pkt.car_class, pkt.car_pi);

    // Open/reopen first so the opening packet is captured below.
    if let SessionAction::Open { car_ordinal, car_class, car_pi } = &action {
        let (car_ordinal, car_class, car_pi) = (*car_ordinal, *car_class, *car_pi);
        // Check if the new stream looks like a rewind into the previous
        // session: race time went backward within the rewind window.
        if let Some(reopen_id) = sm.check_reopen(progress, now_ms) {
            match db::reopen_session(&db, reopen_id) {
                Ok(()) => {
                    sm.set_active_id(Some(reopen_id));
                    println!("[session] rewind detected, continuing #{reopen_id}");
                }
                Err(e) => eprintln!("[session] reopen error: {e}"),
            }
        } else {
            match db::open_session(&db, now_ms as i64, car_ordinal, car_class, car_pi) {
                Ok(id) => {
                    // Genuinely new race — reset all lap tracking. (A rewind
                    // reopen above deliberately does NOT, to continue the run.)
                    sm.begin_new_session();
                    sm.set_active_id(Some(id));
                    println!("[session] opened #{id}");
                }
                Err(e) => {
                    eprintln!("[session] open error: {e}");
                    let _ = tx.send(ServerEvent::SessionError(format!("Failed to open session: {e}")));
                }
            }
        }
    }

    // Record this packet while the session is still active.
    if let Some(session_id) = sm.active_session_id() {
        // Best lap is derived only from laps we time ourselves — Forza's
        // best_lap field carries stale/garbage values across sessions.
        sm.update_race_time(progress);
        if let Some(lap) = sm.note_tick(pkt.is_race_on, pkt.current_lap, pkt.current_race_time) {
            if let Err(e) = db::insert_lap(&db, session_id, lap.lap_number, lap.lap_time) {
                eprintln!("[session] lap insert error: {e}");
            }
        }
        if let Err(e) = db::insert_packet(&db, session_id, pkt.timestamp_ms, raw) {
            eprintln!("[session] insert error: {e}");
            let _ = tx.send(ServerEvent::SessionError(format!("Failed to write telemetry: {e}")));
        }
        // Lazily fill car metadata: the opening packet sometimes arrives before the
        // game has populated car_ordinal. This no-ops once car_ordinal is non-zero.
        if pkt.car_ordinal != 0 {
            db::update_session_car_if_unknown(&db, session_id, pkt.car_ordinal, pkt.car_class, pkt.car_pi).ok();
        }
    }

    // Close after recording.
    if matches!(action, SessionAction::Close) {
        if let Some(id) = sm.active_session_id() {
            sm.note_close(now_ms);
            // The final lap ends with the race ending (no line-crossing
            // reset), so record the in-progress lap here. Non-destructive: a
            // rewind reopen continues the same lap and a later close overwrites
            // this provisional value with the true (longer) one.
            let final_lap = sm.finalize_final_lap();
            if let Some(lap) = &final_lap {
                if let Err(e) = db::insert_lap(&db, id, lap.lap_number, lap.lap_time) {
                    eprintln!("[session] final lap insert error: {e}");
                }
            }
            // Discard only a *tiny* lapless session (a pre-race / aborted
            // fragment, ~10s). A longer lapless run is a real point-to-point
            // / sprint race and must be kept. ~400 packets ≈ 10s.
            if sm.laps_recorded() == 0 && final_lap.is_none() && sm.ticks() < 400 {
                if let Err(e) = db::delete_session(&db, id) {
                    eprintln!("[session] discard error: {e}");
                } else {
                    println!("[session] discarded empty session #{id}");
                }
            } else {
                // Best is the fastest lap actually in the table (rewind
                // upserts already corrected); -1.0 = none → keep existing.
                let best = db::min_lap_time(&db, id).ok().flatten().unwrap_or(-1.0);
                if let Err(e) = db::close_session(&db, id, now_ms as i64, best) {
                    eprintln!("[session] close error: {e}");
                    let _ = tx.send(ServerEvent::SessionError(format!("Failed to close session: {e}")));
                } else {
                    println!("[session] closed #{id}");
                }
            }
        }
        sm.set_active_id(None);
    }
}
