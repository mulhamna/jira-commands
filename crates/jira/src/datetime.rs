use anyhow::{anyhow, Result};
use chrono::{Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};

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

    let naive = NaiveDateTime::new(date.expect("date set"), time.expect("time set"));
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

    Ok(Some(local.format("%Y-%m-%dT%H:%M:%S.000%z").to_string()))
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
}
