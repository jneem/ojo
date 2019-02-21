#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "unapply dependencies" {
    $JP init
    echo First > jp_file.txt
    HASH_A=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply -- $HASH_A
    echo Last >> jp_file.txt
    HASH_B=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply $HASH_B

    cat > jp_file.txt <<EOF
First
Middle
Last
EOF
    HASH_C=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply -- $HASH_C

    run $JP patch apply --revert "$HASH_A"
    assert_success

    $JP render
    run cat jp_file.txt
    assert_success
    assert_output ""

    run $JP log
    assert_success
    assert_output ""
}
