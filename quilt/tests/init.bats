#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates files" {
    $JP init
    assert [ -e .jp ]
    assert [ -e .jp/db ]
}

@test "init with existing repo" {
    $JP init
    run $JP init
    assert_failure
    assert_output --partial "There is already a repository"
}
