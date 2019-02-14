#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "export: output filename" {
    $JP init
    echo "Content" > jp_file.txt
    HASH=`$JP patch create -a Me -m Msg --output-hash`

    # Default output is the name of the hash.
    run $JP patch export $HASH
    assert_success
    assert_output "Successfully wrote the file '$HASH'"
    assert [ -e $HASH ]

    # The filename can be overridden.
    run $JP patch export $HASH -o output.txt
    assert_success
    assert_output "Successfully wrote the file 'output.txt'"
    assert [ -e output.txt ]

    run $JP patch export $HASH --output output2.txt
    assert_success
    assert_output "Successfully wrote the file 'output2.txt'"
    assert [ -e output2.txt ]
}

@test "export: unwritable" {
    $JP init
    echo "Content" > jp_file.txt
    HASH=`$JP patch create -a Me -m Msg --output-hash`

    touch out.txt
    chmod ugo-w out.txt
    run $JP patch export $HASH -o out.txt
    assert_failure
    assert_line --index 0 "Error: Couldn't create file 'out.txt'"
    assert_line --index 1 --partial "Permission denied"
}

@test "export: bad hash" {
    $JP init
    run $JP patch export Pkw4lrX8l5dt93DbfdTmMCzFSJr3CjhF2t8u9I0R2BrM=
    assert_failure
    assert_output --partial "Error: There is no patch with hash \"Pkw4"

    run $JP patch export blah
    assert_failure
    assert_line --index 0 "Error: Found a broken PatchId"
}

@test "export: export and import" {
    $JP init
    echo Content > jp_file.txt
    HASH=`$JP patch create -a Me -m Msg --output-hash`
    echo $HASH
    $JP patch export -o patch.txt $HASH

    mkdir other
    cd other
    $JP init
    $JP patch import ../patch.txt
    $JP patch apply $HASH
    $JP render
    run cat jp_file.txt
    assert_output Content
}
