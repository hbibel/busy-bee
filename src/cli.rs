use std::path::PathBuf;

use chrono::{Datelike, Days};
use chrono::{Local, NaiveDate, NaiveTime};
use clap::{Parser, Subcommand};
use regex::Regex;

/// A small tool to maintain a log of working times
#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Where this application should store its data. Defaults to an operating
    /// system specific convention.
    #[arg(long, short)]
    pub storage_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Record when you started working or came back from a break
    ClockIn {
        /// Specify the date, default is today
        #[arg(value_parser=parse_date, long, short)]
        date: Option<NaiveDate>,
        /// Specify the time, default is now
        #[arg(value_parser=parse_time)]
        time: Option<NaiveTime>,
    },
    /// Record when you took a break or stopped working
    ClockOut {
        /// Specify the date, default is today
        #[arg(value_parser=parse_date, long, short)]
        date: Option<NaiveDate>,
        /// Specify the time, default is now
        #[arg(value_parser=parse_time)]
        time: Option<NaiveTime>,
    },
    /// View log entries for a specific day
    View {
        #[arg(value_parser=parse_date)]
        date: NaiveDate,
    },
    /// Delete a previously recorded log entry
    Delete {
        /// Date of the event to delete, default is today
        #[arg(value_parser=parse_date, long, short)]
        date: Option<NaiveDate>,
        /// Event ID to delete
        id: u32,
    },
    /// View a monthly summary of recorded times
    Report {
        /// Month to view recorded times for
        #[arg(value_parser=parse_month)]
        date: Option<NaiveDate>,
    },
}

fn parse_time(user_input: &str) -> Result<NaiveTime, String> {
    if user_input == "now" {
        return Ok(Local::now().naive_local().time());
    }

    let re = Regex::new(r"^(\d{1,2}):?(\d{2})$").unwrap();
    let captures = re.captures(user_input).ok_or(format!(
        "Unknown time format: '{user_input}'; try e.g. 730, 0730, 07:30"
    ))?;
    let (hour, minute) = (&captures[1], &captures[2]);
    // Can just unwrap() the parse results, because the regex ensures that
    // we're dealing with numeric characters only
    NaiveTime::from_hms_opt(hour.parse().unwrap(), minute.parse().unwrap(), 0)
        .ok_or(format!("{hour}:{minute} is not a valid time"))
}

fn parse_date(user_input: &str) -> Result<NaiveDate, String> {
    if user_input == "today" {
        return Ok(Local::now().naive_local().date());
    }
    if user_input == "yesterday" {
        return Ok((Local::now().naive_local() - Days::new(1)).date());
    }

    let re = Regex::new(r"^(\d{2,4})-?(\d{2})-?(\d{2})$").unwrap();
    let captures = re.captures(user_input).ok_or(format!(
        "Unknown date format: '{user_input}'; \
        try e.g. 2024-01-31, 20240131, 240131"
    ))?;

    // Can just unwrap() the parse results, because the regex ensures that
    // we're dealing with numeric characters only
    let mut year = captures[1].parse::<i32>().unwrap();
    // TODO: Hack; fix within the next 975 years
    if year < 2000 {
        year += 2000;
    }
    let month = captures[2].parse::<u32>().unwrap();
    let day = captures[3].parse::<u32>().unwrap();
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or(format!("{year}-{month}-{day} is not a valid date"))
}

pub fn parse_month(user_input: &str) -> Result<NaiveDate, String> {
    let parts: Vec<_> =
        user_input.splitn(2, |c| c == '/' || c == ' ').collect();
    let month = parts
        .first()
        .ok_or("Empty input for month".to_string())
        .and_then(|s| month_from_str(s))?;
    let mut year = parts.get(1).map_or_else(
        || Ok(Local::now().year()),
        |s| s.parse().map_err(|e| format!("{e}")),
    )?;
    if year < 2000 {
        year += 2000;
    }
    NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or(format!("Invalid month: {month}"))
}

