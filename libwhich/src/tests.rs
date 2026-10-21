//! The logic surface, with the directories handed in: nothing here touches the
//! environment except the two smoke tests that read `PATH`, which only read it.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::util;

/// A directory holding an executable `name` (on Windows, an `.exe` is executable by
/// extension alone).
fn dir_with_executable(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(name);
    fs::write(&path, "#!/bin/sh\necho hi\n").expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    (dir, path)
}

#[cfg(windows)]
const BIN: &str = "my_test_bin.exe";
#[cfg(not(windows))]
const BIN: &str = "my_test_bin";

fn found(names: &[&str], dirs: &[&Path]) -> Vec<PathBuf> {
    which_in(names, dirs.iter().map(|d| d.to_path_buf())).collect()
}

#[test]
fn finds_an_executable_in_a_given_dir() {
    let (dir, path) = dir_with_executable(BIN);
    let results = found(&["my_test_bin"], &[dir.path()]);
    assert_eq!(results, vec![path.canonicalize().unwrap()]);
}

#[test]
fn results_are_canonical_absolute_paths() {
    let (dir, path) = dir_with_executable(BIN);
    let link_dir = tempfile::tempdir().unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&path, link_dir.path().join(BIN)).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&path, link_dir.path().join(BIN)).unwrap();

    let results = found(&["my_test_bin"], &[link_dir.path()]);
    assert_eq!(results, vec![path.canonicalize().unwrap()]);
    assert!(results[0].is_absolute());
    let _ = dir;
}

#[test]
fn results_are_grouped_by_name_then_in_dir_order() {
    let (first, first_bin) = dir_with_executable(BIN);
    let (second, second_bin) = dir_with_executable(BIN);
    let (other, other_bin) = dir_with_executable(BIN);
    let other_name = if cfg!(windows) { "other.exe" } else { "other" };
    fs::rename(&other_bin, other.path().join(other_name)).unwrap();
    let other_bin = other.path().join(other_name);

    let results = found(
        &["other", "my_test_bin"],
        &[first.path(), other.path(), second.path()],
    );
    assert_eq!(
        results,
        vec![
            other_bin.canonicalize().unwrap(),
            first_bin.canonicalize().unwrap(),
            second_bin.canonicalize().unwrap(),
        ]
    );
}

#[test]
fn first_result_per_name_is_the_first_dir_that_has_it() {
    let (first, first_bin) = dir_with_executable(BIN);
    let (second, _) = dir_with_executable(BIN);
    let mut it = which_in(&["my_test_bin"], [first.path(), second.path()]);
    assert_eq!(it.next(), Some(first_bin.canonicalize().unwrap()));
}

#[test]
fn a_name_with_a_separator_is_checked_where_it_points() {
    let (dir, path) = dir_with_executable(BIN);
    let unrelated = tempfile::tempdir().unwrap();
    let direct = path.to_str().unwrap();
    assert_eq!(
        found(&[direct], &[unrelated.path()]),
        vec![path.canonicalize().unwrap()]
    );
    // And a bare name is never treated as relative to the working directory.
    assert!(found(&["my_test_bin"], &[unrelated.path()]).is_empty());
    let _ = dir;
}

#[test]
fn returns_nothing_for_a_missing_name() {
    let (dir, _) = dir_with_executable(BIN);
    assert!(found(&["__absolutely_nonexistent_binary_12345__"], &[dir.path()]).is_empty());
}

#[test]
fn returns_nothing_for_no_names_or_no_dirs() {
    let (dir, _) = dir_with_executable(BIN);
    let none: &[&str] = &[];
    assert!(found(none, &[dir.path()]).is_empty());
    assert!(found(&["my_test_bin"], &[]).is_empty());
}

#[test]
fn does_not_find_directories() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join(BIN)).unwrap();
    assert!(found(&["my_test_bin"], &[dir.path()]).is_empty());
}

#[cfg(unix)]
#[test]
fn does_not_find_non_executable_files() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, path) = dir_with_executable(BIN);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(found(&["my_test_bin"], &[dir.path()]).is_empty());
    assert_eq!(is_valid_executable(&path), None);
}

#[cfg(unix)]
#[test]
fn does_not_find_dangling_symlinks() {
    let (dir, path) = dir_with_executable(BIN);
    let link = dir.path().join("dangling");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(found(&["dangling"], &[dir.path()]).is_empty());
    assert_eq!(is_valid_executable(&link), None);
}

#[test]
fn is_valid_executable_split_joins_then_checks() {
    let (dir, path) = dir_with_executable(BIN);
    assert_eq!(
        is_valid_executable_split(dir.path(), BIN),
        Some(path.canonicalize().unwrap())
    );
    assert_eq!(is_valid_executable_split(dir.path(), "nope"), None);
}

