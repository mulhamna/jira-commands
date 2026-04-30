use anyhow::{anyhow, Result};
use chrono::{
    Datelike, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday,
};

/// Build a Jira worklog `started` timestamp.
///
/// Input forms:
/// - no date + no start => None (let Jira default to now)
/// - date only => use that date with the current local time
/// - start only => use today's local date with that time
/// - date + start => combine both
///
/// Output format: `YYYY-MM-DDTHH:MM:SS.000+ZZZZ`
pub fn build_worklog_started(date: Option<&str>, start: Option<&str>) -> Result<Option<String>> {
    if date.is_none() && start.is_none() {
        return Ok(None);
    }

    let now = Local::now();

    let date = match date {
        Some(value) => Some(parse_date(value)?),
        None => Some(now.date_naive()),
    };

    let time = match start {
        Some(value) => Some(parse_time(value)?),
        None => Some(now.time()),
    };

    let local = resolve_local_datetime(date.expect("date set"), time.expect("time set"))?;

    Ok(Some(format_started(local)))
}

/// Build a Jira worklog `started` timestamp for a specific date.
///
/// When `start` is omitted, the current local time is used.
pub fn build_worklog_started_for_date(date: NaiveDate, start: Option<&str>) -> Result<String> {
    let time = match start {
        Some(value) => parse_time(value)?,
        None => Local::now().time(),
    };

    let local = resolve_local_datetime(date, time)?;
    Ok(format_started(local))
}

/// Build an inclusive list of dates for a worklog range.
pub fn build_worklog_range_dates(
    from: &str,
    to: &str,
    exclude_weekends: bool,
) -> Result<Vec<NaiveDate>> {
    let start_date = parse_date(from)?;
    let end_date = parse_date(to)?;

    if end_date < start_date {
        return Err(anyhow!(
            "Invalid range: end date '{}' is before start date '{}'",
            to.trim(),
            from.trim()
        ));
    }

    let mut dates = Vec::new();
    let mut current = start_date;

    loop {
        let is_weekend = matches!(current.weekday(), Weekday::Sat | Weekday::Sun);
        if !exclude_weekends || !is_weekend {
            dates.push(current);
        }

        if current == end_date {
            break;
        }

        current = current
            .succ_opt()
            .ok_or_else(|| anyhow!("Could not advance date range beyond {}", current))?;
    }

    Ok(dates)
}

fn resolve_local_datetime(date: NaiveDate, time: NaiveTime) -> Result<chrono::DateTime<Local>> {
    let naive = NaiveDateTime::new(date, time);
    let local = match Local.from_local_datetime(&naive) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(first, _) => first,
        LocalResult::None => {
            return Err(anyhow!(
                "Could not resolve local time for started timestamp: {} {}",
                naive.date(),
                naive.time().format("%H:%M:%S")
            ))
        }
    };

    Ok(local)
}

fn format_started(local: chrono::DateTime<Local>) -> String {
    local.format("%Y-%m-%dT%H:%M:%S.000%z").to_string()
}

fn parse_date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| {
        anyhow!(
            "Invalid date '{}'. Expected format: YYYY-MM-DD",
            value.trim()
        )
    })
}

fn parse_time(value: &str) -> Result<NaiveTime> {
    let value = value.trim();

    NaiveTime::parse_from_str(value, "%H:%M")
        .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M:%S"))
        .map_err(|_| {
            anyhow!(
                "Invalid start time '{}'. Expected format: HH:MM or HH:MM:SS",
                value
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_when_no_inputs_are_provided() {
        assert!(build_worklog_started(None, None).unwrap().is_none());
    }

    #[test]
    fn builds_timestamp_when_date_and_time_are_provided() {
        let started = build_worklog_started(Some("2026-04-21"), Some("09:30"))
            .unwrap()
            .unwrap();

        assert!(started.starts_with("2026-04-21T09:30:00.000"));
        assert!(started.ends_with("+0000") || started.ends_with("-0000") || started.len() == 28);
    }

    #[test]
    fn rejects_invalid_date() {
        let err = build_worklog_started(Some("21-04-2026"), Some("09:30")).unwrap_err();
        assert!(err.to_string().contains("Invalid date"));
    }

    #[test]
    fn rejects_invalid_time() {
        let err = build_worklog_started(Some("2026-04-21"), Some("9.30")).unwrap_err();
        assert!(err.to_string().contains("Invalid start time"));
    }

    #[test]
    fn builds_inclusive_range_dates() {
        let dates = build_worklog_range_dates("2026-04-21", "2026-04-23", false).unwrap();
        let rendered: Vec<String> = dates.iter().map(|d| d.to_string()).collect();

        assert_eq!(rendered, vec!["2026-04-21", "2026-04-22", "2026-04-23"]);
    }

    #[test]
    fn excludes_weekends_from_range_when_requested() {
        let dates = build_worklog_range_dates("2026-04-24", "2026-04-27", true).unwrap();
        let rendered: Vec<String> = dates.iter().map(|d| d.to_string()).collect();

        assert_eq!(rendered, vec!["2026-04-24", "2026-04-27"]);
    }

    #[test]
    fn rejects_reversed_ranges() {
        let err = build_worklog_range_dates("2026-04-27", "2026-04-24", false).unwrap_err();
        assert!(err.to_string().contains("Invalid range"));
    }
}
