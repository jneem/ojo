use {
    anyhow::Result,
    assert_fs::{assert::PathAssert, prelude::*},
    cmd_lib::run_cmd as rc___,
    predicates::prelude::*,
    test_context::test_context,
};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn init_creates_files(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    ctx.temp_dir.child(".ojo").assert(predicate::path::exists());
    ctx.temp_dir
        .child(".ojo/db")
        .assert(predicate::path::exists());
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn init_with_existing_repo(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    rc___!($ojo init)?;
    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .arg("init")
        .assert()
        .failure()
        .stdout(predicate::str::contains("There is already a repository"));
    Ok(())
}
