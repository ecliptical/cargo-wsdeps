use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn diff_empty_workspace() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/empty-workspace");
    let mut cmd = cargo_bin_cmd!();
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
    let mut cmd = cargo_bin_cmd!();
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
    let mut cmd = cargo_bin_cmd!();
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
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(""));

    Ok(())
}

#[test]
fn diff_single_ref() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/single-ref");
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(""));

    Ok(())
}

#[test]
fn diff_mixed_deps() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/mixed-deps");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--dotted")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

#[test]
fn diff_no_shared_deps() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/no-shared-deps");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--aggressive")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// Promote dev-dependencies and build-dependencies (not just normal deps).
#[test]
fn diff_dev_build_deps() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/dev-build-deps");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// When two members declare the same dep with different feature sets,
/// the promoted workspace entry should union the features.
#[test]
fn diff_feature_merge() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/feature-merge");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// When members declare a shared dep with `default-features = false`,
/// the promoted workspace entry should preserve `default-features = false`.
#[test]
fn diff_default_features() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/default-features");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// `--aggressive` must not inline a workspace dep that:
///   - is inherited by 2+ members, or
///   - is inherited by one member while held inline by another (still
///     consolidated to `workspace = true`, but not pulled back inline).
#[test]
fn diff_aggressive_blocked() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/aggressive-blocked");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--aggressive")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// A workspace dep no member references should be removed.
#[test]
fn diff_stale_only() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/stale-only");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// Intra-workspace path dependencies must be skipped (never promoted).
#[test]
fn diff_path_deps() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/path-deps");
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(""));

    Ok(())
}

/// Renamed dependencies (`alias = { package = "real", ... }`).
/// Pins current behavior; the tool keys on the package's real name.
#[test]
fn diff_renamed_deps() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/renamed-deps");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// When members disagree on a shared dep's version, pin the
/// reconciliation behavior (currently: takes the first member's req).
#[test]
fn diff_conflicting_versions() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/conflicting-versions");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// Workspace Cargo.toml without a `[workspace.dependencies]` table:
/// the table should be created on demand.
#[test]
fn diff_no_ws_deps_table() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/no-ws-deps-table");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// `--dotted` with promoted deps that have extras (features/optional)
/// must keep extras alongside `workspace = true` on the member entry.
#[test]
fn diff_dotted_extras() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/dotted-extras");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--dotted")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// `--aggressive` inlining a workspace dep into the sole inheriting member,
/// merging workspace fields with the member's existing extras (e.g. `optional`).
#[test]
fn diff_aggressive_merge() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/aggressive-merge");
    let expected = fs::read_to_string(dir.join("diff.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--aggressive")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// Workspace selection (`-p`) restricts the analysis to the chosen member.
#[test]
fn diff_workspace_selection() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/multi-member");
    let expected = fs::read_to_string(dir.join("diff-p-crate1.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("-p")
        .arg("crate1")
        .arg("diff")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}

/// `--dotted` produces dotted-key form for newly-inserted bare member entries
/// (contrast with `diff_multi_member` which uses inline-table form).
#[test]
fn diff_multi_member_dotted() -> Result<()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-resources/multi-member");
    let expected = fs::read_to_string(dir.join("diff-dotted.patch"))?;
    let mut cmd = cargo_bin_cmd!();
    cmd.current_dir(&dir)
        .arg("wsdeps")
        .arg("diff")
        .arg("--dotted")
        .assert()
        .success()
        .stdout(predicate::eq(expected));

    Ok(())
}
