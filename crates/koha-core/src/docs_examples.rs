//! Menjaga dokumentasi tetap sesuai dengan kode. Hanya dikompilasi di tes.
//!
//! - Setiap blok ```` ```toml ```` di `README.md` dan `docs/configuration.md` harus berupa
//!   config yang valid; blok ```` ```toml state ```` harus berupa state yang valid. Potongan
//!   yang bukan config lengkap ditulis sebagai ```` ```text ````.
//! - Dokumen konfigurasi harus menyebut semua aksi bawaan, tema, dan font.
//! - Semua tautan relatif di berkas `.md` harus menunjuk berkas yang ada.
//!
//! Bila salah satunya gagal setelah kamu mengubah kode, dokumennya yang perlu diperbarui.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Builtin, Config};
use crate::state::State;
use crate::theme::{FONT_IBM_PLEX_MONO, FONT_PRESS_START_2P, FONT_VT323, ThemeSet};

// `include_str!` membaca berkas saat kompilasi: menghapus atau mengganti nama berkasnya
// membuat tes ini gagal dikompilasi, bukan lolos diam-diam.
const README: &str = include_str!("../../../README.md");
const CONFIGURATION: &str = include_str!("../../../docs/configuration.md");

/// Satu blok kode berpagar di dalam markdown.
struct Block {
    /// Teks setelah ``` pembuka, misalnya `toml` atau `toml state`.
    info: String,
    body: String,
    /// Nomor baris (mulai dari 1) tempat blok dibuka.
    line: usize,
}

fn fenced_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current: Option<Block> = None;
    for (index, line) in markdown.lines().enumerate() {
        if let Some(info) = line.trim_start().strip_prefix("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => {
                    current = Some(Block {
                        info: info.trim().to_owned(),
                        body: String::new(),
                        line: index + 1,
                    });
                }
            }
        } else if let Some(block) = current.as_mut() {
            block.body.push_str(line);
            block.body.push('\n');
        }
    }
    blocks
}

fn blocks_with_info<'a>(blocks: &'a [Block], info: &str) -> Vec<&'a Block> {
    blocks.iter().filter(|block| block.info == info).collect()
}

/// Markdown tanpa isi blok kode, supaya `](` di dalam contoh kode tidak terbaca sebagai tautan.
fn without_code_blocks(markdown: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
        } else if !inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Target semua tautan dan gambar inline `[teks](target)`.
fn link_targets(markdown: &str) -> Vec<String> {
    let text = without_code_blocks(markdown);
    let mut targets = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else { break };
        targets.push(after[..end].trim().to_owned());
        rest = &after[end + 1..];
    }
    targets
}

