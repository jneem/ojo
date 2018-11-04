#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates files" {
    touch file.txt
    $JP init file.txt
    assert [ -e .jp ]
    assert [ -e .jp/db ]
}

@test "init with existing repo" {
    touch file.txt
    $JP init file.txt
    run $JP init file.txt
    assert_failure
    assert_output --partial "There is already a repository"
}
