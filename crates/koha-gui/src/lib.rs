//! GUI KOHA (`winit` + `softbuffer` pada langkah berikutnya).

/// Judul jendela popup.
pub fn window_title() -> String {
    koha_core::APP_NAME.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_is_uppercase_app_name() {
        assert_eq!(window_title(), "KOHA");
    }
}
