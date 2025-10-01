use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_command() {
    let mut cmd = Command::cargo_bin("ai-coach").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Terminal-based training log"))
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("workout"));
}

#[test]
fn test_version_command() {
    let mut cmd = Command::cargo_bin("ai-coach").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_completions_command() {
    let mut cmd = Command::cargo_bin("ai-coach").unwrap();
    cmd.arg("completions").arg("bash");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_ai-coach"));
}
