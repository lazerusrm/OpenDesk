use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PasswordError {
    #[error("password hashing failed")]
    HashingFailed,
    #[error("password verification failed")]
    VerificationFailed,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_password_round_trip() {
        let hash = hash_password("test-password").expect("hash");
        assert!(verify_password("test-password", &hash).is_ok());
        assert!(verify_password("wrong-password", &hash).is_err());
    }
}