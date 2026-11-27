//! Git object and checkout observations consumed by lifecycle operations.
use vstd::{contrib::exec_spec::ExecSpecEq, prelude::*};

verus! {
pub open spec fn lowercase_hex(b: u8) -> bool {
    (48 <= b <= 57) || (97 <= b <= 102)
}

/// The byte alphabet is common to Git object IDs and publication digests.
pub(crate) fn is_lowercase_hex(b: u8) -> (valid: bool)
    ensures valid == lowercase_hex(b),
{ (48 <= b && b <= 57) || (97 <= b && b <= 102) }

pub(crate) fn valid_git_oid(bytes: &[u8]) -> (valid: bool)
    ensures valid == ((bytes.len() == 40 || bytes.len() == 64)
        && (forall|i: int| 0 <= i < bytes.len() ==> lowercase_hex(#[trigger] bytes@[i]))
        && (exists|i: int| 0 <= i < bytes.len() && #[trigger] bytes@[i] != 48)),
{
    if bytes.len() != 40 && bytes.len() != 64 { return false; }
    let mut nonzero = false;
    let mut i = 0;
    while i < bytes.len()
        invariant
            i <= bytes.len(),
            forall|j: int| 0 <= j < i ==> lowercase_hex(#[trigger] bytes@[j]),
            nonzero == (exists|j: int| 0 <= j < i && #[trigger] bytes@[j] != 48),
        decreases bytes.len() - i,
    {
        let b = bytes[i];
        if !is_lowercase_hex(b) { return false; }
        nonzero = nonzero || b != 48;
        i += 1;
    }
    nonzero
}

pub(crate) enum ParentLine<'a> { End, Other, Parent(&'a [u8]) }

pub(crate) open spec fn has_parent_prefix(line: Seq<u8>) -> bool {
    line.len() >= 7 && line.subrange(0, 7) == seq![112u8, 97, 114, 101, 110, 116, 32]
}

pub(crate) fn parent_header(line: &[u8]) -> (result: ParentLine<'_>)
    ensures
        (result is End) == (line.len() == 0),
        (result is Parent) == has_parent_prefix(line@),
        result is Parent ==> result->Parent_0@ == line@.subrange(7, line.len() as int),
        (result is Other) == (line.len() > 0 && !has_parent_prefix(line@)),
{
    if line.is_empty() { return ParentLine::End; }
    let keyword = ['p' as u8, 'a' as u8, 'r' as u8, 'e' as u8, 'n' as u8, 't' as u8, ' ' as u8];
    let prefix = keyword.as_slice();
    if line.len() >= prefix.len() {
        let (head, rest) = line.split_at(prefix.len());
        proof {
            assert(head.deep_view() =~= head@);
            assert(prefix.deep_view() =~= prefix@);
        }
        if <&[u8]>::exec_eq(head, prefix) { return ParentLine::Parent(rest); }
    }
    ParentLine::Other
}
}

#[cfg(not(verus_keep_ghost))]
use crate::document::relative_path;
#[cfg(not(verus_keep_ghost))]
use serde_json::Value;
#[cfg(not(verus_keep_ghost))]
use sha2::{Digest, Sha256};
#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
pub(crate) const LITERAL_PATHSPEC_CONTROLS: [&str; 4] = [
    "GIT_LITERAL_PATHSPECS",
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];

#[cfg(not(verus_keep_ghost))]
pub(crate) fn full_oid(oid: &str) -> Result<(), String> {
    if valid_git_oid(oid.as_bytes()) {
        Ok(())
    } else {
        Err("expected full nonzero lowercase commit OID".into())
    }
}

#[cfg(not(verus_keep_ghost))]
pub(crate) fn commit_parents(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut parents = Vec::new();
    for line in bytes.split(|b| *b == b'\n') {
        match parent_header(line) {
            ParentLine::End => break,
            ParentLine::Other => continue,
            ParentLine::Parent(raw) => {
                let oid = std::str::from_utf8(raw).map_err(|_| "invalid parent OID")?;
                full_oid(oid)?;
                parents.push(oid.to_owned());
            }
        }
    }
    Ok(parents)
}
#[cfg(not(verus_keep_ghost))]
pub(crate) fn artifact_directories(config: &Value) -> Result<Vec<&str>, String> {
    config["artifacts"]
        .as_array()
        .ok_or("missing artifact declarations")?
        .iter()
        .map(|item| item["path"].as_str().ok_or("missing artifact path".into()))
        .collect()
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub(crate) mode: String,
    pub(crate) kind: String,
    pub(crate) oid: String,
}
#[cfg(not(verus_keep_ghost))]
pub type Tree = BTreeMap<String, Entry>;
#[cfg(not(verus_keep_ghost))]
fn records(raw: &[u8]) -> Result<impl Iterator<Item = &str>, String> {
    if raw.last().is_some_and(|c| *c != 0) {
        return Err("incomplete Git inventory".into());
    }
    let text = std::str::from_utf8(raw).map_err(|_| "non-UTF8 Git inventory")?;
    Ok(text.split_terminator('\0'))
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
fn matches_blob(oid: &str, len: u64, source: impl Read) -> Result<bool, String> {
    match oid.len() {
        40 => blob_matches::<sha1::Sha1>(oid, len, source),
        64 => blob_matches::<Sha256>(oid, len, source),
        _ => Err("invalid Git blob OID".into()),
    }
}
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
pub fn checkout_changes(root: &Path, expected: &Tree, index: &[u8]) -> Result<Vec<String>, String> {
    let mut stages = BTreeSet::new();
    let mut changed = BTreeSet::new();
    for row in records(index)? {
        let ([mode, oid, stage], path) = fields(row, "index")?;
        full_oid(oid)?;
        let tree = expected
            .get(path)
            .map(|entry| (entry.mode.as_str(), entry.oid.as_str()));
        let differs = crate::verified_lifecycle::index_entry_changed(stage, mode, oid, tree)?;
        if !stages.insert((path, stage)) {
            return Err("duplicate Git index stage".into());
        }
        if differs {
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
#[cfg(not(verus_keep_ghost))]
struct Blob {
    oid: String,
    length: usize,
}
#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
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
#[cfg(not(verus_keep_ghost))]
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
