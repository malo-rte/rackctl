//! End-to-end CLI tests, all using `--mock` so they need no hardware and stay
//! deterministic (test-writing-rules TST-8/TST-12).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::Command;
use predicates::prelude::*;

fn tascamctl() -> Command {
    Command::cargo_bin("tascamctl").expect("binary builds")
}

#[test]
fn list_succeeds_and_shows_known_keys() {
    tascamctl()
        .args(["--mock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mute"))
        .stdout(predicate::str::contains("eq-low-volume"))
        .stdout(predicate::str::contains("master-volume"));
}

#[test]
fn get_returns_seeded_defaults() {
    tascamctl()
        .args(["--mock", "get", "master-volume"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("127"));
    tascamctl()
        .args(["--mock", "get", "mute", "-c", "3"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("false"));
}

#[test]
fn get_enum_shows_label() {
    tascamctl()
        .args(["--mock", "get", "route", "-c", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Master Left (0)"));
}

#[test]
fn set_valid_value_succeeds_silently() {
    tascamctl()
        .args(["--mock", "set", "mute", "on", "-c", "3"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn relative_and_toggle_succeed() {
    // `+N` / `-N` on an int and `toggle` on a bool all exit 0. The leading `-`
    // must be taken as the value, not parsed as an option.
    for args in [
        &["--mock", "set", "master-volume", "+5"][..],
        &["--mock", "set", "master-volume", "-5"][..],
        &["--mock", "set", "mute", "toggle", "-c", "2"][..],
    ] {
        tascamctl()
            .args(args)
            .assert()
            .success()
            .stdout(predicate::str::is_empty());
    }
}

#[test]
fn toggle_on_int_fails() {
    tascamctl()
        .args(["--mock", "set", "master-volume", "toggle"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn set_out_of_range_fails() {
    tascamctl()
        .args(["--mock", "set", "eq-low-volume", "999", "-c", "0"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn unknown_control_fails() {
    tascamctl()
        .args(["--mock", "get", "nonsuch"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn setting_the_meter_block_fails() {
    tascamctl()
        .args(["--mock", "set", "meter", "1"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn global_control_rejects_nonzero_channel() {
    tascamctl()
        .args(["--mock", "set", "master-volume", "100", "-c", "1"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn missing_argument_is_a_usage_error() {
    // clap reports usage errors with exit code 2.
    tascamctl()
        .args(["--mock", "set"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn meters_prints_all_channels() {
    for extra in [&[][..], &["--raw"][..]] {
        let mut cmd = tascamctl();
        cmd.arg("--mock").arg("meters").args(extra);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("ch1 "))
            .stdout(predicate::str::contains("ch16 "))
            .stdout(predicate::str::contains("master"));
    }
}
