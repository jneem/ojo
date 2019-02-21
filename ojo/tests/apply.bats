#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "deleted middle" {
    $OJO init
    cat > ojo_file.txt <<EOF
First
Second
Third
EOF
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"

    cat > ojo_file.txt <<EOF
First
Third
EOF

    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"

    $OJO render
    run cat ojo_file.txt
    assert_line --index 0 "First"
    assert_line --index 1 "Third"
}

@test "conflict" {
    $OJO init
    cat > ojo_file.txt <<EOF
First
Last
EOF
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"

    cat > ojo_file.txt <<EOF
First
Second
Last
EOF
    HASH_A=`$OJO patch create -a Author -m Msg --output-hash`

    cat > ojo_file.txt <<EOF
First
Middle
Last
EOF
    HASH_B=`$OJO patch create -a Author -m Msg --output-hash`

    $OJO patch apply "$HASH_A"
    $OJO patch apply "$HASH_B"
    run $OJO render
    cat ojo_file.txt
    assert_failure
    assert_output "Error: Couldn't render a file, because the data isn't ordered"
}

@test "delete and undelete" {
    $OJO init
    echo "Test" > ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"

    truncate --size 0 ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"
    $OJO patch apply -R "$HASH"
    $OJO patch apply "$HASH"
}

@test "replacement" {
    $OJO init
    echo "Test" > ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"

    echo "Alt" > ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg --output-hash`
    $OJO patch apply "$HASH"
    $OJO patch apply -R "$HASH"
    $OJO patch apply "$HASH"
}

