
pub fn set_secret(service: &str, user: &str, secret: &str) -> keyring::Result<()> {
    keyring::Entry::new(service, user)?.set_password(secret)
}

pub fn get_secret(service: &str, user: &str) -> keyring::Result<String> {
    keyring::Entry::new(service, user)?.get_password()
}
