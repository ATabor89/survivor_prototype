//! Small RON files in the launch directory: run recipes and the saved profile.
use serde::Serialize;
use std::io::{Read, Write};
use std::path::Path;

/// Player-editable files stay small; refuse anything that is clearly not one.
pub const MAX_FILE_BYTES: usize = 64 * 1024;

/// Read a whole file, rejecting one larger than [`MAX_FILE_BYTES`].
pub fn read_text(path: &Path) -> Result<String, String> {
    let describe = |error: std::io::Error| format!("{}: {error}", path.display());
    let mut text = String::new();
    std::fs::File::open(path)
        .map_err(describe)?
        .take(MAX_FILE_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(describe)?;
    if text.len() > MAX_FILE_BYTES {
        return Err(format!(
            "{} is larger than {} KiB.",
            path.display(),
            MAX_FILE_BYTES / 1024
        ));
    }
    Ok(text)
}

/// Serialize `value` and replace `path` with it.
///
/// The text is written to a temporary file in the same directory and renamed
/// into place, so a failed write leaves the earlier file intact. Rust's rename
/// replaces an existing file on both Windows and Unix.
pub fn write_ron<T: Serialize>(value: &T, path: &Path) -> Result<(), String> {
    let text = ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|error| error.to_string())?;
    let temporary = path.with_extension(format!("ron.{}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("{}: {error}", path.display()))
}
