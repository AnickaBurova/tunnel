use std::str;

pub fn is_val<T: str::FromStr>(x: String) -> Result<(), String> {
    x.parse::<T>()
        .and_then(|_| Ok(()))
        .or_else(|_| Err(format!("Cannot parse {}", x)))
}
