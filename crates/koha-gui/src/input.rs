//! Penerjemahan event `winit` menjadi [`Input`] untuk state machine menu.

use koha_core::Input;
use winit::keyboard::{Key, NamedKey};

/// Hasil menerjemahkan satu penekanan tombol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAction {
    Send(Input),
    Ignore,
}

/// Menerjemahkan tombol yang ditekan.
///
/// - `shift`: apakah Shift sedang ditahan (untuk membedakan Tab dan Shift+Tab).
/// - `repeat`: `true` bila ini pengulangan karena tombol ditahan.
/// - `synthetic`: `true` bila event ini dibuat `winit` (bukan penekanan baru).
///
/// Aturannya mengikuti AHK: hanya `↑ ↓ Tab Shift+Tab Enter Esc` yang diterima,
/// Shift sendirian diabaikan (ia ditekan lebih dulu sebelum Tab), dan tombol
/// lain apa pun menutup menu. Pengulangan hanya berlaku untuk navigasi; `Enter`
/// dan `Esc` yang berulang diabaikan agar menahan `Esc` tidak menutup beberapa
/// level sekaligus.
pub fn map_key(key: &Key, shift: bool, repeat: bool, synthetic: bool) -> KeyAction {
    use KeyAction::{Ignore, Send};
    // Saat jendela mendapat fokus, `winit` menerbitkan penekanan tombol sintetis
    // untuk tombol yang sedang ditahan. Kasus nyatanya: KOHA dipanggil dari
    // sebuah tombol (misalnya Lenovo Vantage) yang masih tertahan ketika jendela
    // muncul. Tanpa pengecualian ini tombol itu dianggap "tombol lain" dan
    // langsung menutup menu.
    if synthetic {
        return Ignore;
    }
    match key {
        Key::Named(NamedKey::ArrowUp) => Send(Input::Up),
        Key::Named(NamedKey::ArrowDown) => Send(Input::Down),
        Key::Named(NamedKey::Tab) if shift => Send(Input::Up),
        Key::Named(NamedKey::Tab) => Send(Input::Down),
        // NumpadEnter juga terbaca sebagai `Enter` pada kunci logis.
        Key::Named(NamedKey::Enter) if repeat => Ignore,
        Key::Named(NamedKey::Enter) => Send(Input::Enter),
        Key::Named(NamedKey::Escape) if repeat => Ignore,
        Key::Named(NamedKey::Escape) => Send(Input::Back),
        Key::Named(NamedKey::Shift) => Ignore,
        _ => Send(Input::Dismiss),
    }
}

/// Aturan dismiss-on-blur.
///
/// Jendela baru dibuat bisa menerima event "kehilangan fokus" sebelum sempat
/// benar-benar mendapat fokus. AHK menahannya dengan timer 200 ms; di sini
/// `Focused(false)` hanya dianggap `Dismiss` **setelah** jendela pernah
/// menerima fokus. Aturan ini deterministik dan tidak bergantung pada waktu.
#[derive(Debug, Default)]
pub struct FocusGate {
    had_focus: bool,
}

impl FocusGate {
    pub fn on_focus_changed(&mut self, focused: bool) -> Option<Input> {
        if focused {
            self.had_focus = true;
            None
        } else if self.had_focus {
            Some(Input::Dismiss)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(key: NamedKey) -> Key {
        Key::Named(key)
    }

    fn send(input: Input) -> KeyAction {
        KeyAction::Send(input)
    }

    #[test]
    fn arrows_map_to_up_and_down() {
        assert_eq!(
            map_key(&named(NamedKey::ArrowUp), false, false, false),
            send(Input::Up)
        );
        assert_eq!(
            map_key(&named(NamedKey::ArrowDown), false, false, false),
            send(Input::Down)
        );
    }

    #[test]
    fn tab_is_down_and_shift_tab_is_up() {
        assert_eq!(
            map_key(&named(NamedKey::Tab), false, false, false),
            send(Input::Down)
        );
        assert_eq!(
            map_key(&named(NamedKey::Tab), true, false, false),
            send(Input::Up)
        );
    }

    #[test]
    fn enter_and_escape_map_to_enter_and_back() {
        assert_eq!(
            map_key(&named(NamedKey::Enter), false, false, false),
            send(Input::Enter)
        );
        assert_eq!(
            map_key(&named(NamedKey::Escape), false, false, false),
            send(Input::Back)
        );
    }

    #[test]
    fn shift_alone_is_ignored_not_dismissed() {
        assert_eq!(
            map_key(&named(NamedKey::Shift), true, false, false),
            KeyAction::Ignore
        );
    }

    #[test]
    fn other_keys_dismiss() {
        assert_eq!(
            map_key(&Key::Character("a".into()), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::Space), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::Control), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::Alt), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::Super), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::F5), false, false, false),
            send(Input::Dismiss)
        );
    }

    #[test]
    fn held_navigation_keys_keep_repeating() {
        assert_eq!(
            map_key(&named(NamedKey::ArrowDown), false, true, false),
            send(Input::Down)
        );
        assert_eq!(
            map_key(&named(NamedKey::ArrowUp), false, true, false),
            send(Input::Up)
        );
        assert_eq!(
            map_key(&named(NamedKey::Tab), false, true, false),
            send(Input::Down)
        );
        assert_eq!(
            map_key(&named(NamedKey::Tab), true, true, false),
            send(Input::Up)
        );
    }

    #[test]
    fn held_enter_and_escape_do_not_repeat() {
        assert_eq!(
            map_key(&named(NamedKey::Enter), false, true, false),
            KeyAction::Ignore
        );
        assert_eq!(
            map_key(&named(NamedKey::Escape), false, true, false),
            KeyAction::Ignore
        );
    }

    #[test]
    fn synthetic_key_events_are_ignored_even_for_keys_that_would_dismiss() {
        // Tombol pemicu yang masih tertahan saat jendela muncul, atau tombol
        // yang dianggap tertahan oleh sistem (mis. AudioVolumeUp).
        for key in [
            named(NamedKey::AudioVolumeUp),
            named(NamedKey::F12),
            Key::Character("a".into()),
            named(NamedKey::Enter),
            named(NamedKey::Escape),
            named(NamedKey::ArrowDown),
        ] {
            assert_eq!(
                map_key(&key, false, false, true),
                KeyAction::Ignore,
                "{key:?}"
            );
        }
    }

    #[test]
    fn a_real_press_of_the_same_keys_still_works() {
        assert_eq!(
            map_key(&named(NamedKey::AudioVolumeUp), false, false, false),
            send(Input::Dismiss)
        );
        assert_eq!(
            map_key(&named(NamedKey::Enter), false, false, false),
            send(Input::Enter)
        );
    }

    #[test]
    fn blur_before_ever_being_focused_is_ignored() {
        let mut gate = FocusGate::default();
        assert_eq!(gate.on_focus_changed(false), None);
    }

    #[test]
    fn blur_after_being_focused_dismisses() {
        let mut gate = FocusGate::default();
        assert_eq!(gate.on_focus_changed(true), None);
        assert_eq!(gate.on_focus_changed(false), Some(Input::Dismiss));
    }

    #[test]
    fn early_blur_does_not_prevent_later_dismiss() {
        let mut gate = FocusGate::default();
        assert_eq!(gate.on_focus_changed(false), None);
        assert_eq!(gate.on_focus_changed(true), None);
        assert_eq!(gate.on_focus_changed(false), Some(Input::Dismiss));
    }
}
