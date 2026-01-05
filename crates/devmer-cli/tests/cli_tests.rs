//! CLI integration tests

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

/// Get a command for the devmer binary
fn cmd() -> Command {
    Command::cargo_bin("devmer").unwrap()
}

#[test]
fn test_help() {
    cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A self-hosted Infrastructure as Code tool"))
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("preview"))
        .stdout(predicate::str::contains("up"))
        .stdout(predicate::str::contains("down"));
}

#[test]
fn test_version() {
    cmd()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("devmer"))
        .stdout(predicate::str::contains("rust"));
}

#[test]
fn test_version_flag() {
    cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("devmer"));
}

#[test]
fn test_stack_help() {
    cmd()
        .args(["stack", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stack management commands"))
        .stdout(predicate::str::contains("ls"))
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("select"));
}

#[test]
fn test_config_help() {
    cmd()
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration management"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"));
}

#[test]
fn test_secrets_help() {
    cmd()
        .args(["secrets", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Secrets management"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("rotate"));
}

#[test]
fn test_state_help() {
    cmd()
        .args(["state", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("State management"))
        .stdout(predicate::str::contains("export"))
        .stdout(predicate::str::contains("import"))
        .stdout(predicate::str::contains("unlock"));
}

#[test]
fn test_convert_help() {
    cmd()
        .args(["convert", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Convert HCL"))
        .stdout(predicate::str::contains("from"))
        .stdout(predicate::str::contains("analyze"))
        .stdout(predicate::str::contains("formats"));
}

#[test]
fn test_convert_formats() {
    cmd()
        .args(["convert", "formats"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Supported Conversions"))
        .stdout(predicate::str::contains("typescript"))
        .stdout(predicate::str::contains("python"))
        .stdout(predicate::str::contains("go"))
        .stdout(predicate::str::contains("rhai"));
}

#[test]
fn test_new_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "test-project";

    cmd()
        .current_dir(temp_dir.path())
        .args(["new", project_name, "--runtime", "typescript"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created project"));

    // Check that files were created
    let project_dir = temp_dir.path().join(project_name);
    assert!(project_dir.exists());
    assert!(project_dir.join("Devmer.toml").exists());
    assert!(project_dir.join("package.json").exists());
    assert!(project_dir.join("tsconfig.json").exists());
    assert!(project_dir.join("index.ts").exists());
    assert!(project_dir.join(".gitignore").exists());
}

#[test]
fn test_new_python_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "py-project";

    cmd()
        .current_dir(temp_dir.path())
        .args(["new", project_name, "--runtime", "python"])
        .assert()
        .success();

    let project_dir = temp_dir.path().join(project_name);
    assert!(project_dir.join("Devmer.toml").exists());
    assert!(project_dir.join("requirements.txt").exists());
    assert!(project_dir.join("pyproject.toml").exists());
    assert!(project_dir.join("__main__.py").exists());
}

#[test]
fn test_new_go_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "go-project";

    cmd()
        .current_dir(temp_dir.path())
        .args(["new", project_name, "--runtime", "go"])
        .assert()
        .success();

    let project_dir = temp_dir.path().join(project_name);
    assert!(project_dir.join("Devmer.toml").exists());
    assert!(project_dir.join("go.mod").exists());
    assert!(project_dir.join("main.go").exists());
}

#[test]
fn test_new_rhai_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "rhai-project";

    cmd()
        .current_dir(temp_dir.path())
        .args(["new", project_name, "--runtime", "rhai"])
        .assert()
        .success();

    let project_dir = temp_dir.path().join(project_name);
    assert!(project_dir.join("Devmer.toml").exists());
    assert!(project_dir.join("main.rhai").exists());
}

#[test]
fn test_new_project_already_exists() {
    let temp_dir = TempDir::new().unwrap();
    let project_name = "existing-project";

    // Create the directory first
    std::fs::create_dir(temp_dir.path().join(project_name)).unwrap();

    cmd()
        .current_dir(temp_dir.path())
        .args(["new", project_name])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_init_no_project() {
    let temp_dir = TempDir::new().unwrap();

    // Running init in a directory without Devmer.toml should work
    cmd()
        .current_dir(temp_dir.path())
        .args(["init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized Devmer project"));

    // Devmer.toml should be created
    assert!(temp_dir.path().join("Devmer.toml").exists());
}

#[test]
fn test_init_already_initialized() {
    let temp_dir = TempDir::new().unwrap();

    // Create a Devmer.toml
    std::fs::write(temp_dir.path().join("Devmer.toml"), "name = \"test\"").unwrap();

    // Running init should fail
    cmd()
        .current_dir(temp_dir.path())
        .args(["init"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_login_help() {
    cmd()
        .args(["login"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available providers"))
        .stdout(predicate::str::contains("aws"))
        .stdout(predicate::str::contains("gcp"))
        .stdout(predicate::str::contains("azure"));
}

#[test]
fn test_login_unknown_provider() {
    cmd()
        .args(["login", "unknown"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown provider"));
}

#[test]
fn test_verbose_flag() {
    cmd()
        .args(["-v", "--help"])
        .assert()
        .success();
}

#[test]
fn test_working_directory_flag() {
    let temp_dir = TempDir::new().unwrap();

    cmd()
        .args(["-C", temp_dir.path().to_str().unwrap(), "version"])
        .assert()
        .success();
}
