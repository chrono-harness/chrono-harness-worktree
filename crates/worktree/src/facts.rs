//! Git object and checkout observations consumed by lifecycle operations.
use crate::document::relative_path;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{ErrorKind, Read},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};

pub(crate) const LITERAL_PATHSPEC_CONTROLS: [&str; 4] = [
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];

pub(crate) fn full_oid(oid: &str) -> Result<(), String> {
    if matches!(oid.len(), 40 | 64)
        && oid
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && oid.bytes().any(|b| b != b'0')
    {
        Ok(())
    } else {
        Err("expected full nonzero lowercase commit OID".into())
    }
}

pub(crate) fn commit_parents(bytes: &[u8]) -> Result<Vec<String>, String> {
    bytes
        .split(|b| *b == b'\n')
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.strip_prefix(b"parent "))
        .map(|raw| {
            let oid = std::str::from_utf8(raw).map_err(|_| "invalid parent OID")?;
            full_oid(oid)?;
            Ok(oid.to_owned())
        })
        .collect()
}
pub(crate) fn artifact_directories(config: &Value) -> Result<Vec<&str>, String> {
    config["artifacts"]
        .as_array()
        .ok_or("missing artifact declarations")?
        .iter()
        .map(|item| item["path"].as_str().ok_or("missing artifact path".into()))
        .collect()
}
pub(crate) fn artifact_exclusions(artifacts: &[&str]) -> Vec<String> {
    artifacts
        .iter()
        .filter_map(|prefix| {
            let path = prefix.strip_suffix('/')?;
            (relative_path(path).is_ok()
                && path
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != ".."))
            .then(|| format!(":(top,exclude,literal){prefix}"))
        })
        .collect()
}
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub(crate) mode: String,
    pub(crate) kind: String,
    pub(crate) oid: String,
}
pub type Tree = BTreeMap<String, Entry>;
fn records(raw: &[u8]) -> Result<impl Iterator<Item = &str>, String> {
    if raw.last().is_some_and(|c| *c != 0) {
        return Err("incomplete Git inventory".into());
    }
    let text = std::str::from_utf8(raw).map_err(|_| "non-UTF8 Git inventory")?;
    Ok(text.split_terminator('\0'))
}
fn fields<'a>(line: &'a str, what: &str) -> Result<([&'a str; 3], &'a str), String> {
    let (meta, path) = line
        .split_once('\t')
        .ok_or_else(|| format!("invalid Git {what} row"))?;
    let tokens: Vec<_> = meta.split(' ').filter(|field| !field.is_empty()).collect();
    let tokens = tokens
        .try_into()
        .map_err(|_| format!("invalid Git {what} metadata"))?;
    relative_path(path)?;
    Ok((tokens, path))
}
pub fn parse_tree(raw: &[u8]) -> Result<Tree, String> {
    let mut tree = Tree::new();
    for line in records(raw)? {
        let ([mode, kind, oid], path) = fields(line, "tree")?;
        full_oid(oid)?;
        if tree
            .insert(
                path.to_owned(),
                Entry {
                    mode: mode.into(),
                    kind: kind.into(),
                    oid: oid.into(),
                },
            )
            .is_some()
        {
            return Err("duplicate Git tree path".into());
        }
    }
    Ok(tree)
}
fn blob_matches<D: Digest>(oid: &str, len: u64, mut source: impl Read) -> Result<bool, String> {
    let mut hash = D::new();
    hash.update(format!("blob {len}\0").as_bytes());
    let mut buffer = [0u8; 64 * 1024];
    let mut size = 0u64;
    loop {
        match source.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                size = size
                    .checked_add(n as u64)
                    .ok_or("Git blob length overflow")?;
                hash.update(&buffer[..n]);
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    if len != size {
        return Err("checkout file changed while hashing".into());
    }
    let actual: String = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(actual == oid)
}
fn matches_blob(oid: &str, len: u64, source: impl Read) -> Result<bool, String> {
    match oid.len() {
        40 => blob_matches::<sha1::Sha1>(oid, len, source),
        64 => blob_matches::<Sha256>(oid, len, source),
        _ => Err("invalid Git blob OID".into()),
    }
}
fn physical(root: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let mut path = root.to_owned();
    let mut parts = Path::new(name).components().peekable();
    while let Some(component) = parts.next() {
        path.push(component);
        if parts.peek().is_some() {
            match fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() => (),
                Ok(_) => return Ok(None),
                Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
            }
        }
    }
    Ok(Some(path))
}
fn matches(root: &Path, name: &str, entry: &Entry) -> Result<bool, String> {
    if entry.kind != "blob" || !matches!(entry.mode.as_str(), "100644" | "100755" | "120000") {
        return Ok(false);
    }
    let Some(path) = physical(root, name)? else {
        return Ok(false);
    };
    let before = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    if entry.mode == "120000" {
        if !before.is_symlink() {
            return Ok(false);
        }
        let target = fs::read_link(path).map_err(|e| e.to_string())?;
        let bytes = target.as_os_str().as_bytes();
        return matches_blob(&entry.oid, bytes.len() as u64, bytes);
    }
    if !before.is_file() || (before.mode() & 0o100 != 0) != (entry.mode == "100755") {
        return Ok(false);
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| e.to_string())?;
    let opened = file.metadata().map_err(|e| e.to_string())?;
    if !opened.is_file()
        || (opened.dev(), opened.ino(), opened.len(), opened.mode())
            != (before.dev(), before.ino(), before.len(), before.mode())
    {
        return Err(format!(
            "checkout file identity changed while opening: {name}"
        ));
    }
    matches_blob(&entry.oid, opened.len(), file)
}
pub fn checkout_changes(root: &Path, expected: &Tree, index: &[u8]) -> Result<Vec<String>, String> {
    let mut stages = BTreeSet::new();
    let mut changed = BTreeSet::new();
    for row in records(index)? {
        let ([mode, oid, stage], path) = fields(row, "index")?;
        full_oid(oid)?;
        if !matches!(stage, "0" | "1" | "2" | "3") {
            return Err("invalid Git index stage".into());
        }
        if !stages.insert((path, stage)) {
            return Err("duplicate Git index stage".into());
        }
        if stage != "0"
            || expected
                .get(path)
                .is_none_or(|entry| entry.mode != mode || entry.oid != oid)
        {
            changed.insert(path.to_owned());
        }
    }
    for (path, entry) in expected {
        if !stages.contains(&(path.as_str(), "0")) || !matches(root, path, entry)? {
            changed.insert(path.clone());
        }
    }
    Ok(changed.into_iter().collect())
}

/// The ls-tree, cat-file and index transports use the host's bound Git executable.
/// Verify immutable bytes against the Git object named in the original metadata.
struct Blob {
    oid: String,
    length: usize,
}
impl Blob {
    fn metadata(line: &str, hash_width: usize) -> Result<Self, String> {
        let fields = line.split(' ').collect::<Vec<_>>();
        let [oid, "blob", size] = fields.as_slice() else {
            return Err("expected Git blob metadata".into());
        };
        full_oid(oid)?;
        let length = size
            .parse::<usize>()
            .map_err(|_| "invalid blob byte count")?;
        if oid.len() != hash_width || size != &length.to_string() {
            return Err("invalid Git object metadata width or byte count".into());
        }
        Ok(Self {
            oid: (*oid).into(),
            length,
        })
    }
    fn header(&self) -> String {
        format!("{} blob {}\n", self.oid, self.length)
    }
    fn verify(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() != self.length || !matches_blob(&self.oid, self.length as u64, bytes)? {
            return Err("Git blob content differs from metadata".into());
        }
        Ok(())
    }
}

pub fn acquire_immutable_input(
    oid: &str,
    path: &str,
    limit: usize,
    mut git: impl FnMut(&[&str], &[u8], Option<usize>) -> Result<Vec<u8>, String>,
) -> Result<Vec<u8>, String> {
    full_oid(oid)?;
    relative_path(path)?;
    if !(1..=64 * 1024 * 1024).contains(&limit) {
        return Err("invalid immutable input limit".into());
    }
    let input = format!("{oid}:{path}\0");
    let bytes = git(&["cat-file", "--batch-check", "-z"], input.as_bytes(), None)?;
    let meta = std::str::from_utf8(&bytes).map_err(|_| "invalid Git blob metadata")?;
    let blob = Blob::metadata(
        meta.strip_suffix('\n')
            .ok_or("incomplete Git blob metadata")?,
        oid.len(),
    )?;
    if blob.length > limit {
        return Err(format!(
            "immutable input {path} has {} bytes; declared limit is {limit}",
            blob.length
        ));
    }
    let bytes = git(
        &["cat-file", "blob", &blob.oid],
        &[],
        Some(blob.length.max(1)),
    )?;
    blob.verify(&bytes)?;
    Ok(bytes)
}

/// Use independent show calls for very small sets. Otherwise partition complete
/// cat-file frames under the same stdout bound as the caller's original Git observations.
pub fn acquire_registry_blobs(
    oid: &str,
    paths: &[String],
    limit: usize,
    mut git: impl FnMut(&[&str], &[u8]) -> Result<Vec<u8>, String>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    full_oid(oid)?;
    let mut seen = BTreeSet::new();
    for path in paths {
        relative_path(path)?;
        if !seen.insert(path) {
            return Err("duplicate registered blob path".into());
        }
    }
    if paths.len() < 3 || paths.len().saturating_mul(91) > limit {
        return paths
            .iter()
            .map(|path| Ok((path.clone(), git(&["show", &format!("{oid}:{path}")], &[])?)))
            .collect();
    }
    let input = paths
        .iter()
        .map(|path| format!("{oid}:{path}\n"))
        .collect::<String>();
    let bytes = git(&["cat-file", "--batch-check"], input.as_bytes())?;
    let metadata = std::str::from_utf8(&bytes).map_err(|_| "invalid Git batch metadata")?;
    let blobs = metadata
        .strip_suffix('\n')
        .ok_or("incomplete Git batch metadata")?
        .split('\n')
        .map(|line| Blob::metadata(line, oid.len()))
        .collect::<Result<Vec<_>, _>>()?;
    if blobs.len() != paths.len() {
        return Err("Git batch metadata count differs from requested paths".into());
    }
    let mut result = Vec::with_capacity(paths.len());
    while result.len() < paths.len() {
        let first = result.len();
        let mut budget = limit;
        let count = blobs[first..]
            .iter()
            .take_while(|blob| {
                let Some(remaining) = budget
                    .checked_sub(blob.header().len() + 1)
                    .and_then(|left| left.checked_sub(blob.length))
                else {
                    return false;
                };
                budget = remaining;
                true
            })
            .count();
        if count < 2 {
            let path = &paths[first];
            let bytes = git(&["show", &format!("{oid}:{path}")], &[])?;
            blobs[first].verify(&bytes)?;
            result.push((path.clone(), bytes));
            continue;
        }
        let end = first + count;
        let input = blobs[first..end]
            .iter()
            .map(|blob| format!("{}\n", blob.oid))
            .collect::<String>();
        let original = git(&["cat-file", "--batch"], input.as_bytes())?;
        let mut cursor = original.as_slice();
        for (blob, path) in blobs[first..end].iter().zip(&paths[first..end]) {
            let body = cursor
                .strip_prefix(blob.header().as_bytes())
                .ok_or("Git batch header differs")?;
            let (bytes, tail) = body
                .split_at_checked(blob.length)
                .ok_or("truncated Git batch body")?;
            cursor = tail
                .strip_prefix(b"\n")
                .ok_or("missing Git batch terminator")?;
            blob.verify(bytes)?;
            result.push((path.clone(), bytes.to_vec()));
        }
        if !cursor.is_empty() {
            return Err("trailing Git batch bytes".into());
        }
    }
    Ok(result)
}
