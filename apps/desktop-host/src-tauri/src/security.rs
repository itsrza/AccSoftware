/// Validate an SQL identifier before it is inserted into a SQL statement.
/// Dynamic identifiers must come from application-controlled allowlists.
pub fn validate_identifier(value: &str) -> Result<&str, String> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    if valid {
        Ok(value)
    } else {
        Err("SQL-001: invalid SQL identifier".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::validate_identifier;

    #[test]
    fn accepts_normal_identifier() {
        assert_eq!(validate_identifier("sales_invoices").unwrap(), "sales_invoices");
    }

    #[test]
    fn rejects_injection_text() {
        assert!(validate_identifier("sales_invoices; DROP TABLE users").is_err());
    }
}
