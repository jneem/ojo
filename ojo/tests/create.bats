#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "patch create: I/O error message" {
    $OJO init
    run $OJO patch create -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file ojo_file.txt"
}

@test "patch create: choose file" {
    $OJO init
    run $OJO patch create --path other_file.txt -a me -m msg
    assert_failure
    assert_line --index 0 "Error: Could not read the file other_file.txt"
}

@test "patch create: author required" {
    $OJO init
    touch ojo_file.txt
    run $OJO patch create -m msg
    assert_failure
}

@test "patch create: msg required" {
    $OJO init
    touch ojo_file.txt
    run $OJO patch create -a me
    assert_failure
}

@test "patch create: empty file ok" {
    $OJO init
    touch ojo_file.txt
    run $OJO patch create -a me -m msg
    assert_success
}

@test "patch create: doesn't apply by default" {
    $OJO init
    echo contents > ojo_file.txt
    $OJO patch create -a me -m msg
    $OJO render --path out.txt
    run cat out.txt
    assert_output ""
}

@test "patch create: then-apply" {
    $OJO init
    echo contents > ojo_file.txt
    $OJO patch create -a me -m msg --then-apply
    $OJO render --path out.txt
    run cat out.txt
    assert_output "contents"
}

@test "patch create: output-hash" {
    $OJO init
    echo contents > ojo_file.txt
    run $OJO patch create -a Author -m Msg --output-hash
    assert_success
    assert_output --regexp "^P[-=_a-zA-Z0-9]{44}$"
}