fn month_from_str(s: &str) -> Result<u32, String> {
    if s.chars().all(|c| c.is_ascii_digit()) {
        s.parse().map_err(|e| format!("{e}"))
    } else {
        match s.to_ascii_lowercase().as_str() {
            "jan" | "january" => Ok(1),
            "feb" | "february" => Ok(2),
            "mar" | "march" => Ok(3),
            "apr" | "april" => Ok(4),
            "may" => Ok(5),
            "jun" | "june" => Ok(6),
            "jul" | "july" => Ok(7),
            "aug" | "august" => Ok(8),
            "sep" | "september" => Ok(9),
            "oct" | "october" => Ok(10),
            "nov" | "november" => Ok(11),
            "dec" | "december" => Ok(12),
            _ => Err(format!(
                "Invalid month specifier {s}, try e.g., '1' or 'Jan'"
            )),
        }
    }
}

#[cfg(test)]
mod tests {

    use chrono::Datelike;

    use super::*;

    #[test]
    fn test_parse_time_730() {
        let expected = NaiveTime::from_hms_opt(7, 30, 0).unwrap();
        assert_eq!(parse_time("730"), Ok(expected));
    }

    #[test]
    fn test_parse_time_1730() {
        let expected = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        assert_eq!(parse_time("1730"), Ok(expected));
    }

    #[test]
    fn test_parse_time_7_30() {
        let expected = NaiveTime::from_hms_opt(7, 30, 0).unwrap();
        assert_eq!(parse_time("7:30"), Ok(expected));
    }

    #[test]
    fn test_parse_time_17_30() {
        let expected = NaiveTime::from_hms_opt(17, 30, 0).unwrap();
        assert_eq!(parse_time("17:30"), Ok(expected));
    }

    #[test]
    fn test_parse_date_yesterday() {
        let yesterday = Local::now().naive_local() - Days::new(1);
        let yesterday = yesterday.date();
        assert_eq!(parse_date("yesterday"), Ok(yesterday));
    }

    #[test]
    fn test_parse_date_yyyymmdd() {
        let expected = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        assert_eq!(parse_date("20240113"), Ok(expected));
    }

    #[test]
    fn test_parse_date_yymmdd() {
        let expected = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        assert_eq!(parse_date("240113"), Ok(expected));
    }

    #[test]
    fn test_parse_date_yyyy_mm_dd() {
        let expected = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        assert_eq!(parse_date("2024-01-13"), Ok(expected));
    }

    #[test]
    fn test_parse_date_yy_mm_dd() {
        let expected = NaiveDate::from_ymd_opt(2024, 1, 13).unwrap();
        assert_eq!(parse_date("24-01-13"), Ok(expected));
    }

    #[test]
    fn test_parse_month_mmm() {
        let current_year = Local::now().year();
        let expected = NaiveDate::from_ymd_opt(current_year, 2, 1).unwrap();
        assert_eq!(parse_month("Feb"), Ok(expected));
    }

    #[test]
    fn test_parse_month_mmm_yy() {
        let expected = NaiveDate::from_ymd_opt(2022, 2, 1).unwrap();
        assert_eq!(parse_month("Feb 22"), Ok(expected));
    }

    #[test]
    fn test_parse_month_mmm_yyyy() {
        let expected = NaiveDate::from_ymd_opt(2022, 2, 1).unwrap();
        assert_eq!(parse_month("Feb 2022"), Ok(expected));
    }

    #[test]
    fn test_parse_month_m() {
        let current_year = Local::now().year();
        let expected = NaiveDate::from_ymd_opt(current_year, 2, 1).unwrap();
        assert_eq!(parse_month("2"), Ok(expected));
    }

    #[test]
    fn test_parse_month_mm() {
        let current_year = Local::now().year();
        let expected = NaiveDate::from_ymd_opt(current_year, 2, 1).unwrap();
        assert_eq!(parse_month("02"), Ok(expected));
    }

    #[test]
    fn test_parse_month_m_yy() {
        let expected = NaiveDate::from_ymd_opt(2022, 2, 1).unwrap();
        assert_eq!(parse_month("2/2022"), Ok(expected));
    }

    #[test]
    fn test_parse_month_mm_yy() {
        let expected = NaiveDate::from_ymd_opt(2022, 2, 1).unwrap();
        assert_eq!(parse_month("02/2022"), Ok(expected));
    }
}
