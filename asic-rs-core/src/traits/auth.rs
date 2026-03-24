pub use secrecy::{ExposeSecret, SecretString};

/// Credentials for authenticating with a miner.
#[derive(Clone, Debug)]
pub struct MinerAuth {
    pub username: String,
    pub password: SecretString,
}

impl MinerAuth {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: SecretString::from(password.into()),
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
