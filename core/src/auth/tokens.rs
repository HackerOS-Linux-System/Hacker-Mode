use anyhow::Result;
use keyring::Entry;

pub fn store_token(service: &str, token: &str) -> Result<()> {
    let entry = Entry::new("hackeros-hm", service)?;
    entry.set_password(token)?;
    Ok(())
}

pub fn load_token(service: &str) -> Result<Option<String>> {
    let entry = Entry::new("hackeros-hm", service)?;
    match entry.get_password() {
        Ok(t)                        => Ok(Some(t)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e)                       => Err(e.into()),
    }
}

pub fn delete_token(service: &str) -> Result<()> {
    let entry = Entry::new("hackeros-hm", service)?;
    entry.delete_password()?;
    Ok(())
}
