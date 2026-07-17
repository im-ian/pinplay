#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fake_executable(directory: &TempDir, name: &str, body: &str) -> std::path::PathBuf {
    let path = directory.path().join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
    path
}

#[test]
fn help_describes_the_direct_source_syntax() {
    Command::cargo_bin("pinplay")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("pinplay [OPTIONS] [SOURCE]"))
        .stdout(predicate::str::contains("--start"))
        .stdout(predicate::str::contains("--size"))
        .stdout(predicate::str::contains("doctor"));
}

#[test]
fn exact_source_is_passed_after_an_option_delimiter() {
    let directory = TempDir::new().unwrap();
    let capture = directory.path().join("args.txt");
    let marker = directory.path().join("injected");
    let mpv = fake_executable(
        &directory,
        "mpv",
        "printf '%s\\n' \"$@\" > \"$PINPLAY_CAPTURE\"",
    );
    let source = format!(
        "https://example.test/video;touch {}",
        marker.to_string_lossy()
    );

    Command::cargo_bin("pinplay")
        .unwrap()
        .env("PINPLAY_CAPTURE", &capture)
        .args(["--mpv", mpv.to_str().unwrap(), &source, "--mute"])
        .assert()
        .success();

    let captured = fs::read_to_string(capture).unwrap();
    let args: Vec<_> = captured.lines().collect();
    assert_eq!(&args[args.len() - 2..], ["--", source.as_str()]);
    assert!(
        !marker.exists(),
        "source text must never be executed by a shell"
    );
}

#[test]
fn doctor_json_is_machine_readable_and_offline() {
    let directory = TempDir::new().unwrap();
    let mpv = fake_executable(&directory, "mpv", "echo 'mpv 1.0'");
    let yt_dlp = fake_executable(&directory, "yt-dlp", "echo '2026.01.01'");
    let deno = fake_executable(&directory, "deno", "echo 'deno 2.0'");

    Command::cargo_bin("pinplay")
        .unwrap()
        .args([
            "--mpv",
            mpv.to_str().unwrap(),
            "--yt-dlp",
            yt_dlp.to_str().unwrap(),
            "--deno",
            deno.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ready\": true"))
        .stdout(predicate::str::contains("\"name\": \"mpv\""))
        .stdout(predicate::str::contains("\"name\": \"yt-dlp\""))
        .stdout(predicate::str::contains("\"name\": \"deno\""));
}

#[test]
fn doctor_rejects_a_binary_that_cannot_report_its_version() {
    let directory = TempDir::new().unwrap();
    let broken_mpv = fake_executable(&directory, "mpv", "exit 42");
    let yt_dlp = fake_executable(&directory, "yt-dlp", "echo '2026.01.01'");
    let deno = fake_executable(&directory, "deno", "echo 'deno 2.0'");

    Command::cargo_bin("pinplay")
        .unwrap()
        .args([
            "--mpv",
            broken_mpv.to_str().unwrap(),
            "--yt-dlp",
            yt_dlp.to_str().unwrap(),
            "--deno",
            deno.to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .assert()
        .code(3)
        .stdout(predicate::str::contains("\"ready\": false"))
        .stdout(predicate::str::contains("\"found\": false"));
}

#[test]
fn missing_player_has_an_actionable_error() {
    Command::cargo_bin("pinplay")
        .unwrap()
        .args([
            "--mpv",
            "/definitely/missing/mpv",
            "https://example.test/video",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("pinplay doctor"))
        .stderr(predicate::str::contains("mpv"));
}
