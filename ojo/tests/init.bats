#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates files" {
    $OJO init
    assert [ -e .ojo ]
    assert [ -e .ojo/db ]
}

@test "init with existing repo" {
    $OJO init
    run $OJO init
    assert_failure
    assert_output --partial "There is already a repository"
}
