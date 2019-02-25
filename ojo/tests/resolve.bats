#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "resolve: create and apply patch" {
    echo "0-1 0-2 1-3 2-3" | $OJO synthesize
    run $OJO resolve
    assert_failure

    HASH=`echo "1111" | $OJO resolve --author me --testing 2>&1 | cut -d " " -f 3`
    $OJO patch apply $HASH
    $OJO render
}

