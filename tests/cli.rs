use std::process::Command;

#[test]
fn version_flag_prints_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_gshell"))
        .arg("--version")
        .output()
        .expect("gshell binary should run");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        format!("gshell {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_flag_prints_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_gshell"))
        .arg("--help")
        .output()
        .expect("gshell binary should run");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("gshell -c <command>"));
    assert!(stdout.contains("gshell --version"));
    assert!(stdout.contains("~/.gshrc"));
}

#[test]
fn c_flag_runs_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_gshell"))
        .args(["-c", "echo hello"])
        .output()
        .expect("gshell binary should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn c_flag_inherits_parent_environment() {
    let output = Command::new(env!("CARGO_BIN_EXE_gshell"))
        .args(["-c", "echo $INHERITED_VAR"])
        .env("INHERITED_VAR", "from-parent")
        .output()
        .expect("gshell binary should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "from-parent\n");
}
