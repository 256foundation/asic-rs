pub use secrecy::{ExposeSecret, SecretString};

/// Username/password credentials.
#[derive(Clone, Debug)]
pub struct UserAndPassAuth {
    pub username: String,
    pub password: SecretString,
}

/// Credentials for authenticating with a miner.
///
/// Most firmwares authenticate with a username/password ([`MinerAuth::UserAndPass`]).
/// Some (e.g. VNish, BraiinsOS HTTP) accept a pre-issued bearer token instead
/// ([`MinerAuth::TokenAuth`]). The two modes are mutually exclusive — backends
/// `match` on the variant to pick the right path.
#[derive(Clone, Debug)]
pub enum MinerAuth {
    /// Username + password login.
    UserAndPass(UserAndPassAuth),
    /// Pre-issued bearer token (used directly, no password login).
    TokenAuth(SecretString),
}

impl MinerAuth {
    /// Build username/password credentials.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        MinerAuth::UserAndPass(UserAndPassAuth {
            username: username.into(),
            password: SecretString::from(password.into()),
        })
    }

    /// Build pre-issued token credentials (e.g. a bearer token).
    pub fn from_token(token: impl Into<String>) -> Self {
        MinerAuth::TokenAuth(SecretString::from(token.into()))
    }

    /// Username for user/pass auth; empty string for token auth.
    pub fn username(&self) -> &str {
        match self {
            MinerAuth::UserAndPass(c) => &c.username,
            MinerAuth::TokenAuth(_) => "",
        }
    }

    /// Password for user/pass auth; empty string for token auth.
    pub fn password(&self) -> &str {
        match self {
            MinerAuth::UserAndPass(c) => c.password.expose_secret(),
            MinerAuth::TokenAuth(_) => "",
        }
    }

    /// The pre-issued token, if this is token auth.
    pub fn token(&self) -> Option<&SecretString> {
        match self {
            MinerAuth::TokenAuth(t) => Some(t),
            MinerAuth::UserAndPass(_) => None,
        }
    }
}

/// Trait for applying authentication credentials to a miner at runtime.
///
/// `set_auth` has a default no-op for backends that don't support
/// credential override at runtime.
pub trait HasAuth: Send + Sync {
    /// Apply authentication credentials to this miner.
    fn set_auth(&mut self, _auth: MinerAuth) {}
}

/// Trait for declaring the default credentials for a backend.
///
/// Returns empty credentials by default for backends that don't require auth.
pub trait HasDefaultAuth: Send + Sync {
    /// The default credentials for this backend.
    fn default_auth() -> MinerAuth
    where
        Self: Sized,
    {
        MinerAuth::new("", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_password() {
        // Arrange
        let auth = MinerAuth::new("admin", "secret123");

        // Act
        let debug = format!("{:?}", auth);

        // Assert
        assert!(debug.contains("admin"));
        assert!(!debug.contains("secret123"));
    }
}
