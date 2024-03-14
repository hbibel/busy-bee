use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Display, Write},
    ops::Sub,
};

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};

use crate::data::{Event, EventKind};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct ViewError {
    detail: String,
}

impl Display for ViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Could not generate view, cause:\n{}", self.detail)
    }
}

impl<T: Error> From<T> for ViewError {
    fn from(value: T) -> Self {
        ViewError {
            detail: format!("{value}"),
        }
    }
}

pub fn daily_report(
    date: &NaiveDate,
    events: &[Event],
) -> Result<String, ViewError> {
    let mut result = String::new();

    write!(result, "Records for ")?;
    let today = Local::now().date_naive();
    if same_date(date, &today) {
        write!(result, "today, ")?;
    }
    writeln!(result, "{}:", date.format("%b %d, %Y"))?;

    for (i, event) in events.iter().enumerate() {
        let local_time: DateTime<Local> = DateTime::from(event.dt);
        let time_str = local_time.format("%H:%M");
        let kind_str = match event.kind {
            EventKind::ClockIn => "clock in ",
            EventKind::ClockOut => "clock out",
        };
        writeln!(result, "{i} | {time_str} | {kind_str} |")?;
    }

    let WorkingTime {
        hours,
        minutes,
        complete,
    } = working_time(events);
    writeln!(result, "Total working time: {hours:02}:{minutes:02} hours")?;
    if !complete {
        writeln!(result, "Incomplete records, please update")?;
    }
    Ok(result)
}

pub fn monthly_report(
    date: &NaiveDate,
    events: &[Event],
) -> Result<String, ViewError> {
    let mut result = String::new();

    writeln!(result, "Summary for {}:", date.format("%B %Y"))?;

    // using BTreeMap for its sorted keys
    let mut events_per_day = BTreeMap::new();
    for event in events {
        let days_events = events_per_day
            .entry(event.dt.day())
            .or_insert_with(Vec::new);
        days_events.push(event.clone());
    }

    for (day, days_events) in events_per_day {
        let WorkingTime {
            hours,
            minutes,
            complete,
        } = working_time(&days_events);
        let mut comment = "";
        if !complete {
            comment = "Incomplete records, please update";
        }

        let recorded_time = if complete {
            format!("{hours:02}:{minutes:02}")
        } else {
            "?".to_string()
        };
        writeln!(result, "{day:<2} | {recorded_time:<5} | {comment}")?;
    }

    let WorkingTime {
        hours,
        minutes,
        complete: _,
    } = working_time(events);
    writeln!(result, "Total working time: {hours:02}:{minutes:02} hours")?;
    // TODO compute overtime
    Ok(result)
}

fn same_date<T: Datelike, U: Datelike>(date1: &T, date2: &U) -> bool {
    date1.day() == date2.day()
        && date1.month() == date2.month()
        && date1.year() == date2.year()
}

struct WorkingTime {
    hours: u32,
    minutes: u32,
    complete: bool,
}

fn working_time(events: &[Event]) -> WorkingTime {
    let (worked, complete, _) = events.iter().fold(
        (Duration::new(0, 0).unwrap(), true, None),
        |(duration, complete, maybe_previous), event| match (
            maybe_previous,
            event,
        ) {
            (
                None,
                Event {
                    kind: EventKind::ClockIn,
                    dt: _,
                },
            ) => (duration, complete, Some(event)),
            (
                None,
                Event {
                    kind: EventKind::ClockOut,
                    dt: _,
                },
            ) => (duration, false, None),
            (
                Some(_),
                Event {
                    kind: EventKind::ClockIn,
                    dt: _,
                },
            ) => (duration, false, Some(event)),
            (
                Some(prev),
                Event {
                    kind: EventKind::ClockOut,
                    dt,
                },
            ) => (duration + dt.sub(prev.dt), complete, None),
        },
    );

    let hours: u32 = worked.num_hours().try_into().unwrap();
    let minutes: u32 = (worked.num_minutes() % 60).try_into().unwrap();
    WorkingTime {
        hours,
        minutes,
        complete,
    }
}
