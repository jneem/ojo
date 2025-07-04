use {
    anyhow::Result,
    assert_fs::{assert::PathAssert, prelude::*},
    cmd_lib::{run_cmd as rc___, run_fun as rf___},
    test_context::test_context,
};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn unapply_dependencies(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\n")?;
    let hash_a = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash_a)?;

    ojo_file.write_str("Last\n")?;
    let hash_b = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash_b)?;

    ojo_file.touch()?;
    ojo_file.write_str("First\nMiddle\nLast\n")?;
    let hash_c = rf___!($ojo patch create -a Author -m Msg --output-hash)?;
    rc___!($ojo patch apply $hash_c)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "apply", "--revert", &hash_a])
        .assert()
        .success();

    rc___!($ojo render)?;
    ojo_file.assert("");

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["log"])
        .assert()
        .success()
        .stdout("");
    Ok(())
}
