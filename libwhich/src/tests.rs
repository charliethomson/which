use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[test]
fn finds_common_binaries() {
    for name in &["ls", "cat", "echo", "sh"] {
        let results: Vec<_> = which(&[name]).expect("PATH should be set").collect();
        assert!(!results.is_empty(), "expected to find `{name}` on PATH");
        for path in &results {
            assert!(path.exists(), "{} should exist on disk", path.display());
            assert!(path.is_file(), "{} should be a file", path.display());
        }
    }
}

#[test]
fn returns_empty_for_nonexistent_binary() {
    let results: Vec<_> = which(&["__absolutely_nonexistent_binary_12345__"])
        .expect("PATH should be set")
        .collect();
    assert!(results.is_empty());
}

#[test]
fn multiple_names_returns_results_for_each() {
    let results: Vec<_> = which(&["ls", "sh"]).expect("PATH should be set").collect();
    assert!(results.len() >= 2, "expected results for both ls and sh");
}

#[test]
fn results_are_absolute_paths() {
    let results: Vec<_> = which(&["ls"]).expect("PATH should be set").collect();
    assert!(!results.is_empty());
    for path in &results {
        assert!(path.is_absolute(), "{} should be absolute", path.display());
    }
}

#[test]
fn results_are_executable() {
    let results: Vec<_> = which(&["ls"]).expect("PATH should be set").collect();
    assert!(!results.is_empty());
    for path in &results {
        let mode = fs::metadata(path)
            .expect("metadata should be readable")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "{} should have executable bits set",
            path.display()
        );
    }
}

#[test]
fn does_not_find_non_executable_files() {
    let dir = tempfile::tempdir().expect("failed to create tempdir");
    let file_path = dir.path().join("not_executable");
    fs::write(&file_path, "#!/bin/sh\necho hi").expect("failed to write file");
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644))
        .expect("failed to set permissions");

    let original_path = std::env::var_os("PATH").unwrap();
    // SAFETY: test is single-threaded via cargo test -- --test-threads=1 or serial execution
    unsafe { std::env::set_var("PATH", dir.path()) };

    let results: Vec<_> = which(&["not_executable"])
        .expect("PATH should be set")
        .collect();

    unsafe { std::env::set_var("PATH", &original_path) };
    assert!(
        results.is_empty(),
        "non-executable file should not be found"
    );
}

#[test]
fn finds_executable_in_custom_path() {
    let dir = tempfile::tempdir().expect("failed to create tempdir");
    let file_path = dir.path().join("my_test_bin");
    fs::write(&file_path, "#!/bin/sh\necho hi").expect("failed to write file");
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o755))
        .expect("failed to set permissions");

    let original_path = std::env::var_os("PATH").unwrap();
    let new_path = format!(
        "{}:{}",
        dir.path().display(),
        original_path.to_string_lossy()
    );
    // SAFETY: test is single-threaded via cargo test -- --test-threads=1 or serial execution
    unsafe { std::env::set_var("PATH", &new_path) };

    let results: Vec<_> = which(&["my_test_bin"])
        .expect("PATH should be set")
        .collect();

    unsafe { std::env::set_var("PATH", &original_path) };
    assert!(!results.is_empty(), "should find executable in custom PATH");
    assert!(results[0].ends_with("my_test_bin"));
}

#[test]
fn accepts_string_and_str_slices() {
    let _from_str: Vec<_> = which(&["ls"]).unwrap().collect();
    let _from_string: Vec<_> = which(&["ls".to_string()]).unwrap().collect();
}

#[test]
fn empty_names_returns_empty_iterator() {
    let empty: &[&str] = &[];
    let results: Vec<_> = which(empty).expect("PATH should be set").collect();
    assert!(results.is_empty());
}

#[test]
fn matches_system_which() {
    let our_results: Vec<_> = which(&["ls"]).expect("PATH should be set").collect();
    assert!(!our_results.is_empty());

    let system_output = std::process::Command::new("/usr/bin/which")
        .arg("ls")
        .output()
        .expect("failed to run system which");
    let system_path_str = String::from_utf8_lossy(&system_output.stdout);
    let system_path = Path::new(system_path_str.trim()).canonicalize().ok();

    if let Some(system_path) = system_path {
        assert!(
            our_results.contains(&system_path),
            "our results {:?} should contain the system which result {:?}",
            our_results,
            system_path,
        );
    }
}
