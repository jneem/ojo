#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "patch create: I/O error message" {
    $QL init
    run $QL patch create -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file quilt_file.txt"
}

@test "patch create: choose file" {
    $QL init
    run $QL patch create --path other_file.txt -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file other_file.txt"
}

@test "patch create: author required" {
    $QL init
    touch quilt_file.txt
    run $QL patch create -m msg
    assert_failure
}

@test "patch create: msg required" {
    $QL init
    touch quilt_file.txt
    run $QL patch create -a me
    assert_failure
}

@test "patch create: empty file ok" {
    $QL init
    touch quilt_file.txt
    run $QL patch create -a me -m msg
    assert_success
}

@test "patch create: doesn't apply by default" {
    $QL init
    echo contents > quilt_file.txt
    $QL patch create -a me -m msg
    $QL render --path out.txt
    run cat out.txt
    assert_output ""
}

@test "patch create: then-apply" {
    $QL init
    echo contents > quilt_file.txt
    $QL patch create -a me -m msg --then-apply
    $QL render --path out.txt
    run cat out.txt
    assert_output "contents"
}

@test "patch create: output-hash" {
    $QL init
    echo contents > quilt_file.txt
    run $QL patch create -a Author -m Msg --output-hash
    assert_success
    assert_output --regexp "^P[-=_a-zA-Z0-9]{44}$"
}
