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
fn init_creates_a_master_branch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    let output = rf___!($ojo branch list)?;
    assert_eq!(output, "* master");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn new_branch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new new-branch)?;

    let output = rf___!($ojo branch list)?;
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "* master");
    assert_eq!(lines[1], "  new-branch");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn new_branch_with_existing_name(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new new-branch)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["branch", "new", "new-branch"])
        .assert()
        .failure()
        .stdout("Error: The branch \"new-branch\" already exists\n");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn branch_list_is_alphabetized(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new zebra)?;
    rc___!($ojo branch new aardvark)?;

    let output = rf___!($ojo branch list)?;
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "  aardvark");
    assert_eq!(lines[1], "* master");
    assert_eq!(lines[2], "  zebra");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn branch_switch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new zebra)?;
    rc___!($ojo branch new aardvark)?;

    rc___!($ojo branch switch aardvark)?;
    let output = rf___!($ojo branch list)?;
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "* aardvark");
    assert_eq!(lines[1], "  master");
    assert_eq!(lines[2], "  zebra");

    rc___!($ojo branch switch zebra)?;
    let output = rf___!($ojo branch list)?;
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "  aardvark");
    assert_eq!(lines[1], "  master");
    assert_eq!(lines[2], "* zebra");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn delete_branch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new zebra)?;
    rc___!($ojo branch new aardvark)?;
    rc___!($ojo branch delete aardvark)?;

    let output = rf___!($ojo branch list)?;
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines[0], "* master");
    assert_eq!(lines[1], "  zebra");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn delete_branch_on_active_branch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;
    rc___!($ojo branch new zebra)?;
    rc___!($ojo branch new aardvark)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["branch", "delete", "master"])
        .assert()
        .failure()
        .stdout("Error: \"master\" is the current branch\n");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn new_branch_creates_empty_file(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.write_str("content\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    rc___!($ojo branch new aardvark)?;
    rc___!($ojo branch switch aardvark)?;
    rc___!($ojo render)?;

    ojo_file.assert(predicate::path::exists());
    ojo_file.assert(predicate::path::is_file());
    ojo_file.assert("");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn clone_branch_copies_the_file(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");
    let out_file = ctx.temp_dir.child("out.txt");

    rc___!($ojo init)?;

    ojo_file.write_str("content\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    rc___!($ojo branch clone aardvark)?;
    rc___!($ojo branch switch aardvark)?;
    rc___!($ojo render out.txt)?;

    out_file.assert(predicate::path::exists());
    out_file.assert(predicate::path::is_file());
    out_file.assert("content\n");
    Ok(())
}
