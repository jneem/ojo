use {
    anyhow::Result,
    assert_fs::{assert::PathAssert, prelude::*},
    cmd_lib::{run_cmd as rc___, run_fun as rf___},
    test_context::test_context,
};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn deleted_middle(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nSecond\nThird\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nThird\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    rc___!($ojo render)?;
    ojo_file.assert("First\nThird\n");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn conflict(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nLast\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nSecond\nLast\n")?;
    let hash_a = rf___!($ojo patch create -a Author -m Msg --output-hash)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nMiddle\nLast\n")?;
    let hash_b = rf___!($ojo patch create -a Author -m Msg --output-hash)?;

    rc___!($ojo patch apply $hash_a)?;
    rc___!($ojo patch apply $hash_b)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .arg("render")
        .assert()
        .failure()
        .stdout("Error: Couldn't render a file, because the data isn't ordered\n");
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn delete_and_undelete(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.touch()?;
    ojo_file.write_str("Test\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    ojo_file.touch()?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    rc___!($ojo patch apply -R $hash)?;
    rc___!($ojo patch apply $hash)?;
    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn replacement(ctx: &libs::OjoContext) -> Result<()> {
    // drop tempfile and use assert_fs::TempDir only?
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.touch()?;
    ojo_file.write_str("Test\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;

    ojo_file.touch()?;
    ojo_file.write_str("Alt\n")?;
    let hash = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash)?;
    rc___!($ojo patch apply -R $hash)?;
    rc___!($ojo patch apply $hash)?;

    Ok(())
}
