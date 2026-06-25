//! End-to-end CLI tests, all using `--mock` so they need no hardware and stay
//! deterministic (test-writing-rules TST-8/TST-12).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("rackctl-us16x08").expect("binary builds")
}

/// Run `save` to `path` (optionally a single strip), asserting success.
fn save_to(path: &str, channel: Option<&str>) {
    let mut cmd = bin();
    cmd.arg("--mock").arg("save").arg(path);
    if let Some(ch) = channel {
        cmd.arg("-c").arg(ch);
    }
    cmd.assert().success();
}

#[test]
fn list_succeeds_and_shows_known_keys() {
    bin()
        .args(["--mock", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mute"))
        .stdout(predicate::str::contains("eq-low-volume"))
        .stdout(predicate::str::contains("master-volume"));
}

#[test]
fn get_returns_seeded_defaults() {
    bin()
        .args(["--mock", "get", "master-volume"])
        .assert()
        .success()
        // The default raw 127 reads out as 0 dB in display units.
        .stdout(predicate::str::starts_with("+0 dB"));
    bin()
        .args(["--mock", "get", "mute", "-c", "3"])
        .assert()
        .success()
        .stdout(predicate::str::starts_with("false"));
}

#[test]
fn topology_explains_routing() {
    // Backend-independent, so it needs no device or --mock.
    bin()
        .arg("topology")
        .assert()
        .success()
        .stdout(predicate::str::contains("signal flow"))
        .stdout(predicate::str::contains("MASTER"))
        .stdout(predicate::str::contains("Output 1..8"));
}

#[test]
fn info_enum_lists_values() {
    bin()
        .args(["--mock", "info", "comp-ratio"])
        .assert()
        .success()
        .stdout(predicate::str::contains("enum"))
        .stdout(predicate::str::contains("0=1.0:1"))
        .stdout(predicate::str::contains("14=inf:1"));
}

#[test]
fn info_shows_description_for_global_switches() {
    bin()
        .args(["--mock", "info", "buss-out"])
        .assert()
        .success()
        .stdout(predicate::str::contains("about:"))
        .stdout(predicate::str::contains("stereo master bus"));
}

#[test]
fn info_int_shows_range() {
    bin()
        .args(["--mock", "info", "master-volume"])
        .assert()
        .success()
        .stdout(predicate::str::contains("int"))
        .stdout(predicate::str::contains("0..=133"));
}

#[test]
fn info_unknown_control_fails() {
    bin()
        .args(["--mock", "info", "nonsuch"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn get_enum_shows_label() {
    bin()
        .args(["--mock", "get", "route", "-c", "0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Master Left (0)"));
}

#[test]
fn set_valid_value_succeeds_silently() {
    bin()
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
        bin()
            .args(args)
            .assert()
            .success()
            .stdout(predicate::str::is_empty());
    }
}

#[test]
fn toggle_on_int_fails() {
    bin()
        .args(["--mock", "set", "master-volume", "toggle"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn set_out_of_range_clamps() {
    // An absolute value beyond the control's range is clamped to it (as the GUI
    // sliders and relative adjusts do), not rejected.
    bin()
        .args(["--mock", "set", "eq-low-volume", "999", "-c", "0"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn unknown_control_fails() {
    bin()
        .args(["--mock", "get", "nonsuch"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn setting_the_meter_block_fails() {
    bin()
        .args(["--mock", "set", "meter", "1"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn global_control_rejects_nonzero_channel() {
    bin()
        .args(["--mock", "set", "master-volume", "100", "-c", "1"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn missing_argument_is_a_usage_error() {
    // clap reports usage errors with exit code 2.
    bin().args(["--mock", "set"]).assert().failure().code(2);
}

#[test]
fn save_writes_mixer_and_strip_json() {
    let dir = tempfile::tempdir().unwrap();
    let mixer = dir.path().join("mix.json");
    let strip = dir.path().join("strip.json");

    save_to(mixer.to_str().unwrap(), None);
    save_to(strip.to_str().unwrap(), Some("0"));

    let mixer_json = std::fs::read_to_string(&mixer).unwrap();
    assert!(mixer_json.contains("\"kind\": \"mixer\""));
    assert!(mixer_json.contains("master-volume"));
    assert!(mixer_json.contains("channels"));

    let strip_json = std::fs::read_to_string(&strip).unwrap();
    assert!(strip_json.contains("\"kind\": \"strip\""));
    assert!(strip_json.contains("comp-ratio"));
}

#[test]
fn load_strip_requires_a_channel() {
    let dir = tempfile::tempdir().unwrap();
    let strip = dir.path().join("strip.json");
    save_to(strip.to_str().unwrap(), Some("0"));
    let strip = strip.to_str().unwrap();

    // Applying a strip to a channel works.
    bin()
        .args(["--mock", "load", strip, "-c", "5"])
        .assert()
        .success();
    // Without a target channel it is an error.
    bin()
        .args(["--mock", "load", strip])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn load_mixer_rejects_a_channel() {
    let dir = tempfile::tempdir().unwrap();
    let mixer = dir.path().join("mix.json");
    save_to(mixer.to_str().unwrap(), None);
    let mixer = mixer.to_str().unwrap();

    bin().args(["--mock", "load", mixer]).assert().success();
    bin()
        .args(["--mock", "load", mixer, "-c", "0"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn load_missing_file_fails() {
    bin()
        .args(["--mock", "load", "/no/such/preset.json"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn meters_prints_all_channels() {
    for extra in [&[][..], &["--raw"][..]] {
        let mut cmd = bin();
        cmd.arg("--mock").arg("meters").args(extra);
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("ch1 "))
            .stdout(predicate::str::contains("ch16 "))
            .stdout(predicate::str::contains("master"));
    }
}

#[test]
fn default_preset_round_trips() {
    // Point the config dir at a temp location (XDG, Linux) so the test does not
    // touch the user's real default preset.
    let dir = tempfile::tempdir().unwrap();

    // Loading before one exists is an error.
    bin()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--mock", "default"])
        .assert()
        .failure()
        .code(1);

    // Save the current mixer as the default, then load it back.
    bin()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--mock", "default", "--save"])
        .assert()
        .success();
    assert!(
        dir.path()
            .join("rackctl/us16x08/default-preset.json")
            .exists()
    );
    bin()
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["--mock", "default"])
        .assert()
        .success();
}
