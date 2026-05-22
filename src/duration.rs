use std::time::Duration;

use crate::error::{NexaError, Result};

pub fn parse_duration(s: &str) -> Result<Duration> {
    if s.is_empty() {
        return Err(NexaError::InvalidSpec("duration string is empty".into()));
    }

    if let Some(ms) = s.strip_suffix("ms") {
        let n: u64 = ms
            .parse()
            .map_err(|_| NexaError::InvalidSpec(format!("invalid duration: {s}")))?;
        return Ok(Duration::from_millis(n));
    }
    if let Some(h) = s.strip_suffix('h') {
        let n: u64 = h
            .parse()
            .map_err(|_| NexaError::InvalidSpec(format!("invalid duration: {s}")))?;
        return Ok(Duration::from_secs(n * 3600));
    }
    if let Some(m) = s.strip_suffix('m') {
        let n: u64 = m
            .parse()
            .map_err(|_| NexaError::InvalidSpec(format!("invalid duration: {s}")))?;
        return Ok(Duration::from_secs(n * 60));
    }
    if let Some(sec) = s.strip_suffix('s') {
        let n: u64 = sec
            .parse()
            .map_err(|_| NexaError::InvalidSpec(format!("invalid duration: {s}")))?;
        return Ok(Duration::from_secs(n));
    }

    // Bare number → seconds
    match s.parse::<u64>() {
        Ok(n) => Ok(Duration::from_secs(n)),
        Err(_) => Err(NexaError::InvalidSpec(format!("invalid duration: {s}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds() {
        assert_eq!(parse_duration("10s").unwrap(), Duration::from_secs(10));
    }

    #[test]
    fn parse_minutes() {
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
    }

    #[test]
    fn parse_hours() {
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    #[test]
    fn parse_milliseconds() {
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
    }

    #[test]
    fn parse_bare_number_as_seconds() {
        assert_eq!(parse_duration("30").unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn reject_empty_string() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn reject_invalid_format() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("10x").is_err());
    }
}
