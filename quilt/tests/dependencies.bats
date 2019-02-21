#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "unapply dependencies" {
    $QL init
    echo First > quilt_file.txt
    HASH_A=`$QL patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $QL patch apply -- $HASH_A
    echo Last >> quilt_file.txt
    HASH_B=`$QL patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $QL patch apply $HASH_B

    cat > quilt_file.txt <<EOF
First
Middle
Last
EOF
    HASH_C=`$QL patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $QL patch apply -- $HASH_C

    run $QL patch apply --revert "$HASH_A"
    assert_success

    $QL render
    run cat quilt_file.txt
    assert_success
    assert_output ""

    run $QL log
    assert_success
    assert_output ""
}
