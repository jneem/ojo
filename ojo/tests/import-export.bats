#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "export: output filename" {
    $OJO init
    echo "Content" > ojo_file.txt
    HASH=`$OJO patch create -a Me -m Msg --output-hash`

    # Default output is the name of the hash.
    run $OJO patch export $HASH
    assert_success
    assert_output "Successfully wrote the file '$HASH'"
    assert [ -e $HASH ]

    # The filename can be overridden.
    run $OJO patch export $HASH -o output.txt
    assert_success
    assert_output "Successfully wrote the file 'output.txt'"
    assert [ -e output.txt ]

    run $OJO patch export $HASH --output output2.txt
    assert_success
    assert_output "Successfully wrote the file 'output2.txt'"
    assert [ -e output2.txt ]
}

@test "export: unwritable" {
    $OJO init
    echo "Content" > ojo_file.txt
    HASH=`$OJO patch create -a Me -m Msg --output-hash`

    touch out.txt
    chmod ugo-w out.txt
    run $OJO patch export $HASH -o out.txt
    assert_failure
    assert_line --index 0 "Error: Couldn't create file 'out.txt'"
    assert_line --index 1 --partial "Permission denied"
}

@test "export: bad hash" {
    $OJO init
    run $OJO patch export Pkw4lrX8l5dt93DbfdTmMCzFSJr3CjhF2t8u9I0R2BrM=
    assert_failure
    assert_output --partial "Error: There is no patch with hash \"Pkw4"

    run $OJO patch export blah
    assert_failure
    assert_line --index 0 "Error: Found a broken PatchId"
}

@test "export: export and import" {
    $OJO init
    echo Content > ojo_file.txt
    HASH=`$OJO patch create -a Me -m Msg --output-hash`
    echo $HASH
    $OJO patch export -o patch.txt $HASH

    mkdir other
    cd other
    $OJO init
    $OJO patch import ../patch.txt
    $OJO patch apply $HASH
    $OJO render
    run cat ojo_file.txt
    assert_output Content
}

@test "import: bad file" {
    $OJO init
    run $OJO patch import no_such_file.txt
    assert_failure
    assert_line --index 0 "Error: Failed to read file 'no_such_file.txt'"
    assert_line --index 1 --partial "No such file"
}
