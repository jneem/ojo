#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "log with no repo" {
    run $JP log
    assert_failure
    assert_output "Error: Failed to find a jp repository"
}
