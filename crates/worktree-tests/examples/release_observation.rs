//! Release acceptance for the version for the admitted tag and public help text.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use vstd::prelude::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {
pub fn program() -> (value: &'static str)
    ensures value@ == "chrono-worktree"@,
{ "chrono-worktree" }

pub fn help_fragment() -> (value: &'static str)
    ensures value@ == "chrono-worktree start"@,
{ "chrono-worktree start" }

pub fn accepts(version_matches: bool, help_matches: bool) -> (accepted: bool)
    ensures accepted == (version_matches && help_matches),
{
    version_matches && help_matches
}
}

#[cfg(not(verus_keep_ghost))]
fn grep(mode: &str, pattern: &str, path: &str) -> Result<bool, String> {
    // grep interprets its documented matching mode and reports the original matching lines.
    std::process::Command::new("grep")
        .args([mode, pattern, path])
        .status()
        .map(|result| result.success())
        .map_err(|error| error.to_string())
}

#[cfg(not(verus_keep_ghost))]
fn main() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    let [tag, version, help_file] = arguments.as_slice() else {
        return Err("usage: release_observation TAG VERSION HELP_FILE".into());
    };
    // The release source already admitted this tag. Preserve the original shell's byte delimiters.
    let core = tag.strip_prefix('v').unwrap_or(tag);
    let core = core.split_once('-').map_or(core, |(version, _)| version);
    let version_matches = version == &format!("{} {core}", program());
    let help_matches = grep("-F", help_fragment(), help_file)?;
    let admitted = accepts(version_matches, help_matches);
    if admitted {
        Ok(())
    } else {
        Err(format!("unexpected release observation for {arguments:?}"))
    }
}
