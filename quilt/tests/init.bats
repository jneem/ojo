#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates files" {
    $QL init
    assert [ -e .quilt ]
    assert [ -e .quilt/db ]
}

@test "init with existing repo" {
    $QL init
    run $QL init
    assert_failure
    assert_output --partial "There is already a repository"
}
