use {anyhow::Result, test_context::test_context};

mod libs;

#[test_context(libs::OjoContext)]
#[test]
fn log_with_no_repo(_ctx: &libs::OjoContext) -> Result<()> {
    assert_cmd::Command::cargo_bin("ojo")
        .unwrap()
        .arg("log")
        .assert()
        .failure()
        .stdout("Error: Failed to find a ojo repository\n");
    Ok(())
}