#[test]
fn accepts_str_and_string_names() {
    let (dir, _) = dir_with_executable(BIN);
    let from_str: Vec<_> = which_in(&["my_test_bin"], [dir.path()]).collect();
    let from_string: Vec<_> = which_in(&["my_test_bin".to_string()], [dir.path()]).collect();
    assert_eq!(from_str, from_string);
    assert_eq!(from_str.len(), 1);
}

#[test]
fn empty_path_entry_means_the_current_directory() {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let value = format!("{sep}{}", std::env::temp_dir().display());
    let dirs = util::search_paths_from(Some(OsStr::new(&value))).unwrap();
    assert_eq!(dirs[0], PathBuf::from("."));
    assert_eq!(dirs[1], std::env::temp_dir());
}

#[test]
fn unset_path_is_an_error() {
    assert_eq!(
        util::search_paths_from(None).unwrap_err(),
        WhichError::MissingPathVariable
    );
    assert_eq!(WhichError::MissingPathVariable.to_string(), "PATH is unset");
}

#[test]
fn pathext_parsing() {
    let parse = |v: &str| util::parse_pathext(Some(OsStr::new(v)));
    assert_eq!(parse(".COM;.EXE"), vec![".COM", ".EXE"]);
    assert_eq!(parse(" .exe ; cmd ;;. ;"), vec![".exe", ".cmd"]);
    let default = util::parse_pathext(None);
    assert_eq!(default, util::parse_pathext(Some(OsStr::new("  "))));
    assert_eq!(&default[..2], [".COM", ".EXE"]);
}

#[test]
fn candidate_names_append_each_extension_once() {
    let exts = util::parse_pathext(Some(OsStr::new(".EXE;.BAT")));
    assert_eq!(
        util::candidate_names("foo", &exts),
        vec!["foo", "foo.EXE", "foo.BAT"]
    );
    assert_eq!(
        util::candidate_names("foo.exe", &exts),
        vec!["foo.exe", "foo.exe.BAT"]
    );
    assert_eq!(util::candidate_names("foo", &[]), vec!["foo"]);
}

#[test]
fn executable_extension_is_case_insensitive() {
    let exts = util::parse_pathext(Some(OsStr::new(".EXE;.CMD")));
    assert!(util::has_executable_extension(
        Path::new("a/foo.exe"),
        &exts
    ));
    assert!(util::has_executable_extension(Path::new("FOO.Cmd"), &exts));
    assert!(!util::has_executable_extension(Path::new("foo.txt"), &exts));
    assert!(!util::has_executable_extension(Path::new("foo"), &exts));
    assert!(!util::has_executable_extension(Path::new("foo.exe"), &[]));
}

#[test]
fn verbatim_prefixes_are_simplified() {
    let simplify = |s: &str| util::simplify_verbatim(PathBuf::from(s));
    assert_eq!(simplify(r"\\?\C:\x\y.exe"), PathBuf::from(r"C:\x\y.exe"));
    assert_eq!(
        simplify(r"\\?\UNC\host\share\y.exe"),
        PathBuf::from(r"\\host\share\y.exe")
    );
    assert_eq!(
        simplify(r"\\?\Volume{x}\y"),
        PathBuf::from(r"\\?\Volume{x}\y")
    );
    assert_eq!(simplify("/usr/bin/ls"), PathBuf::from("/usr/bin/ls"));
}

#[test]
fn separators_are_detected() {
    assert!(util::has_separator("./foo"));
    assert!(util::has_separator("/bin/ls"));
    assert!(!util::has_separator("ls"));
    assert_eq!(util::has_separator(r"C:\x\y.exe"), cfg!(windows));
}

#[test]
fn which_reads_path_from_the_environment() {
    let name = if cfg!(windows) { "cmd" } else { "sh" };
    let results: Vec<_> = which(&[name]).expect("PATH is set").collect();
    assert!(!results.is_empty(), "expected `{name}` on PATH");
    for path in &results {
        assert!(path.is_absolute(), "{}", path.display());
        assert!(path.is_file(), "{}", path.display());
    }
    assert_eq!(
        results[0],
        which_in(&[name], search_paths().unwrap()).next().unwrap()
    );
}

#[cfg(unix)]
#[test]
fn matches_system_which() {
    let Some(system) = which(&["which"]).unwrap().next() else {
        return;
    };
    let out = std::process::Command::new(system)
        .arg("sh")
        .output()
        .expect("run which");
    let printed = String::from_utf8_lossy(&out.stdout);
    let Ok(system_sh) = Path::new(printed.trim()).canonicalize() else {
        return;
    };
    let ours: Vec<_> = which(&["sh"]).unwrap().collect();
    assert!(
        ours.contains(&system_sh),
        "{ours:?} should contain {system_sh:?}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn errors_serialise_with_their_key() {
    let json = serde_json::to_value(WhichError::MissingPathVariable).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "$type": "dev.thmsn.which.search_paths.error.unset" })
    );
    let back: WhichError = serde_json::from_value(json).unwrap();
    assert_eq!(back, WhichError::MissingPathVariable);
}
