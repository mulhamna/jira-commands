use keyring::Entry;

use crate::error::{JiraError, Result};

const SERVICE_NAME: &str = "jira-cli";

pub struct Auth;

impl Auth {
    /// Save token to OS keyring.
    pub fn save_token(email: &str, token: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, email)
            .map_err(|e| JiraError::Auth(format!("Keyring error: {e}")))?;
        entry
            .set_password(token)
            .map_err(|e| JiraError::Auth(format!("Failed to save token: {e}")))?;
        Ok(())
    }

    /// Retrieve token from OS keyring.
    pub fn get_token(email: &str) -> Result<String> {
        let entry = Entry::new(SERVICE_NAME, email)
            .map_err(|e| JiraError::Auth(format!("Keyring error: {e}")))?;
        entry
            .get_password()
            .map_err(|e| JiraError::Auth(format!("Token not found: {e}")))
    }

    /// Delete token from OS keyring.
    pub fn delete_token(email: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, email)
            .map_err(|e| JiraError::Auth(format!("Keyring error: {e}")))?;
        entry
            .delete_credential()
            .map_err(|e| JiraError::Auth(format!("Failed to delete token: {e}")))?;
        Ok(())
    }
}
