use std::{
    error::Error,
    fmt::Display,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use tempfile::NamedTempFile;

#[derive(Debug)]
pub enum PersistenceError {
    EventNotFoundError { id: u32 },
    InvalidFormatError,
    InvalidDataError { detail: String },
    IoError { err: io::Error },
}

impl Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Persistence error") // TODO
    }
}

impl Error for PersistenceError {}

impl From<io::Error> for PersistenceError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError { err }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum EventKind {
    ClockIn,
    ClockOut,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Event {
    pub kind: EventKind,
    pub dt: DateTime<Utc>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct StoredEvent {
    pub id: u32,
    pub event: Event,
}

impl Event {
    pub fn clock_in<Tz: TimeZone>(dt: &DateTime<Tz>) -> Event {
        Self {
            kind: EventKind::ClockIn,
            dt: dt.to_utc(),
        }
    }

    pub fn clock_out<Tz: TimeZone>(dt: &DateTime<Tz>) -> Event {
        Self {
            kind: EventKind::ClockOut,
            dt: dt.to_utc(),
        }
    }
}

pub fn create_event(storage_dir: &Path, event: &Event) -> Result<()> {
    let mut events = read_events(storage_dir, event.dt.date_naive())
        .with_context(|| {
            let sd = storage_dir.display();
            format!("Could not read events from storage directory {sd}")
        })?;
    events.push(event.clone());
    events.sort_by_key(|event| event.dt);

    let events_as_str: String = events
        .iter()
        .map(event_to_str)
        .collect::<Vec<_>>()
        .join("\n");

    let file_name = get_file_name(&event.dt);
    let file_path = storage_dir.join(file_name);

    write_to_file(&file_path, &events_as_str).with_context(|| {
        let fd = file_path.display();
        format!("Could not write events to file {fd}")
    })
}

pub fn read_events(storage_dir: &Path, date: NaiveDate) -> Result<Vec<Event>> {
    let file_name = get_file_name(&date);
    let file_path = storage_dir.join(file_name);

    let mut file_content = String::new();

    if !file_path.is_file() {
        // This could also mean that the file is not readable by the current
        // user
        return Ok(Vec::new());
    }

    let _ = File::open(file_path)?.read_to_string(&mut file_content)?;
    file_content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_event)
        .collect()
}

fn parse_event(line: &str) -> Result<Event> {
    let cols: Vec<_> = line.split(',').map(str::trim).collect();
    if cols.len() != 2 {
        bail!("Misformatted line: {line}")
    }

    // allowing [0] because we previously asserted that this element exists
    #[allow(clippy::match_on_vec_items)]
    let kind = match cols[0] {
        "clock-in" => Ok(EventKind::ClockIn),
        "clock-out" => Ok(EventKind::ClockOut),
        other => Err(PersistenceError::InvalidDataError {
            detail: format!("Unknown event kind {other}"),
        }),
    }?;

    let date_str = cols[1];
    let dt = DateTime::parse_from_rfc3339(date_str)
        .map_err(|err| PersistenceError::InvalidDataError {
            detail: format!("Could not parse {date_str} as datetime: {err}"),
        })?
        .with_timezone(&Utc);
    Ok(Event { kind, dt })
}

fn event_to_str(event: &Event) -> String {
    let kind_str = match event.kind {
        EventKind::ClockIn => "clock-in",
        EventKind::ClockOut => "clock-out",
    };
    let date_str = event.dt.to_rfc3339();

    format!("{kind_str},{date_str}")
}

pub fn delete_event(
    storage_dir: &Path,
    date: NaiveDate,
    id: u32,
) -> Result<()> {
    let events = read_events(storage_dir, date)?;
    #[allow(clippy::cast_possible_truncation)]
    let events: Vec<&Event> = events
        .iter()
        .enumerate()
        .filter(|(event_id, _)| *event_id as u32 != id)
        .map(|(_, event)| event)
        .collect();

    let events_as_str: String =
        events.iter().map(|event| event_to_str(event)).collect();

    let file_name = get_file_name(&date);
    let file_path = storage_dir.join(file_name);

    write_to_file(&file_path, &events_as_str)
}

fn get_file_name<T: Datelike>(has_date: &T) -> String {
    format!(
        "{}-{:0>2}-{:0>2}.csv",
        has_date.year(),
        has_date.month(),
        has_date.day()
    )
}

fn write_to_file(file_path: &Path, content: &str) -> Result<()> {
    // atomic write, by writing to a temp file first then rename
    let mut tmp_file = NamedTempFile::new()?;
    tmp_file.write_all(content.as_bytes())?;

    fs::rename(tmp_file, file_path)?;

    // Sync file in order to minimize the risk of data loss. There's an
    // interesting discussion here:
    // https://github.com/Stebalien/tempfile/issues/110
    Ok(OpenOptions::new().write(true).open(file_path)?.sync_all()?)
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use chrono::Local;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn get_file_name_pads_month_and_day() {
        let date = NaiveDate::from_ymd_opt(2022, 1, 2).unwrap();
        assert_eq!(get_file_name(&date), "2022-01-02.csv");
    }

    #[test]
    fn create_read_delete_events() {
        // happy paths
        let d = tempdir().unwrap();
        let dir = d.path();
        let event1 = Event {
            kind: EventKind::ClockIn,
            dt: Local::now().to_utc(),
        };
        create_event(dir, &event1).unwrap();

        let expected_events = vec![event1.clone()];
        assert_eq!(
            read_events(dir, Local::now().date_naive()).unwrap(),
            expected_events
        );

        let event2 = Event {
            kind: EventKind::ClockOut,
            dt: Local::now().to_utc(),
        };
        create_event(dir, &event2).unwrap();

        let expected_events = vec![event1.clone(), event2.clone()];
        assert_eq!(
            read_events(dir, Local::now().date_naive()).unwrap(),
            expected_events
        );

        delete_event(dir, Local::now().date_naive(), 0).unwrap();

        let expected_events = vec![event2.clone()];
        assert_eq!(
            read_events(dir, Local::now().date_naive()).unwrap(),
            expected_events
        );
    }

    #[test]
    fn read_returns_events() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 31).unwrap();

        let d = tempdir().unwrap();
        let dir = d.path();
        let file_path = d.path().join(get_file_name(&date));

        let file_content = "clock-in,2020-01-31T08:15:00Z\n\
            clock-out,2020-01-31T16:15:00Z\n";
        File::create(file_path)
            .unwrap()
            .write_all(file_content.as_bytes())
            .unwrap();

        let actual = read_events(dir, date);
        let expected = vec![
            Event {
                kind: EventKind::ClockIn,
                dt: Utc.with_ymd_and_hms(2020, 1, 31, 8, 15, 0).unwrap(),
            },
            Event {
                kind: EventKind::ClockOut,
                dt: Utc.with_ymd_and_hms(2020, 1, 31, 16, 15, 0).unwrap(),
            },
        ];
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn read_returns_empty_list_if_file_does_not_exist() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 31).unwrap();

        let d = tempdir().unwrap();
        let dir = d.path();

        let actual = read_events(dir, date).unwrap();
        assert!(actual.is_empty());
    }

    #[test]
    fn read_returns_empty_list_if_file_is_empty() {
        let date = NaiveDate::from_ymd_opt(2020, 1, 31).unwrap();

        let d = tempdir().unwrap();
        let dir = d.path();
        let file_path = d.path().join("2020-01-31.txt");

        let file_content = "\n";
        File::create(file_path)
            .unwrap()
            .write_all(file_content.as_bytes())
            .unwrap();

        let actual = read_events(dir, date).unwrap();
        assert!(actual.is_empty());
    }
}
