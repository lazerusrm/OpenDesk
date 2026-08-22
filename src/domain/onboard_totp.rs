use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;
use time::OffsetDateTime;

pub const TOTP_DIGITS: u32 = 6;
pub const TOTP_PERIOD_SECS: i64 = 30;
pub const TOTP_SECRET_BYTES: usize = 20;
const TOTP_SKEW_STEPS: i64 = 1;
const BASE32: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn generate_totp_secret() -> Vec<u8> {
    let mut secret = vec![0u8; TOTP_SECRET_BYTES];
    rand::thread_rng().fill_bytes(&mut secret);
    secret
}

pub fn decode_base32(value: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut nbits = 0;
    let mut out = Vec::new();
    for ch in value.chars() {
        if ch == '=' || ch.is_whitespace() {
            continue;
        }
        let encoded = ch.to_ascii_uppercase() as u8;
        let val = BASE32.iter().position(|&item| item == encoded)?;
        bits = (bits << 5) | val as u32;
        nbits += 5;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }
    Some(out)
}

pub fn encode_base32(data: &[u8]) -> String {
    let mut bits = 0u32;
    let mut nbits = 0;
    let mut out = String::new();
    for &byte in data {
        bits = (bits << 8) | u32::from(byte);
        nbits += 8;
        while nbits >= 5 {
            nbits -= 5;
            out.push(BASE32[((bits >> nbits) & 31) as usize] as char);
        }
    }
    if nbits > 0 {
        out.push(BASE32[((bits << (5 - nbits)) & 31) as usize] as char);
    }
    out
}

pub fn otpauth_url(username: &str, secret: &[u8]) -> String {
    let label: String = username
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    format!(
        "otpauth://totp/OpenDesk:{label}?secret={secret}&issuer=OpenDesk&digits={TOTP_DIGITS}&period={TOTP_PERIOD_SECS}",
        secret = encode_base32(secret)
    )
}

pub fn parse_onboard_code(value: &str) -> Option<String> {
    let digits: String = value.chars().filter(|ch| ch.is_ascii_digit()).collect();
    (digits.len() == TOTP_DIGITS as usize).then_some(digits)
}

pub fn totp_code(secret: &[u8], unix: i64) -> String {
    format!(
        "{:0width$}",
        hotp(secret, timestep(unix)),
        width = TOTP_DIGITS as usize
    )
}

/// Returns the matching timestep when the code is valid for this secret.
pub fn verify_totp(secret: &[u8], code: &str, now: OffsetDateTime) -> Option<i64> {
    let parsed = parse_onboard_code(code)?;
    let unix = now.unix_timestamp();
    let center = timestep(unix);
    for delta in -TOTP_SKEW_STEPS..=TOTP_SKEW_STEPS {
        let step = center + delta;
        if step < 0 {
            continue;
        }
        let candidate = format!(
            "{:0width$}",
            hotp(secret, step),
            width = TOTP_DIGITS as usize
        );
        if constant_time_eq(candidate.as_bytes(), parsed.as_bytes()) {
            return Some(step);
        }
    }
    None
}

fn timestep(unix: i64) -> i64 {
    unix.div_euclid(TOTP_PERIOD_SECS)
}

fn hotp(secret: &[u8], timestep: i64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("hmac key");
    mac.update(&(timestep as u64).to_be_bytes());
    let hash = mac.finalize().into_bytes();
    let offset = (hash[19] & 0x0f) as usize;
    let binary =
        u32::from_be_bytes(hash[offset..offset + 4].try_into().expect("hmac slice")) & 0x7fff_ffff;
    binary % 10u32.pow(TOTP_DIGITS)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = (left.len() ^ right.len()) as u8;
    for index in 0..left.len().max(right.len()) {
        difference |=
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn rfc6238_sha1_six_digit_vector() {
        let secret = b"12345678901234567890";
        let now = OffsetDateTime::from_unix_timestamp(59).expect("ts");
        assert_eq!(totp_code(secret, 59), "287082");
        assert_eq!(verify_totp(secret, "287082", now), Some(1));
        assert_eq!(verify_totp(secret, "000000", now), None);
    }

    #[test]
    fn parse_onboard_code_accepts_spaced_digits() {
        assert_eq!(parse_onboard_code("12 34 56").as_deref(), Some("123456"));
        assert_eq!(parse_onboard_code("12345"), None);
        assert_eq!(parse_onboard_code("12345a"), None);
    }

    #[test]
    fn otpauth_url_uses_base32_secret() {
        let secret = b"12345678901234567890";
        let url = otpauth_url("admin.user", secret);
        assert!(url.starts_with("otpauth://totp/OpenDesk:adminuser?secret="));
        assert!(url.contains("&issuer=OpenDesk"));
        assert!(encode_base32(secret)
            .chars()
            .all(|ch| BASE32.contains(&(ch as u8))));
        assert_eq!(
            decode_base32(&encode_base32(secret)).as_deref(),
            Some(&secret[..])
        );
    }
}
