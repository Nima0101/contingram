use serde_json::Value;
use std::process::{Command, Output};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_contingram"))
        .args(args)
        .env("CONTINGRAM_TELEMETRY", "0")
        .output()
        .unwrap()
}
#[test]
fn command_contract_and_exit_codes() {
    for args in [
        &["--help"][..],
        &["--version"],
        &["demo"],
        &["telemetry", "status"],
        &["telemetry", "explain"],
    ] {
        assert!(run(args).status.success(), "{args:?}");
    }
    let file = "examples/contracts/receipt.json";
    for (option, value, code) in [
        ("--horizon", "2", 0),
        ("--horizon", "1", 1),
        ("--max-work", "0", 3),
    ] {
        let out = run(&["analyze", file, option, value]);
        assert_eq!(out.status.code(), Some(code));
        assert!(out.stderr.is_empty());
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["schema_version"], 1);
    }
    for args in [
        vec!["analyze", file, "--horizon", "2", "--horizon", "2"],
        vec!["lower", "."],
        vec!["lower", "missing-file"],
        vec!["analyze", file, "--horizon", "33"],
        vec!["--unknown"],
        vec!["analyze", file, "--max-work", "-1"],
    ] {
        let out = run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty());
        let _: Value = serde_json::from_slice(&out.stderr).unwrap();
    }
    let out = run(&["lower", file]);
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ir"]["semantic_version"], 1);
}
#[test]
fn portable_artifact_and_space_path() {
    let dir = std::env::temp_dir().join(format!("contingram cli {}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("policy with spaces.json");
    let file = "examples/contracts/receipt.json";
    let out = run(&["analyze", file, "--horizon", "2"]);
    std::fs::write(&path, out.stdout).unwrap();
    let report = path.to_str().unwrap();
    for command in ["verify", "explain", "follow"] {
        assert!(run(&[command, file, report]).status.success());
    }
    let out = run(&["follow", file, report, "absent-final", "ack"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stdout).unwrap()["kind"],
        "goal"
    );
    assert_eq!(
        run(&["follow", file, report, "unexpected"]).status.code(),
        Some(2)
    );
    let mut v: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    v["horizon"] = 0.into();
    std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    assert_eq!(run(&["verify", file, report]).status.code(), Some(2));
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn telemetry_is_discoverable_and_collection_is_disabled() {
    assert!(String::from_utf8(run(&["--help"]).stdout)
        .unwrap()
        .contains("telemetry"));
    for flag in ["0", "1"] {
        let out = Command::new(env!("CARGO_BIN_EXE_contingram"))
            .args(["telemetry", "status"])
            .env("CONTINGRAM_TELEMETRY", flag)
            .output()
            .unwrap();
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(value["collection_enabled"], false);
        assert_eq!(value["endpoint"], Value::Null);
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_are_payload_free_errors() {
    use std::os::unix::ffi::OsStrExt;
    let out = Command::new(env!("CARGO_BIN_EXE_contingram"))
        .arg("lower")
        .arg(std::ffi::OsStr::from_bytes(b"\xffSECRET_SENTINEL"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let _: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(!String::from_utf8_lossy(&out.stderr).contains("SECRET_SENTINEL"));
}

#[test]
fn input_stream_cap_crlf_and_telemetry_do_not_change_results() {
    let file = "examples/contracts/receipt.json";
    let a = run(&["analyze", file]);
    let b = Command::new(env!("CARGO_BIN_EXE_contingram"))
        .args(["analyze", file])
        .env("CONTINGRAM_TELEMETRY", "1")
        .output()
        .unwrap();
    assert_eq!(a.stdout, b.stdout);
    assert_eq!(a.status.code(), b.status.code());
    let dir = std::env::temp_dir().join(format!("contingram-input-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("crlf.json");
    std::fs::write(
        &path,
        std::fs::read_to_string(file).unwrap().replace('\n', "\r\n"),
    )
    .unwrap();
    assert_eq!(run(&["analyze", path.to_str().unwrap()]).stdout, a.stdout);
    std::fs::write(&path, vec![b' '; contingram::MAX_CONTRACT_BYTES + 1]).unwrap();
    let out = run(&["analyze", path.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stderr).unwrap()["code"],
        "input_limit"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn device_input_is_rejected_before_reading() {
    let out = run(&["lower", "/dev/null"]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stderr).unwrap()["code"],
        "io_error"
    );
}
