//! Abstraksi OS (ports and adapters). Implementasi per OS dipilih dengan `cfg`.

/// Nama OS tempat crate ini dikompilasi.
pub fn os_name() -> &'static str {
    std::env::consts::OS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_name_is_not_empty() {
        assert!(!os_name().is_empty());
    }
}
