//! Logika murni KOHA. Crate ini tidak boleh memanggil API OS maupun GUI.

/// Nama aplikasi, dipakai bersama oleh crate lain.
pub const APP_NAME: &str = "koha";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_koha() {
        assert_eq!(APP_NAME, "koha");
    }
}
