use serde::Serialize;
use sha2::{Digest, Sha256};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Canonical JSON is part of the on-disk worktree registration and receipt identity.
pub(crate) fn digest<T: Serialize>(value: &T) -> Result<String, String> {
    serde_jcs::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|e| e.to_string())
}

use serde::{
    Deserialize, Deserializer,
    de::{DeserializeOwned, Error},
};
use serde_json::Value;
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

pub(crate) fn strict_json_value<'de, D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
    let strict = serde_yaml_ng::Value::deserialize(d).map_err(|error| {
        let message = error.to_string();
        if let Some((quoted, location)) = message
            .strip_prefix("duplicate entry with key ")
            .and_then(|tail| tail.rsplit_once(" at line "))
            && let Ok(key) = litrs::StringLit::parse(quoted)
        {
            return D::Error::custom(format!(
                "duplicate JSON member {} at line {location}",
                key.value()
            ));
        }
        error
    })?;
    serde_json::to_value(strict).map_err(D::Error::custom)
}

pub(crate) fn json(bytes: &[u8]) -> Result<Value, String> {
    let mut input = serde_json::Deserializer::from_slice(bytes);
    let value = strict_json_value(&mut input).map_err(|error| error.to_string())?;
    input.end().map_err(|error| error.to_string())?;
    Ok(value)
}
pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_value(json(bytes)?).map_err(|error| error.to_string())
}
pub(crate) fn valid_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.contains(['\0', '\n', '\r', '\\'])
        || Path::new(value)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("invalid relative path: {value:?}"));
    }
    Ok(())
}
pub(crate) fn no_symlink_parents(root: &Path, relative: &str) -> Result<PathBuf, String> {
    relative_path(relative)?;
    let mut current = root.to_owned();
    for part in Path::new(relative).components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_symlink() => {
                return Err(format!(
                    "symlink path is not allowed: {}",
                    current.display()
                ));
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.to_string()),
            _ => {}
        }
    }
    Ok(current)
}
pub(crate) fn copy_hashed(
    mut input: impl Read,
    mut output: impl Write,
) -> Result<(String, u64), String> {
    let mut digest = Sha256::new();
    let mut length = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let n = match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.to_string()),
        };
        output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        digest.update(&buffer[..n]);
        length = length.checked_add(n as u64).ok_or("file length overflow")?;
    }
    Ok((format!("{:x}", digest.finalize()), length))
}
pub(crate) fn file_identity(path: &Path) -> Result<(String, u64), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("not a regular input file: {}", path.display()));
    }
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file() || (metadata.dev(), metadata.ino()) != (opened.dev(), opened.ino()) {
        return Err(format!(
            "input file changed before open: {}",
            path.display()
        ));
    }
    copy_hashed(file, io::sink())
}
