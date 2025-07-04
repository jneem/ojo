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
fn export_output_filename(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");

    rc___!($ojo init)?;
    ojo_file.write_str("Content\n")?;
    let hash = rf___!($ojo patch create -a Me -m Msg --output-hash)?;

    // Default output is the name of the hash.
    let output = rf___!($ojo patch export $hash)?;
    assert_eq!(output, format!("Successfully wrote the file '{hash}'"));
    ctx.temp_dir.child(&hash).assert(predicate::path::exists());

    // The filename can be overridden with -o.
    let output = rf___!($ojo patch export $hash -o output.txt)?;
    assert_eq!(output, "Successfully wrote the file 'output.txt'");
    ctx.temp_dir
        .child("output.txt")
        .assert(predicate::path::exists());

    // The filename can be overridden with --output.
    let output = rf___!($ojo patch export $hash --output output2.txt)?;
    assert_eq!(output, "Successfully wrote the file 'output2.txt'");
    ctx.temp_dir
        .child("output2.txt")
        .assert(predicate::path::exists());

    Ok(())
}

// The rust image on CircleCI doesn't have a non-root user, so it runs tests
// as root. When running this test as root, it fails because root always has
// permission.
// TODO: figure out how to run as non-root on CircleCI.
#[test_context(libs::OjoContext)]
#[test]
#[ignore = "This doesn't work on CircleCI"]
fn export_unwritable(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");
    let out_file = ctx.temp_dir.child("out.txt");

    rc___!($ojo init)?;
    ojo_file.write_str("Content\n")?;
    let hash = rf___!($ojo patch create -a Me -m Msg --output-hash)?;

    out_file.touch()?;
    // Make file unwritable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(out_file.path())?.permissions();
        perms.set_mode(0o444); // Read-only
        std::fs::set_permissions(out_file.path(), perms)?;
    }

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "export", &hash, "-o", "out.txt"])
        .assert()
        .failure()
        .stdout(predicates::str::starts_with(
            "Error: Couldn't create file 'out.txt'",
        ));

    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn export_bad_hash(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args([
            "patch",
            "export",
            "Pkw4lrX8l5dt93DbfdTmMCzFSJr3CjhF2t8u9I0R2BrM=",
        ])
        .assert()
        .failure()
        .stdout(predicates::str::starts_with(
            "Error: There is no patch with hash \"Pkw4",
        ));

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "export", "blah"])
        .assert()
        .failure()
        .stdout(predicates::str::starts_with(
            "Error: Found a broken PatchId\n",
        ));

    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn export_and_import(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;
    let ojo_file = ctx.temp_dir.child("ojo_file.txt");
    let other_dir = ctx.temp_dir.child("other");

    rc___!($ojo init)?;
    ojo_file.write_str("Content\n")?;
    let hash = rf___!($ojo patch create -a Me -m Msg --output-hash)?;
    rc___!($ojo patch export -o patch.txt $hash)?;

    other_dir.create_dir_all()?;
    std::env::set_current_dir(other_dir.path())?;

    let other_ojo_file = other_dir.child("ojo_file.txt");
    rc___!($ojo init)?;
    rc___!($ojo patch import ../patch.txt)?;
    rc___!($ojo patch apply $hash)?;
    rc___!($ojo render)?;

    other_ojo_file.assert("Content\n");

    Ok(())
}

#[test_context(libs::OjoContext)]
#[test]
fn import_bad_file(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!($ojo init)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .args(["patch", "import", "no_such_file.txt"])
        .assert()
        .failure()
        .stdout(predicates::str::starts_with(
            "Error: Failed to read file 'no_such_file.txt'",
        ));

    Ok(())
}
