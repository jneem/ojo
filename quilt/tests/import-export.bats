#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "export: output filename" {
    $QL init
    echo "Content" > quilt_file.txt
    HASH=`$QL patch create -a Me -m Msg --output-hash`

    # Default output is the name of the hash.
    run $QL patch export $HASH
    assert_success
    assert_output "Successfully wrote the file '$HASH'"
    assert [ -e $HASH ]

    # The filename can be overridden.
    run $QL patch export $HASH -o output.txt
    assert_success
    assert_output "Successfully wrote the file 'output.txt'"
    assert [ -e output.txt ]

    run $QL patch export $HASH --output output2.txt
    assert_success
    assert_output "Successfully wrote the file 'output2.txt'"
    assert [ -e output2.txt ]
}

@test "export: unwritable" {
    $QL init
    echo "Content" > quilt_file.txt
    HASH=`$QL patch create -a Me -m Msg --output-hash`

    touch out.txt
    chmod ugo-w out.txt
    run $QL patch export $HASH -o out.txt
    assert_failure
    assert_line --index 0 "Error: Couldn't create file 'out.txt'"
    assert_line --index 1 --partial "Permission denied"
}

@test "export: bad hash" {
    $QL init
    run $QL patch export Pkw4lrX8l5dt93DbfdTmMCzFSJr3CjhF2t8u9I0R2BrM=
    assert_failure
    assert_output --partial "Error: There is no patch with hash \"Pkw4"

    run $QL patch export blah
    assert_failure
    assert_line --index 0 "Error: Found a broken PatchId"
}

@test "export: export and import" {
    $QL init
    echo Content > quilt_file.txt
    HASH=`$QL patch create -a Me -m Msg --output-hash`
    echo $HASH
    $QL patch export -o patch.txt $HASH

    mkdir other
    cd other
    $QL init
    $QL patch import ../patch.txt
    $QL patch apply $HASH
    $QL render
    run cat quilt_file.txt
    assert_output Content
}

@test "import: bad file" {
    $QL init
    run $QL patch import no_such_file.txt
    assert_failure
    assert_line --index 0 "Error: Failed to read file 'no_such_file.txt'"
    assert_line --index 1 --partial "No such file"
}
