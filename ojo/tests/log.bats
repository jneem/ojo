#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "log with no repo" {
    run $OJO log
    assert_failure
    assert_output "Error: Failed to find a ojo repository"
}
