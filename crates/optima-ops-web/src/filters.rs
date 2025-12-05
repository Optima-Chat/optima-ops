//! Custom Askama template filters

/// Round a float to the specified number of decimal places
pub fn round(value: &f64, precision: usize) -> askama::Result<String> {
    Ok(format!("{:.prec$}", value, prec = precision))
}

/// Default value filter for Option<&str>
pub fn default<'a>(value: &'a Option<&'a str>, default_value: &'a str) -> askama::Result<&'a str> {
    Ok(value.unwrap_or(default_value))
}
