/// Trait for accessing environment variables in a testable way
///
/// This trait provides an abstraction over environment variable access,
/// allowing for dependency injection and mocking in tests without using
/// unsafe global state manipulation.

#[cfg(test)]
use std::collections::HashMap;
pub trait EnvProvider {
    /// Get the value of an environment variable
    fn get_var(&self, key: &str) -> Option<String>;
}

/// Production implementation that uses the system environment
pub struct SystemEnvProvider;

impl EnvProvider for SystemEnvProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// Test-only mock implementation for environment variables
#[cfg(test)]
pub struct MockEnvProvider {
    vars: HashMap<String, String>,
}

#[cfg(test)]
impl MockEnvProvider {
    /// Create a new empty mock environment provider
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Set an environment variable in the mock
    pub fn set_var(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    /// Remove an environment variable from the mock
    pub fn remove_var(&mut self, key: &str) -> &mut Self {
        self.vars.remove(key);
        self
    }

    /// Create a mock with common Git environment variables set
    pub fn with_git_author(name: impl Into<String>, email: impl Into<String>) -> Self {
        let mut mock = Self::new();
        mock.set_var("GIT_AUTHOR_NAME", name);
        mock.set_var("GIT_AUTHOR_EMAIL", email);
        mock
    }
}

#[cfg(test)]
impl EnvProvider for MockEnvProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        self.vars.get(key).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_env_provider() {
        let provider = SystemEnvProvider;

        // This test depends on the system environment, so we just verify
        // the interface works without asserting specific values
        let _path = provider.get_var("PATH");
        let _nonexistent = provider.get_var("NONEXISTENT_VAR_12345");
    }

    #[test]
    fn test_mock_env_provider() {
        let mut mock = MockEnvProvider::new();

        // Initially empty
        assert_eq!(mock.get_var("TEST_VAR"), None);

        // Set a variable
        mock.set_var("TEST_VAR", "test_value");
        assert_eq!(mock.get_var("TEST_VAR"), Some("test_value".to_string()));

        // Remove a variable
        mock.remove_var("TEST_VAR");
        assert_eq!(mock.get_var("TEST_VAR"), None);
    }

    #[test]
    fn test_mock_env_provider_builder() {
        let mut mock = MockEnvProvider::new();
        mock.set_var("VAR1", "value1").set_var("VAR2", "value2");

        assert_eq!(mock.get_var("VAR1"), Some("value1".to_string()));
        assert_eq!(mock.get_var("VAR2"), Some("value2".to_string()));
        assert_eq!(mock.get_var("VAR3"), None);
    }

    #[test]
    fn test_with_git_author() {
        let mock = MockEnvProvider::with_git_author("Test User", "test@example.com");

        assert_eq!(
            mock.get_var("GIT_AUTHOR_NAME"),
            Some("Test User".to_string())
        );
        assert_eq!(
            mock.get_var("GIT_AUTHOR_EMAIL"),
            Some("test@example.com".to_string())
        );
        assert_eq!(mock.get_var("OTHER_VAR"), None);
    }
}
