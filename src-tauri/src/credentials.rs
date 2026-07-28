use keyring::Entry;

const SERVICE: &str = "m2manager";

pub fn store_secret(account: &str, secret: &str) -> Result<(), String> {
    Entry::new(SERVICE, account)
        .map_err(|e| e.to_string())?
        .set_password(secret)
        .map_err(|e| e.to_string())
}

pub fn get_secret(account: &str) -> Result<String, String> {
    Entry::new(SERVICE, account)
        .map_err(|e| e.to_string())?
        .get_password()
        .map_err(|e| e.to_string())
}

pub fn delete_secret(account: &str) -> Result<(), String> {
    Entry::new(SERVICE, account)
        .map_err(|e| e.to_string())?
        .delete_credential()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_then_get_roundtrip() {
        store_secret("diagnostic_test", "hunter2").expect("store failed");
        let value = get_secret("diagnostic_test").expect("get failed");
        assert_eq!(value, "hunter2");
        println!("roundtrip ok, cleaning up");
        delete_secret("diagnostic_test").expect("delete failed");
    }
}
