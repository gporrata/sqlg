use crate::store::StoreError;

/// Placeholder so non-macOS builds still typecheck. sqlg ships on macOS.
pub struct Keychain;

impl Keychain {
    pub(crate) fn user_default() -> Result<Self, StoreError> {
        Err(StoreError::Keychain(
            "macOS Keychain is unavailable on this platform".into(),
        ))
    }

    pub(crate) fn get(&self, _account: &str) -> Result<Option<String>, StoreError> {
        Err(StoreError::Keychain(
            "macOS Keychain is unavailable on this platform".into(),
        ))
    }

    pub(crate) fn set(&self, _account: &str, _secret: &str) -> Result<(), StoreError> {
        Err(StoreError::Keychain(
            "macOS Keychain is unavailable on this platform".into(),
        ))
    }

    pub(crate) fn delete(&self, _account: &str) -> Result<(), StoreError> {
        Err(StoreError::Keychain(
            "macOS Keychain is unavailable on this platform".into(),
        ))
    }
}
