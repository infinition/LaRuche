use crate::abeille::{Abeille, ContextExecution, NiveauDanger, ResultatAbeille};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const CALENDAR_FILE: &str = "essaim-calendar.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalendarEntry {
    id: String,
    title: String,
    date: String,
    time: Option<String>,
    description: Option<String>,
    done: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CalendarData {
    entries: Vec<CalendarEntry>,
}

fn calendar_path() -> PathBuf {
    PathBuf::from(CALENDAR_FILE)
}

fn load_calendar() -> CalendarData {
    let path = calendar_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        CalendarData::default()
    }
}

fn save_calendar(data: &CalendarData) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    std::fs::write(calendar_path(), json)?;
    Ok(())
}

/// Add an event/reminder to the calendar.
pub struct CalendarAdd;

#[async_trait]
impl Abeille for CalendarAdd {
    fn nom(&self) -> &str { "calendar_add" }
    fn description(&self) -> &str {
        "Add an event or reminder to the calendar. Specify title, date (YYYY-MM-DD), optional time (HH:MM), and optional description."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "Event title" },
                "date": { "type": "string", "description": "Date in YYYY-MM-DD format" },
                "time": { "type": "string", "description": "Optional time in HH:MM format" },
                "description": { "type": "string", "description": "Optional description" }
            },
            "required": ["title", "date"]
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }

    async fn executer(&self, args: serde_json::Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let title = args["title"].as_str().unwrap_or("Untitled").to_string();
        let date = args["date"].as_str().unwrap_or("").to_string();
        let time = args["time"].as_str().map(|s| s.to_string());
        let description = args["description"].as_str().map(|s| s.to_string());

        if date.is_empty() {
            return Ok(ResultatAbeille::err("Missing 'date' argument"));
        }

        let mut cal = load_calendar();
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        cal.entries.push(CalendarEntry {
            id: id.clone(),
            title: title.clone(),
            date: date.clone(),
            time: time.clone(),
            description,
            done: false,
        });
        save_calendar(&cal)?;

        let time_str = time.map(|t| format!(" at {}", t)).unwrap_or_default();
        Ok(ResultatAbeille::ok(format!(
            "Event added: '{}' on {}{} (id: {})",
            title, date, time_str, id
        )))
    }
}

/// List calendar events.
pub struct CalendarList;

#[async_trait]
impl Abeille for CalendarList {
    fn nom(&self) -> &str { "calendar_list" }
    fn description(&self) -> &str {
        "List upcoming calendar events and reminders. Optionally filter by date."
    }
    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "date": { "type": "string", "description": "Optional: filter by date (YYYY-MM-DD)" }
            },
            "required": []
        })
    }
    fn niveau_danger(&self) -> NiveauDanger { NiveauDanger::Safe }

    async fn executer(&self, args: serde_json::Value, _ctx: &ContextExecution) -> Result<ResultatAbeille> {
        let cal = load_calendar();
        let filter_date = args["date"].as_str();

        let entries: Vec<&CalendarEntry> = cal.entries.iter()
            .filter(|e| {
                if let Some(d) = filter_date {
                    e.date == d
                } else {
                    true
                }
            })
            .collect();

        if entries.is_empty() {
            return Ok(ResultatAbeille::ok("No events found."));
        }

        let mut output = format!("{} event(s):\n\n", entries.len());
        for e in &entries {
            let time_str = e.time.as_deref().map(|t| format!(" {}", t)).unwrap_or_default();
            let status = if e.done { " [DONE]" } else { "" };
            let desc = e.description.as_deref().map(|d| format!("\n  {}", d)).unwrap_or_default();
            output.push_str(&format!(
                "- [{}] {}{}: {}{}{}\n",
                e.id, e.date, time_str, e.title, status, desc
            ));
        }

        Ok(ResultatAbeille::ok(output))
    }
}
