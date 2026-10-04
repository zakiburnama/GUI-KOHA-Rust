//! Pengutipan argumen baris perintah dan validasi URL.
//!
//! Murni (tanpa panggilan OS) dan dikompilasi di semua OS, supaya aturannya
//! bisa dites di mana pun walau hanya Windows yang memakainya.

use crate::PlatformError;

/// Mengutip satu argumen mengikuti aturan penguraian baris perintah Windows
/// (`CommandLineToArgvW`), sehingga program penerima membaca persis string yang
/// sama.
///
/// Aturannya: argumen tanpa spasi, tab, atau tanda kutip dibiarkan apa adanya;
/// selain itu dibungkus `"..."`. Di dalam bungkusan, tanda kutip diberi
/// backslash, dan backslash yang mendahului tanda kutip (atau penutup bungkusan)
/// digandakan. Backslash di tempat lain tidak diubah.
pub fn quote_arg(arg: &str) -> String {
    let needs_quotes = arg.is_empty() || arg.contains([' ', '\t', '\n', '\u{b}', '"']);
    if !needs_quotes {
        return arg.to_owned();
    }

    let mut quoted = String::with_capacity(arg.len() + 2);
    quoted.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                // Gandakan backslash di depannya, lalu escape tanda kutipnya.
                quoted.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            other => {
                quoted.extend(std::iter::repeat_n('\\', backslashes));
                backslashes = 0;
                quoted.push(other);
            }
        }
    }
    // Backslash di ujung akan bertemu tanda kutip penutup, jadi digandakan.
    quoted.extend(std::iter::repeat_n('\\', backslashes * 2));
    quoted.push('"');
    quoted
}

/// Menggabungkan argumen menjadi satu string parameter, dipisah spasi.
pub fn join_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| quote_arg(arg))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Memastikan `url` diawali skema (`https:`, `mailto:`, `ms-settings:`, ...).
///
/// `ShellExecute` menjalankan apa saja yang diberikan, termasuk jalur berkas
/// dan program, jadi teks yang bukan URL ditolak. Skema minimal dua karakter
/// supaya `C:\folder` (huruf drive) tidak terbaca sebagai skema.
pub fn validate_url(url: &str) -> Result<&str, PlatformError> {
    let url = url.trim();
    let invalid = || PlatformError::InvalidUrl(url.to_owned());

    if url.chars().any(char::is_control) {
        return Err(invalid());
    }
    let (scheme, rest) = url.split_once(':').ok_or_else(invalid)?;
    let mut chars = scheme.chars();
    let valid_scheme = scheme.len() >= 2
        && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    if valid_scheme && !rest.is_empty() {
        Ok(url)
    } else {
        Err(invalid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_arguments_are_left_alone() {
        assert_eq!(quote_arg("abc"), "abc");
        assert_eq!(quote_arg("--flag=value"), "--flag=value");
        assert_eq!(quote_arg(r"C:\dir\file.txt"), r"C:\dir\file.txt");
    }

    #[test]
    fn trailing_backslash_without_spaces_is_not_quoted() {
        assert_eq!(quote_arg(r"C:\dir\"), r"C:\dir\");
    }

    #[test]
    fn empty_argument_becomes_an_empty_quoted_string() {
        assert_eq!(quote_arg(""), r#""""#);
    }

    #[test]
    fn spaces_and_tabs_trigger_quoting() {
        assert_eq!(quote_arg("a b"), r#""a b""#);
        assert_eq!(quote_arg("a\tb"), "\"a\tb\"");
    }

    #[test]
    fn embedded_quotes_are_escaped() {
        assert_eq!(quote_arg(r#"a"b"#), r#""a\"b""#);
    }

    #[test]
    fn backslashes_before_a_quote_are_doubled_plus_one() {
        // a\"b  (satu backslash lalu kutip) -> a\\\"b di dalam bungkusan.
        assert_eq!(quote_arg(r#"a\"b"#), r#""a\\\"b""#);
    }

    #[test]
    fn trailing_backslashes_in_a_quoted_argument_are_doubled() {
        assert_eq!(quote_arg(r"a b\"), r#""a b\\""#);
        assert_eq!(quote_arg(r"a b\\"), r#""a b\\\\""#);
    }

    #[test]
    fn inner_backslashes_are_not_changed() {
        assert_eq!(quote_arg(r"a\b c"), r#""a\b c""#);
    }

    #[test]
    fn join_separates_arguments_with_spaces() {
        let args = ["-d".to_owned(), r"C:\My Dir".to_owned(), String::new()];
        assert_eq!(join_args(&args), r#"-d "C:\My Dir" """#);
        assert_eq!(join_args(&[]), "");
    }

    /// Meniru `CommandLineToArgvW` untuk kasus yang diuji: membuktikan bahwa
    /// hasil kutipan dibaca kembali menjadi argumen aslinya.
    fn parse_one(quoted: &str) -> String {
        let mut out = String::new();
        let mut chars = quoted.chars().peekable();
        let mut in_quotes = false;
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    let mut count = 1;
                    while chars.peek() == Some(&'\\') {
                        chars.next();
                        count += 1;
                    }
                    if chars.peek() == Some(&'"') {
                        out.extend(std::iter::repeat_n('\\', count / 2));
                        if count % 2 == 1 {
                            out.push('"');
                            chars.next();
                        }
                    } else {
                        out.extend(std::iter::repeat_n('\\', count));
                    }
                }
                '"' => in_quotes = !in_quotes,
                other => out.push(other),
            }
        }
        assert!(!in_quotes, "tanda kutip tidak seimbang: {quoted}");
        out
    }

    #[test]
    fn quoting_roundtrips_through_windows_parsing_rules() {
        for arg in [
            "plain",
            "with space",
            r"C:\Program Files\App\",
            r#"say "hi" there"#,
            r#"\"#,
            r#"\\"#,
            r#"\""#,
            r#"a\\"b c\"#,
            "",
            "tab\there",
        ] {
            assert_eq!(parse_one(&quote_arg(arg)), arg, "argumen {arg:?}");
        }
    }

    #[test]
    fn real_urls_are_accepted() {
        for url in [
            "https://example.com",
            "http://localhost:8080/path?q=1",
            "mailto:someone@example.com",
            "ms-settings:display",
            "obsidian://open?vault=x",
        ] {
            assert_eq!(validate_url(url), Ok(url), "{url}");
        }
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(validate_url("  https://x.dev \n"), Ok("https://x.dev"));
    }

    #[test]
    fn text_that_is_not_a_url_is_rejected() {
        for url in [
            "",
            "   ",
            "calc.exe",
            r"C:\Windows\notepad.exe",
            "C:/Windows",
            "no scheme here",
            "https:",
            ":nothing",
            "1http://x",
            "ht tp://x",
            "https://x\nsecond",
        ] {
            assert!(validate_url(url).is_err(), "{url:?} seharusnya ditolak");
        }
    }
}