/// Tautan relatif di `markdown` (yang ada di `file`) yang tidak menunjuk berkas yang ada.
/// Tautan web, `mailto:`, dan jangkar di halaman yang sama dilewati; pecahan (`#...`) dan
/// kueri dibuang sebelum diperiksa. Jangkar di berkas lain tidak diperiksa.
fn missing_links(file: &Path, markdown: &str) -> Vec<String> {
    let dir = file.parent().expect("berkas markdown punya folder induk");
    link_targets(markdown)
        .into_iter()
        .filter(|target| {
            !(target.is_empty()
                || target.starts_with('#')
                || target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:"))
        })
        .filter(|target| {
            let path = target.trim_matches(['<', '>']);
            let path = path.split(['#', '?']).next().unwrap_or("");
            !path.is_empty() && !dir.join(path).exists()
        })
        .collect()
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Berkas markdown yang ditulis tangan. `THIRD_PARTY_LICENSES.md` dihasilkan otomatis dan
/// dilewati.
fn handwritten_markdown_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut files: Vec<PathBuf> = ["README.md", "CONTRIBUTING.md", "CHANGELOG.md"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    for dir in ["docs", "tools"] {
        for entry in fs::read_dir(root.join(dir)).expect("folder dokumentasi bisa dibaca") {
            let path = entry.expect("entri folder bisa dibaca").path();
            if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

// ---- pemeriksa itu sendiri ----

#[test]
fn fenced_blocks_are_extracted_with_their_info_string_and_line() {
    let markdown =
        "teks\n\n```toml\na = 1\n```\n\n  ```text\nx\n  ```\n```toml state\nb = 2\n```\n";
    let blocks = fenced_blocks(markdown);
    assert_eq!(blocks.len(), 3);
    assert_eq!((blocks[0].info.as_str(), blocks[0].line), ("toml", 3));
    assert_eq!(blocks[0].body, "a = 1\n");
    assert_eq!(blocks[1].info, "text");
    assert_eq!(blocks[2].info, "toml state");
    assert_eq!(blocks_with_info(&blocks, "toml").len(), 1);
}

#[test]
fn link_targets_ignore_links_inside_code_blocks() {
    let markdown = "[a](x.md) dan ![gambar](img/y.png)\n```text\n[tidak](bukan-tautan.md)\n```\n[b](z.md#bagian)\n";
    assert_eq!(link_targets(markdown), ["x.md", "img/y.png", "z.md#bagian"]);
}

#[test]
fn link_checker_reports_missing_files_and_skips_web_and_anchor_links() {
    let readme = repo_root().join("README.md");
    let markdown = "[ada](docs/platforms.md) [hilang](docs/tidak-ada.md) [web](https://example.com) \
                    [surel](mailto:a@b.c) [jangkar](#bagian) [ada+jangkar](docs/platforms.md#x) \
                    [hilang+jangkar](nope.md#x) ![g](docs/images/hilang.png)";
    assert_eq!(
        missing_links(&readme, markdown),
        ["docs/tidak-ada.md", "nope.md#x", "docs/images/hilang.png"]
    );
}

// ---- dokumentasi sungguhan ----

#[test]
fn configuration_toml_examples_are_valid_configs() {
    let blocks = fenced_blocks(CONFIGURATION);
    let examples = blocks_with_info(&blocks, "toml");
    // Batas bawah mencegah tes lolos kosong bila blok tidak sengaja berganti jenis.
    assert!(examples.len() >= 8, "hanya {} blok toml", examples.len());
    for block in examples {
        if let Err(error) = Config::from_toml_str(&block.body) {
            panic!(
                "docs/configuration.md baris {}: config contoh tidak valid: {error}",
                block.line
            );
        }
    }
}

#[test]
fn configuration_state_examples_are_valid_states() {
    let blocks = fenced_blocks(CONFIGURATION);
    let examples = blocks_with_info(&blocks, "toml state");
    assert!(!examples.is_empty(), "tidak ada blok `toml state`");
    for block in examples {
        if let Err(error) = State::from_toml_str(&block.body) {
            panic!(
                "docs/configuration.md baris {}: state contoh tidak valid: {error}",
                block.line
            );
        }
    }
}

#[test]
fn readme_toml_examples_are_valid_configs() {
    let blocks = fenced_blocks(README);
    let examples = blocks_with_info(&blocks, "toml");
    assert!(!examples.is_empty(), "README tidak punya contoh toml");
    for block in examples {
        if let Err(error) = Config::from_toml_str(&block.body) {
            panic!(
                "README.md baris {}: config contoh tidak valid: {error}",
                block.line
            );
        }
    }
}

#[test]
fn configuration_doc_mentions_every_builtin_theme_and_font() {
    for name in Builtin::NAMES {
        assert!(
            CONFIGURATION.contains(&format!("`{name}`")),
            "aksi bawaan `{name}` tidak disebut"
        );
    }
    for theme in ThemeSet::builtin().iter() {
        assert!(
            CONFIGURATION.contains(&format!("`{}`", theme.name)),
            "tema `{}` tidak disebut",
            theme.name
        );
    }
    for font in [FONT_PRESS_START_2P, FONT_VT323, FONT_IBM_PLEX_MONO] {
        assert!(
            CONFIGURATION.contains(&format!("`{font}`")),
            "font `{font}` tidak disebut"
        );
    }
}

#[test]
fn all_relative_markdown_links_resolve() {
    let mut problems = Vec::new();
    for file in handwritten_markdown_files() {
        let markdown = fs::read_to_string(&file).expect("berkas markdown bisa dibaca");
        for target in missing_links(&file, &markdown) {
            problems.push(format!("{}: tautan rusak -> {target}", file.display()));
        }
    }
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}

#[test]
fn the_expected_handwritten_markdown_files_are_all_checked() {
    let names: Vec<String> = handwritten_markdown_files()
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    for expected in [
        "README.md",
        "CONTRIBUTING.md",
        "CHANGELOG.md",
        "configuration.md",
        "performance.md",
        "platforms.md",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "{expected} tidak ikut diperiksa"
        );
    }
}
