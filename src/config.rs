use std::path::Path;

use anyhow::Result;
use ini::{EscapePolicy, Ini, LineSeparator, ParseOption, WriteOption};

fn load_text_lossy<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let bytes = std::fs::read(path.as_ref())?;

    // check for UTF-16LE BOM
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        let (_, utf_16, _) = unsafe { bytes[2..].align_to::<u16>() };
        Ok(String::from_utf16_lossy(utf_16))
    } else {
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
}

pub fn load_ini<P: AsRef<Path>>(path: P) -> Result<Ini> {
    let text = load_text_lossy(path)?;
    Ok(Ini::load_from_str_opt(
        &text,
        ParseOption {
            enabled_escape: false,
            enabled_quote: false,
        },
    )?)
}

pub fn save_ini<P: AsRef<Path>>(ini: &Ini, path: P) -> Result<()> {
    Ok(ini.write_to_file_opt(
        path,
        WriteOption {
            escape_policy: EscapePolicy::Nothing,
            line_separator: LineSeparator::SystemDefault,
        },
    )?)
}
