use rusqlite::{Connection, Result, params};
use serde::Serialize;

// -- Types --

#[derive(Debug, Serialize, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub text: String,
    pub word_count: i32,
    pub duration_ms: i64,
    pub wpm: f64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct VocabularyEntry {
    pub id: i64,
    pub term: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Stats {
    pub avg_wpm: f64,
    pub words_this_week: i64,
    pub time_saved_minutes: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CorrectionEntry {
    pub id: i64,
    pub from_text: String,
    pub to_text: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ChecklistStep {
    pub step_id: String,
    pub completed: bool,
    pub completed_at: Option<String>,
}

// -- Database functions --

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            text        TEXT NOT NULL,
            word_count  INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            wpm         REAL NOT NULL,
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS vocabulary (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            term TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS checklist (
            step_id      TEXT PRIMARY KEY,
            completed    BOOLEAN DEFAULT 0,
            completed_at DATETIME
        );

        INSERT OR IGNORE INTO settings (key, value) VALUES
            ('hotkey', ''),
            ('api_url', 'http://172.16.1.222:8028/v1'),
            ('api_key', 'cant-be-empty'),
            ('model_id', 'deepdml/faster-whisper-large-v3-turbo-ct2'),
            ('mic_device', 'default'),
            ('typing_speed_wpm', '40');

        CREATE TABLE IF NOT EXISTS corrections (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            from_text TEXT NOT NULL,
            to_text   TEXT NOT NULL
        );

        INSERT OR IGNORE INTO checklist (step_id) VALUES
            ('start_recording'),
            ('customize_shortcuts'),
            ('add_vocabulary'),
            ('configure_api');

        UPDATE settings SET value = 'AltRight'    WHERE key = 'hotkey' AND value = 'RAlt';
        UPDATE settings SET value = 'AltLeft'     WHERE key = 'hotkey' AND value = 'LAlt';
        UPDATE settings SET value = 'ControlRight' WHERE key = 'hotkey' AND value = 'RControl';
        UPDATE settings SET value = 'ControlLeft'  WHERE key = 'hotkey' AND value = 'LControl';
        "
    )
}

pub fn insert_history(conn: &Connection, text: &str, word_count: i32, duration_ms: i64, wpm: f64) -> Result<()> {
    conn.execute(
        "INSERT INTO history (text, word_count, duration_ms, wpm) VALUES (?1, ?2, ?3, ?4)",
        params![text, word_count, duration_ms, wpm],
    )?;
    conn.execute(
        "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY created_at DESC LIMIT 500)",
        [],
    )?;
    Ok(())
}

pub fn get_history(conn: &Connection, limit: i32) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, word_count, duration_ms, wpm, created_at FROM history ORDER BY created_at DESC, id DESC LIMIT ?1"
    )?;
    let entries = stmt.query_map(params![limit], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            text: row.get(1)?,
            word_count: row.get(2)?,
            duration_ms: row.get(3)?,
            wpm: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_stats(conn: &Connection) -> Result<Stats> {
    let avg_wpm: f64 = conn
        .query_row("SELECT COALESCE(AVG(wpm), 0.0) FROM history", [], |row| row.get(0))?;

    let words_this_week: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM history WHERE created_at >= date('now', 'weekday 0', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    let time_saved_minutes: f64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count / 40.0 - duration_ms / 60000.0), 0.0) FROM history WHERE created_at >= date('now', 'weekday 0', '-7 days')",
        [],
        |row| row.get(0),
    )?;

    Ok(Stats {
        avg_wpm,
        words_this_week,
        time_saved_minutes,
    })
}

pub fn add_vocabulary(conn: &Connection, term: &str) -> Result<()> {
    conn.execute("INSERT INTO vocabulary (term) VALUES (?1)", params![term])?;
    Ok(())
}

