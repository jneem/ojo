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
