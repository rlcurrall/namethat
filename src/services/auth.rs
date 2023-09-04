use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;

use crate::error::Result;

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String> {
        Ok(Argon2::default()
            .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))?
            .to_string())
    }

    pub fn check_password(password: &str, hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(hash)?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}
