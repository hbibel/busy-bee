use anyhow::{anyhow, Result};
use busy_bee::{
    cli::{Cli, Commands},
    data::{create_event, delete_event, read_events, Event},
    view::{daily_report, monthly_report, weekly_report},
};
use chrono::{
    DateTime, Datelike, Days, Local, NaiveDate, NaiveTime, TimeZone, Timelike,
    Utc,
};
use clap::Parser;
use directories::ProjectDirs;

fn main() {
    let args = Cli::parse();

    let storage_dir = args.storage_dir.unwrap_or_else(|| {
        let default_dir = ProjectDirs::from("", "", "busy-bee")
            .map(|pd| pd.data_local_dir().to_path_buf());
        default_dir.expect(
            "Could not determine the local data directory for your OS. Please \
            use the '--storage-dir' flag to specify where any local data \
            should be saved.",
        )
    });
    if !storage_dir.exists() {
        std::fs::create_dir(&storage_dir).unwrap();
    }

    match args.command {
        Commands::ClockIn { date, time } => {
            let dt = get_date_time(date, time).unwrap();
            let event = Event::clock_in(&dt);
            let events = create_event(&storage_dir, &event).unwrap();
            let report = daily_report(&dt.date_naive(), &events).unwrap();
            println!("{report}");
        }
        Commands::ClockOut { date, time } => {
            let dt = get_date_time(date, time).unwrap();
            let event = Event::clock_out(&dt);
            let events = create_event(&storage_dir, &event).unwrap();
            let report = daily_report(&dt.date_naive(), &events).unwrap();
            println!("{report}");
        }
        Commands::Delete { date, id } => {
            let date = match date {
                Some(d) => d,
                None => Local::now().date_naive(),
            };
            let events = delete_event(&storage_dir, &date, id).unwrap();
            let report = daily_report(&date, &events).unwrap();
            println!("{report}");
        }
        Commands::View { date } => {
            let events = read_events(&storage_dir, &date).unwrap();
            let report = daily_report(&date, &events).unwrap();
            println!("{report}");
        }
        Commands::WeeklyReport { date } => {
            let reference_day =
                date.unwrap_or_else(|| Local::now().date_naive());
            let monday = reference_day
                .checked_sub_days(Days::new(
                    reference_day.weekday().num_days_from_monday().into(),
                ))
                .unwrap();
            let days = std::iter::successors(Some(monday), |day| {
                let next_day = day.checked_add_days(Days::new(1)).unwrap();
                if next_day <= reference_day {
                    Some(next_day)
                } else {
                    None
                }
            });
            let mut events = Vec::new();
            days.for_each(|date| {
                events.extend(read_events(&storage_dir, &date).unwrap());
            });

            let report = weekly_report(&monday, &events).unwrap();
            println!("{report}");
        }
        Commands::Report { date } => {
            let reference_day =
                date.unwrap_or_else(|| Local::now().date_naive());
            let first_of_month = reference_day.with_day(1).unwrap();
            // iterator over all days in the month
            let days =
                std::iter::successors(Some(first_of_month), move |day| {
                    day.checked_add_days(Days::new(1))
                        .filter(|d| d.month0() == first_of_month.month0())
                });
            let mut events = Vec::new();
            days.for_each(|date| {
                events.extend(read_events(&storage_dir, &date).unwrap());
            });

            let report = monthly_report(&reference_day, &events).unwrap();
            println!("{report}");
        }
    };
}

fn get_date_time(
    maybe_date: Option<NaiveDate>,
    maybe_time: Option<NaiveTime>,
) -> Result<DateTime<Utc>> {
    match (maybe_date, maybe_time) {
        (Some(date), Some(time)) => {
            let naive_dt = date.and_time(time);
            Local
                .from_local_datetime(&naive_dt)
                .single()
                .ok_or_else(|| {
                    anyhow!(
                    "{} cannot be converted to an unambiguous point in time",
                    naive_dt
                )
                })
                .map(|dt| dt.to_utc())
        }
        (None, Some(time)) => Ok(Local::now())
            .and_then(|t| {
                t.with_hour(time.hour())
                    .ok_or(anyhow!("Cannot use {} as hour", time.hour()))
            })
            .and_then(|t| {
                t.with_minute(time.minute())
                    .ok_or(anyhow!("Cannot use {} as minute", time.minute()))
            })
            .map(|t| t.with_timezone(&Utc)),
        (Some(_), None) => Err(anyhow!("Date specified, but no time")),
        (None, None) => Ok(Local::now().to_utc()),
    }
}
