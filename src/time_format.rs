use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn format_timestamp(value: OffsetDateTime) -> String {
    value
        .format(&Rfc3339)
        .unwrap_or_else(|_| value.unix_timestamp().to_string())
}

pub fn parse_timestamp(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

pub fn format_last_checkin_display(value: Option<&str>) -> String {
    match value.and_then(parse_timestamp) {
        Some(timestamp) => format_timestamp(timestamp),
        None => "-".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn format_and_parse_timestamp_round_trip() {
        let value = datetime!(2026-06-23 12:00:00 UTC);
        let formatted = format_timestamp(value);
        let parsed = parse_timestamp(&formatted).expect("parse");
        assert_eq!(parsed, value);
    }
}