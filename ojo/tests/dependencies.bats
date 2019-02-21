#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "unapply dependencies" {
    $OJO init
    echo First > ojo_file.txt
    HASH_A=`$OJO patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $OJO patch apply -- $HASH_A
    echo Last >> ojo_file.txt
    HASH_B=`$OJO patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $OJO patch apply $HASH_B

    cat > ojo_file.txt <<EOF
First
Middle
Last
EOF
    HASH_C=`$OJO patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $OJO patch apply -- $HASH_C

    run $OJO patch apply --revert "$HASH_A"
    assert_success

    $OJO render
    run cat ojo_file.txt
    assert_success
    assert_output ""

    run $OJO log
    assert_success
    assert_output ""
}
