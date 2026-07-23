use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use bcrypt::{verify as bcrypt_verify, HashParts};
use rand::rngs::OsRng;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("password hashing failed")]
    HashingFailed,
    #[error("password verification failed")]
    VerificationFailed,
}

pub const LEGACY_BCRYPT_ALGORITHM: &str = "bcrypt";
pub const MINIMUM_PASSWORD_LENGTH: usize = 8;

pub fn password_meets_policy(password: &str) -> bool {
    password.trim().len() >= MINIMUM_PASSWORD_LENGTH
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| PasswordError::HashingFailed)?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<(), PasswordError> {
    let parsed = PasswordHash::new(password_hash).map_err(|_| PasswordError::VerificationFailed)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| PasswordError::VerificationFailed)
}

pub fn verify_legacy_bcrypt(
    password: &str,
    algorithm: &str,
    verifier: &str,
) -> Result<(), PasswordError> {
    if algorithm != LEGACY_BCRYPT_ALGORITHM {
        return Err(PasswordError::VerificationFailed);
    }
    let parsed = HashParts::from_str(verifier).map_err(|_| PasswordError::VerificationFailed)?;
    if parsed.get_cost() != 6 || !verifier.starts_with("$2b$") {
        return Err(PasswordError::VerificationFailed);
    }
    if !bcrypt_verify(password, verifier).map_err(|_| PasswordError::VerificationFailed)? {
        return Err(PasswordError::VerificationFailed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_policy_requires_eight_trimmed_characters() {
        assert!(password_meets_policy("12345678"));
        assert!(!password_meets_policy(" 1234567 "));
    }

    #[test]
    fn hash_and_verify_password_round_trip() {
        let hash = hash_password("test-password").expect("hash");
        assert!(verify_password("test-password", &hash).is_ok());
        assert!(verify_password("wrong-password", &hash).is_err());
    }

    #[test]
    fn legacy_bcrypt_requires_pro_format() {
        let password = "synthetic-password";
        let hash = bcrypt::hash(password, 6).expect("bcrypt hash");
        assert!(verify_legacy_bcrypt(password, LEGACY_BCRYPT_ALGORITHM, &hash).is_ok());
        assert!(verify_legacy_bcrypt("wrong", LEGACY_BCRYPT_ALGORITHM, &hash).is_err());
        assert!(verify_legacy_bcrypt(password, "argon2", &hash).is_err());

        let long = "p".repeat(72);
        let long_hash = bcrypt::hash(&long, 6).expect("bcrypt hash");
        assert!(verify_legacy_bcrypt(&long, LEGACY_BCRYPT_ALGORITHM, &long_hash).is_ok());
        assert!(verify_legacy_bcrypt(&(long + "x"), LEGACY_BCRYPT_ALGORITHM, &long_hash).is_ok());
    }
}
