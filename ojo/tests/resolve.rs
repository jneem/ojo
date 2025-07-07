use {
    anyhow::Result,
    cmd_lib::{run_cmd as rc___, run_fun as rf___},
    test_context::test_context,
};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn resolve_create_and_apply_patch(ctx: &libs::OjoContext) -> Result<()> {
    let ojo = &ctx.ojo;

    rc___!(echo "0-1 0-2 1-3 2-3" | $ojo synthesize)?;

    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .arg("resolve")
        .assert()
        .failure();

    let hash = rf___!(echo "1111" | $ojo resolve --author me --testing)?;
    let hash = hash.split_whitespace().nth(2).unwrap();

    rc___!($ojo patch apply $hash)?;
    rc___!($ojo render)?;

    Ok(())
}
