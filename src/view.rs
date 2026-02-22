use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{Display, Write},
    ops::Sub,
};

use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, TimeDelta, Weekday,
};

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
    writeln!(result, "{}", report_days(events)?)?;

    // TODO compute overtime
    Ok(result)
}

pub fn weekly_report(
    date: &NaiveDate,
    events: &[Event],
) -> Result<String, ViewError> {
    let mut result = String::new();

    let calendar_week = {
        // ISO 8601 defines year's week 1 to be the week containing the year's
        // first Thursday. So if it's Friday, Jan 1 then the day actually
        // belongs to last year's weeks by that definition.
        let year = if date.month() == 1
            && date.weekday().number_from_monday() - date.day() > 3
        {
            date.year() - 1
        } else {
            date.year()
        };
        let jan1 = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let week_1_monday_offset = match jan1.weekday() {
            Weekday::Mon => 0,
            Weekday::Tue => -1,
            Weekday::Wed => -2,
            Weekday::Thu => -3,
            Weekday::Fri => 3,
            Weekday::Sat => 2,
            Weekday::Sun => 1,
        };
        let week_1_monday = jan1
            .checked_add_signed(TimeDelta::days(week_1_monday_offset))
            .unwrap();
        date.signed_duration_since(week_1_monday).num_days() / 7 + 1
    };
    writeln!(
        result,
        "Summary for calendar week {calendar_week}, starting {}:",
        date.format("%Y-%m-%d")
    )?;

    writeln!(result, "{}", report_days(events)?)?;

    Ok(result)
}

fn report_days(events: &[Event]) -> Result<String, ViewError> {
    let mut result = String::new();

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
