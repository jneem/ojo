use {
    anyhow::Result,
    assert_fs::{assert::PathAssert, prelude::*},
    cmd_lib::{run_cmd as rc___, run_fun as rf___},
    predicates::prelude::*,
    test_context::test_context,
};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_io_error_message(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "create", "-a", "me", "-m", "msg"])
        .assert()
        .failure()
        .stdout(predicate::str::starts_with(
            "Error: Could not read the file ojo_file.txt\n",
        ));
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_choose_file(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "create", "-a", "me", "-m", "msg", "other_file.txt"])
        .assert()
        .failure()
        .stdout(predicate::str::starts_with(
            "Error: Could not read the file other_file.txt\n",
        ));
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_author_required(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;
    ojo_file.touch()?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "create", "-m", "msg"])
        .assert()
        .failure();
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_msg_required(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;
    ojo_file.touch()?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "create", "-a", "me"])
        .assert()
        .failure();
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_empty_file_ok(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;
    ojo_file.touch()?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "create", "-a", "me", "-m", "msg"])
        .assert()
        .success();
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_doesnt_apply_by_default(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");
    let out_file = ctx.temp_dir.child("out.txt");

    rc___!($ojo init)?;
    ojo_file.write_str("contents\n")?;
    rc___!($ojo patch create -a me -m msg)?;
    rc___!($ojo render out.txt)?;

    out_file.assert("");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_then_apply(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");
    let out_file = ctx.temp_dir.child("out.txt");

    rc___!($ojo init)?;
    ojo_file.write_str("contents\n")?;
    rc___!($ojo patch create -a me -m msg --then-apply)?;
    rc___!($ojo render out.txt)?;

    out_file.assert("contents\n");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn patch_create_output_hash(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;
    ojo_file.write_str("contents\n")?;

    let output = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    assert!(is_valid_patch_id(&output));
    Ok(())
}

fn is_valid_patch_id(s: &str) -> bool {
    if s.len() != 45 {
        return false;
    }
    let mut chars = s.chars();
    if chars.next() != Some('P') {
        return false;
    }
    chars.all(|c| {
        c == '-'
            || c == '='
            || c == '_'
            || c.is_ascii_lowercase()
            || c.is_ascii_uppercase()
            || c.is_ascii_digit()
    })
}
