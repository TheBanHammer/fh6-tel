use crate::{db, settings, AppState};

pub fn list_sessions(state: &AppState) -> Result<Vec<db::SessionRow>, String> {
    let conn = state.db.lock().unwrap();
    db::list_sessions(&conn).map_err(|e| e.to_string())
}

pub fn session_packets(
    state: &AppState,
    session_id: i64,
) -> Result<Vec<crate::parser::TelemetryPacket>, String> {
    let conn = state.db.lock().unwrap();
    let blobs = db::get_session_packets(&conn, session_id).map_err(|e| e.to_string())?;
    Ok(blobs
        .iter()
        .filter_map(|b| crate::parser::parse(b).ok())
        .collect())
}

pub fn session_laps(state: &AppState, session_id: i64) -> Result<Vec<db::LapRow>, String> {
    let conn = state.db.lock().unwrap();
    db::get_session_laps(&conn, session_id).map_err(|e| e.to_string())
}

pub fn delete_session(state: &AppState, session_id: i64) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::delete_session(&conn, session_id).map_err(|e| e.to_string())
}

pub fn clear_all_sessions(state: &AppState) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::clear_all_sessions(&conn).map_err(|e| e.to_string())
}

pub fn rename_session(
    state: &AppState,
    session_id: i64,
    name: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::rename_session(&conn, session_id, name.as_deref()).map_err(|e| e.to_string())
}

pub fn set_session_bookmark(
    state: &AppState,
    session_id: i64,
    bookmarked: bool,
) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::set_session_bookmark(&conn, session_id, bookmarked).map_err(|e| e.to_string())
}

pub fn get_settings(state: &AppState) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

pub fn save_settings(state: &AppState, new_settings: settings::Settings) -> Result<(), String> {
    settings::save(&new_settings).map_err(|e| e.to_string())?;
    let auto_record = new_settings.auto_record;
    *state.settings.lock().unwrap() = new_settings;
    state
        .session_manager
        .lock()
        .unwrap()
        .set_auto_record(auto_record);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use std::sync::Mutex;

    // save_settings is covered by the server integration test (Task 7) — it writes to disk.

    fn test_state() -> AppState {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        db::init(&conn).unwrap();
        AppState {
            db: Mutex::new(conn),
            session_manager: Mutex::new(SessionManager::new(true)),
            settings: Mutex::new(settings::Settings::default()),
        }
    }

    #[test]
    fn list_is_empty_then_reflects_inserts() {
        let state = test_state();
        assert_eq!(list_sessions(&state).unwrap().len(), 0);
        {
            let conn = state.db.lock().unwrap();
            db::open_session(&conn, 1000, 42, 5, 800).unwrap();
        }
        assert_eq!(list_sessions(&state).unwrap().len(), 1);
    }

    #[test]
    fn rename_and_bookmark_and_delete_roundtrip() {
        let state = test_state();
        let id = {
            let conn = state.db.lock().unwrap();
            db::open_session(&conn, 1000, 42, 5, 800).unwrap()
        };
        rename_session(&state, id, Some("Practice".into())).unwrap();
        set_session_bookmark(&state, id, true).unwrap();
        let rows = list_sessions(&state).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name.as_deref(), Some("Practice"));
        assert!(rows[0].bookmarked);
        delete_session(&state, id).unwrap();
        assert_eq!(list_sessions(&state).unwrap().len(), 0);
    }

    #[test]
    fn clear_all_removes_everything() {
        let state = test_state();
        {
            let conn = state.db.lock().unwrap();
            db::open_session(&conn, 1, 1, 1, 1).unwrap();
            db::open_session(&conn, 2, 2, 2, 2).unwrap();
        }
        clear_all_sessions(&state).unwrap();
        assert_eq!(list_sessions(&state).unwrap().len(), 0);
    }

    #[test]
    fn get_settings_returns_defaults() {
        let state = test_state();
        assert_eq!(get_settings(&state).port, 20440);
    }
}
