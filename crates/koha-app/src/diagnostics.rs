//! Pesan error config yang menunjuk lokasi masalah, dengan potongan barisnya.

use std::path::Path;

use koha_core::ConfigError;

/// Mengubah [`ConfigError`] menjadi teks seperti ini:
///
/// ```text
/// error: unknown field `bogus`
///   --> /path/config.toml:7:1
///    |
///  7 | bogus = 1
///    | ^
/// ```
///
/// Kesalahan semantik tidak punya nomor baris, jadi yang ditampilkan adalah
/// jalur itemnya (misalnya `menu[1].items[0]`).
pub fn render(path: &Path, source: &str, error: &ConfigError) -> String {
    let location = path.display();
    match error {
        ConfigError::Syntax {
            line,
            column,
            message,
        } => {
            let number = line.to_string();
            let pad = " ".repeat(number.len());
            let text = source.lines().nth(line - 1).unwrap_or("");
            // Spasi di depan tanda ^ meniru isi baris: tab tetap tab, supaya
            // tanda tetap tegak lurus di bawah karakter yang dimaksud.
            let indent: String = text
                .chars()
                .take(column - 1)
                .map(|c| if c == '\t' { '\t' } else { ' ' })
                .collect();
            // Kolom di luar ujung baris (misalnya "kurang kurung tutup" di akhir
            // baris) tetap ditandai dengan spasi.
            let shortfall = (column - 1).saturating_sub(text.chars().count());
            format!(
                "error: {message}\n{pad}--> {location}:{line}:{column}\n{pad} |\n{number} | {text}\n{pad} | {indent}{}^",
                " ".repeat(shortfall)
            )
        }
        ConfigError::Invalid {
            path: item,
            message,
        } => {
            format!("error: {item}: {message}\n --> {location}")
        }
        ConfigError::Toml { message } => format!("error: {message}\n --> {location}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koha_core::Config;
    use std::path::PathBuf;

    fn path() -> PathBuf {
        PathBuf::from("config.toml")
    }

    fn error_of(source: &str) -> ConfigError {
        Config::from_toml_str(source).unwrap_err()
    }

    #[test]
    fn syntax_error_shows_the_offending_line_and_a_caret() {
        let source = "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"url\"\nurl = \"x\"\nbogus = 1\n";
        let text = render(&path(), source, &error_of(source));
        let lines: Vec<&str> = text.lines().collect();
        assert!(lines[0].starts_with("error: "), "{text}");
        assert!(lines[0].contains("bogus"), "{text}");
        assert_eq!(lines[1], " --> config.toml:7:1");
        assert_eq!(lines[2], "  |");
        assert_eq!(lines[3], "7 | bogus = 1");
        assert_eq!(lines[4], "  | ^");
    }

    #[test]
    fn caret_lines_up_under_the_reported_column() {
        let source = "version = 1\n[[menu]]\nid = \"a\"\nlabel = 5\ntype = \"url\"\nurl = \"x\"\n";
        let text = render(&path(), source, &error_of(source));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[1], " --> config.toml:4:9");
        assert_eq!(lines[3], "4 | label = 5");
        // Kolom 9 = karakter "5"; caret diawali 8 spasi setelah "  | ".
        assert_eq!(lines[4], "  |         ^");
    }

    #[test]
    fn gutter_widens_for_multi_digit_line_numbers() {
        let mut source = String::from(
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"A\"\ntype = \"url\"\nurl = \"x\"\n",
        );
        for _ in 0..8 {
            source.push('\n');
        }
        source.push_str("bogus = 1\n");
        let text = render(&path(), &source, &error_of(&source));
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[1], "  --> config.toml:15:1");
        assert_eq!(lines[3], "15 | bogus = 1");
        assert_eq!(lines[4], "   | ^");
    }

    #[test]
    fn tabs_before_the_column_are_kept_so_the_caret_stays_aligned() {
        let error = ConfigError::Syntax {
            line: 1,
            column: 3,
            message: "boom".to_owned(),
        };
        let text = render(&path(), "\t\tx = 1\n", &error);
        assert!(text.ends_with("\n  | \t\t^"), "{text:?}");
    }

    #[test]
    fn semantic_error_shows_the_item_path() {
        let source =
            "version = 1\n[[menu]]\nid = \"a\"\nlabel = \"\"\ntype = \"url\"\nurl = \"x\"\n";
        let text = render(&path(), source, &error_of(source));
        assert!(
            text.starts_with("error: menu[0]: label tidak boleh kosong"),
            "{text}"
        );
        assert!(text.contains("--> config.toml"), "{text}");
    }

    #[test]
    fn error_without_position_still_names_the_file() {
        let error = ConfigError::Toml {
            message: "kacau".to_owned(),
        };
        assert_eq!(
            render(&path(), "", &error),
            "error: kacau\n --> config.toml"
        );
    }

    #[test]
    fn line_beyond_the_source_does_not_panic() {
        let error = ConfigError::Syntax {
            line: 99,
            column: 5,
            message: "boom".to_owned(),
        };
        let text = render(&path(), "x\n", &error);
        assert!(text.contains("99 | "), "{text}");
    }
}
