use keyring::Entry;

const SERVICE_NAME: &str = "nats-manager";

/// Stores a credential in the operating system's credential store.
/// Callers should persist only the returned account key in profile data.
pub fn store(account: &str, secret: &str) -> Result<(), keyring::Error> {
    Entry::new(SERVICE_NAME, account)?.set_password(secret)
}

/// Loads a credential from the operating system's credential store.
pub fn load(account: &str) -> Result<String, keyring::Error> {
    Entry::new(SERVICE_NAME, account)?.get_password()
}

/// Removes a credential from the operating system's credential store.
pub fn delete(account: &str) -> Result<(), keyring::Error> {
    Entry::new(SERVICE_NAME, account)?.delete_credential()
}
