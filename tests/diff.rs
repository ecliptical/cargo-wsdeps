use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn diff_empty_workspace() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/empty-workspace");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .failure();

    Ok(())
}

#[test]
fn diff_multi_member() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/multi-member");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

#[test]
fn diff_single_member() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/single-member");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(""));

    Ok(())
}

#[test]
fn diff_standalone() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/standalone");
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME"))?;
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(""));

    Ok(())
}
