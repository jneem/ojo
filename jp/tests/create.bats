#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "patch create: I/O error message" {
    $JP init
    run $JP patch create -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file jp_file.txt"
}

@test "patch create: choose file" {
    $JP init
    run $JP patch create --path other_file.txt -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file other_file.txt"
}

@test "patch create: author required" {
    $JP init
    touch jp_file.txt
    run $JP patch create -m msg
    assert_failure
}

@test "patch create: msg required" {
    $JP init
    touch jp_file.txt
    run $JP patch create -a me
    assert_failure
}

@test "patch create: empty file ok" {
    $JP init
    touch jp_file.txt
    run $JP patch create -a me -m msg
    assert_success
}

@test "patch create: doesn't apply by default" {
    $JP init
    echo contents > jp_file.txt
    $JP patch create -a me -m msg
    $JP render --path out.txt
    run cat out.txt
    assert_output ""
}

@test "patch create: then-apply" {
    $JP init
    echo contents > jp_file.txt
    $JP patch create -a me -m msg --then-apply
    $JP render --path out.txt
    run cat out.txt
    assert_output "contents"
}

@test "patch create: output-hash" {
    $JP init
    echo contents > jp_file.txt
    run $JP patch create -a Author -m Msg --output-hash
    assert_success
    assert_output --regexp "^[-=_a-zA-Z0-9]{44}$"
}