pub fn remove_vocabulary(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM vocabulary WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn get_vocabulary(conn: &Connection) -> Result<Vec<VocabularyEntry>> {
    let mut stmt = conn.prepare("SELECT id, term FROM vocabulary ORDER BY id")?;
    let entries = stmt.query_map([], |row| {
        Ok(VocabularyEntry {
            id: row.get(0)?,
            term: row.get(1)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<String> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", params![key], |row| row.get(0))
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
    let entries = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_checklist(conn: &Connection) -> Result<Vec<ChecklistStep>> {
    let mut stmt = conn.prepare("SELECT step_id, completed, completed_at FROM checklist ORDER BY rowid")?;
    let entries = stmt.query_map([], |row| {
        Ok(ChecklistStep {
            step_id: row.get(0)?,
            completed: row.get(1)?,
            completed_at: row.get(2)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn get_corrections(conn: &Connection) -> Result<Vec<CorrectionEntry>> {
    let mut stmt = conn.prepare("SELECT id, from_text, to_text FROM corrections ORDER BY id")?;
    let entries = stmt.query_map([], |row| {
        Ok(CorrectionEntry {
            id: row.get(0)?,
            from_text: row.get(1)?,
            to_text: row.get(2)?,
        })
    })?.filter_map(|r| r.ok()).collect();
    Ok(entries)
}

pub fn add_correction(conn: &Connection, from_text: &str, to_text: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO corrections (from_text, to_text) VALUES (?1, ?2)",
        params![from_text, to_text],
    )?;
    Ok(())
}

pub fn remove_correction(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM corrections WHERE id = ?1", params![id])?;
    Ok(())
}

/// Apply all corrections to a transcription result (case-insensitive matching).
pub fn apply_corrections(text: &str, corrections: &[CorrectionEntry]) -> String {
    let mut result = text.to_string();
    for c in corrections {
        if c.from_text.is_empty() {
            continue;
        }
        let lower_result = result.to_lowercase();
        let lower_from = c.from_text.to_lowercase();
        let mut output = String::new();
        let mut search_start = 0;
        while let Some(pos) = lower_result[search_start..].find(&lower_from) {
            let abs_pos = search_start + pos;
            output.push_str(&result[search_start..abs_pos]);
            output.push_str(&c.to_text);
            search_start = abs_pos + c.from_text.len();
        }
        output.push_str(&result[search_start..]);
        result = output;
    }
    result
}

pub fn complete_checklist_step(conn: &Connection, step_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE checklist SET completed = 1, completed_at = CURRENT_TIMESTAMP WHERE step_id = ?1",
        params![step_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_init_creates_tables() {
        let conn = setup_db();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(tables.contains(&"history".to_string()));
        assert!(tables.contains(&"vocabulary".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"checklist".to_string()));
    }

    #[test]
    fn test_default_settings_seeded() {
        let conn = setup_db();
        assert_eq!(get_setting(&conn, "hotkey").unwrap(), "");
        assert_eq!(get_setting(&conn, "api_url").unwrap(), "http://172.16.1.222:8028/v1");
        assert_eq!(get_setting(&conn, "api_key").unwrap(), "cant-be-empty");
        assert_eq!(get_setting(&conn, "model_id").unwrap(), "deepdml/faster-whisper-large-v3-turbo-ct2");
        assert_eq!(get_setting(&conn, "mic_device").unwrap(), "default");
        assert_eq!(get_setting(&conn, "typing_speed_wpm").unwrap(), "40");
    }

    #[test]
    fn test_insert_and_get_history() {
        let conn = setup_db();
        insert_history(&conn, "hello world", 2, 5000, 24.0).unwrap();
        insert_history(&conn, "second entry", 2, 3000, 40.0).unwrap();
        let entries = get_history(&conn, 10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "second entry");
        assert_eq!(entries[1].text, "hello world");
    }

    #[test]
    fn test_history_rolling_cleanup() {
        let conn = setup_db();
        for i in 0..510 {
            insert_history(&conn, &format!("entry {}", i), 1, 1000, 60.0).unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .unwrap();
        assert!(count <= 500, "History should not exceed 500 entries, got {}", count);
    }

    #[test]
    fn test_vocabulary_crud() {
        let conn = setup_db();
        add_vocabulary(&conn, "Kubernetes").unwrap();
        add_vocabulary(&conn, "Tauri").unwrap();
        let terms = get_vocabulary(&conn).unwrap();
        assert_eq!(terms.len(), 2);
        assert_eq!(terms[0].term, "Kubernetes");
        remove_vocabulary(&conn, terms[0].id).unwrap();
        let terms = get_vocabulary(&conn).unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "Tauri");
    }

    #[test]
    fn test_vocabulary_duplicate_rejected() {
        let conn = setup_db();
        add_vocabulary(&conn, "Kubernetes").unwrap();
        let result = add_vocabulary(&conn, "Kubernetes");
        assert!(result.is_err());
    }

    #[test]
    fn test_settings_update() {
        let conn = setup_db();
        set_setting(&conn, "hotkey", "LCtrl").unwrap();
        assert_eq!(get_setting(&conn, "hotkey").unwrap(), "LCtrl");
    }

    #[test]
    fn test_get_all_settings() {
        let conn = setup_db();
        let settings = get_all_settings(&conn).unwrap();
        assert_eq!(settings.len(), 6);
    }

    #[test]
    fn test_stats_empty_history() {
        let conn = setup_db();
        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.avg_wpm, 0.0);
        assert_eq!(stats.words_this_week, 0);
        assert_eq!(stats.time_saved_minutes, 0.0);
    }

    #[test]
    fn test_stats_with_data() {
        let conn = setup_db();
        insert_history(&conn, "one two three", 3, 3000, 60.0).unwrap();
        insert_history(&conn, "four five six seven", 4, 4000, 60.0).unwrap();
        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.avg_wpm, 60.0);
        assert_eq!(stats.words_this_week, 7);
        assert!(stats.time_saved_minutes > 0.0);
    }

    #[test]
    fn test_checklist_default_steps() {
        let conn = setup_db();
        let steps = get_checklist(&conn).unwrap();
        assert_eq!(steps.len(), 4);
        assert!(!steps[0].completed);
    }

    #[test]
    fn test_complete_checklist_step() {
        let conn = setup_db();
        complete_checklist_step(&conn, "start_recording").unwrap();
        let steps = get_checklist(&conn).unwrap();
        let step = steps.iter().find(|s| s.step_id == "start_recording").unwrap();
        assert!(step.completed);
        assert!(step.completed_at.is_some());
    }
}
