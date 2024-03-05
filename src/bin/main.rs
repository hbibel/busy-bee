use anyhow::{anyhow, Result};
use busy_bee::{
    cli::{Cli, Commands},
    data::{create_event, delete_event, Event},
};
use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Timelike, Utc};
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

    match args.command {
        Commands::ClockIn { date, time } => {
            let dt = get_date_time(date, time).unwrap();
            let event = Event::clock_in(&dt);
            create_event(&storage_dir, &event).unwrap();
        }
        Commands::ClockOut { date, time } => {
            let dt = get_date_time(date, time).unwrap();
            let event = Event::clock_out(&dt);
            create_event(&storage_dir, &event).unwrap();
        }
        Commands::Delete { date, id } => {
            let date = match date {
                Some(d) => d,
                None => Local::now().date_naive(),
            };
            delete_event(&storage_dir, date, id).unwrap();
        }
        _ => todo!(),
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
