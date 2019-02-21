#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "deleted middle" {
    $QL init
    cat > quilt_file.txt <<EOF
First
Second
Third
EOF
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"

    cat > quilt_file.txt <<EOF
First
Third
EOF

    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"

    $QL render
    run cat quilt_file.txt
    assert_line --index 0 "First"
    assert_line --index 1 "Third"
}

@test "conflict" {
    $QL init
    cat > quilt_file.txt <<EOF
First
Last
EOF
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"

    cat > quilt_file.txt <<EOF
First
Second
Last
EOF
    HASH_A=`$QL patch create -a Author -m Msg --output-hash`

    cat > quilt_file.txt <<EOF
First
Middle
Last
EOF
    HASH_B=`$QL patch create -a Author -m Msg --output-hash`

    $QL patch apply "$HASH_A"
    $QL patch apply "$HASH_B"
    run $QL render
    cat quilt_file.txt
    assert_failure
    assert_output "Error: Couldn't render a file, because the data isn't ordered"
}

@test "delete and undelete" {
    $QL init
    echo "Test" > quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"

    truncate --size 0 quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"
    $QL patch apply -R "$HASH"
    $QL patch apply "$HASH"
}

@test "replacement" {
    $QL init
    echo "Test" > quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"

    echo "Alt" > quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg --output-hash`
    $QL patch apply "$HASH"
    $QL patch apply -R "$HASH"
    $QL patch apply "$HASH"
}

