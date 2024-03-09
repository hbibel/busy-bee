use std::{
    error::Error,
    fmt::{Display, Write},
};

use chrono::{Datelike, Local, NaiveDate, Timelike};

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
        let time_str = event.dt.format("%H:%M");
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
    let (mut hours, mut minutes, complete, _) = events.iter().fold(
        (0, 0, true, None),
        |(hours, minutes, complete, maybe_previous), event| match (
            maybe_previous,
            event,
        ) {
            (
                None,
                Event {
                    kind: EventKind::ClockIn,
                    dt: _,
                },
            ) => (hours, minutes, complete, Some(event)),
            (
                None,
                Event {
                    kind: EventKind::ClockOut,
                    dt: _,
                },
            ) => (hours, minutes, false, None),
            (
                Some(_),
                Event {
                    kind: EventKind::ClockIn,
                    dt: _,
                },
            ) => (hours, minutes, false, Some(event)),
            (
                Some(prev),
                Event {
                    kind: EventKind::ClockOut,
                    dt,
                },
            ) => (
                hours + dt.hour() - prev.dt.hour(),
                minutes + dt.minute() - prev.dt.minute(),
                complete,
                None,
            ),
        },
    );

    hours += minutes / 60;
    minutes %= 60;
    WorkingTime {
        hours,
        minutes,
        complete,
    }
}
