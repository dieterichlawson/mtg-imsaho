//! Guards on what the test suite's own comments claim.
//!
//! Test documentation rots quietly: it is never compiled, never run, and never
//! reviewed against the thing it describes. Two kinds of rot had accumulated
//! badly enough to be worth failing the build over.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is <repo>/mtg-engine.
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            walk(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every `.rs` file in the workspace, by file name.
fn source_file_names() -> BTreeSet<String> {
    let mut files = Vec::new();
    walk(&repo_root(), &mut files);
    files
        .iter()
        .filter_map(|p| p.file_name()?.to_str().map(str::to_string))
        .collect()
}

/// Every test file except this one — the guards below quote the patterns they
/// forbid, so scanning themselves would always fail.
fn test_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"), &mut files);
    files.retain(|p| p.file_name().is_none_or(|n| n != "doc_references.rs"));
    files.sort();
    files
}

/// A comment that points at `foo.rs` must point at a file that exists.
///
/// These references used to carry line numbers as well (`engine.rs:4085`), and
/// 19 of the 77 already pointed past the end of the file they named — one of
/// them 3000 lines past it. A line number is stale the moment anyone edits the
/// file above it, so the numbers are gone; the file names are worth keeping,
/// and worth checking.
#[test]
fn every_source_file_a_test_comment_names_exists() {
    let known = source_file_names();
    let re = regex_lite_rs_backtick();
    let mut offenders = Vec::new();

    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                continue;
            }
            for name in re(line) {
                if !known.contains(&name) {
                    offenders.push(format!(
                        "{}:{}: names `{name}`, which is not a file in this workspace",
                        path.file_name().unwrap().to_string_lossy(),
                        n + 1
                    ));
                }
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} stale file reference(s) in test comments:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// No test may claim it is expected to fail.
///
/// The suite had accumulated 62 comments saying "This test asserts the EXPECTED
/// CORRECT behavior, so it currently fails. It will start passing as soon as
/// Bug X is fixed." Every one of them was passing — the bugs had been fixed and
/// nobody went back to the comment. A reader who believes them mistrusts a
/// green suite, which is worse than having no comment at all.
///
/// A test that genuinely should not pass yet belongs behind `#[ignore]`, where
/// the runner reports it, rather than in prose the runner cannot see.
#[test]
fn no_test_comment_claims_the_test_is_failing() {
    const STALE: &[&str] = &[
        "EXPECTED CORRECT behavior",
        "currently fails",
        "will start passing",
        "this test should fail",
    ];
    let mut offenders = Vec::new();

    for path in test_files() {
        let Ok(text) = fs::read_to_string(&path) else { continue };
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                continue;
            }
            let lower = line.to_lowercase();
            for needle in STALE {
                if lower.contains(&needle.to_lowercase()) {
                    offenders.push(format!(
                        "{}:{}: {}",
                        path.file_name().unwrap().to_string_lossy(),
                        n + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(offenders.is_empty(),
        "{} comment(s) claim a passing test is failing:\n  {}",
        offenders.len(), offenders.join("\n  "));
}

/// Pull `` `foo.rs` `` references out of a comment line. Backticks only: an
/// unquoted `.rs` in prose is usually part of a path being described rather
/// than a reference to a file in this tree.
fn regex_lite_rs_backtick() -> impl Fn(&str) -> Vec<String> {
    |line: &str| {
        let mut out = Vec::new();
        let bytes: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != '`' {
                i += 1;
                continue;
            }
            let Some(end) = bytes[i + 1..].iter().position(|c| *c == '`') else { break };
            let inner: String = bytes[i + 1..i + 1 + end].iter().collect();
            i += end + 2;
            if !inner.ends_with(".rs") {
                continue;
            }
            // `audits/AUDIT_BUGS.md)` style paths are handled by the .rs check;
            // take the last path segment as the file name.
            let name = inner.rsplit('/').next().unwrap_or(&inner).to_string();
            if name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                out.push(name);
            }
        }
        out
    }
}
