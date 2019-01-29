#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "deleted middle" {
    $JP init
    cat > jp_file.txt <<EOF
First
Second
Third
EOF
    HASH=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply "$HASH"

    cat > jp_file.txt <<EOF
First
Third
EOF

    HASH=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply "$HASH"

    $JP render
    run cat jp_file.txt
    assert_line --index 0 "First"
    assert_line --index 1 "Third"
}

@test "conflict" {
    $JP init
    cat > jp_file.txt <<EOF
First
Last
EOF
    HASH=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply "$HASH"

    cat > jp_file.txt <<EOF
First
Second
Last
EOF
    HASH_A=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`

    cat > jp_file.txt <<EOF
First
Middle
Last
EOF
    HASH_B=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`

    $JP patch apply -- "$HASH_A"
    $JP patch apply -- "$HASH_B"
    run $JP render
    cat jp_file.txt
    assert_failure
    assert_output "Error: Couldn't render a file, because the data isn't ordered"
}