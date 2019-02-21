#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "log with no repo" {
    run $QL log
    assert_failure
    assert_output "Error: Failed to find a quilt repository"
}
