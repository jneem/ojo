#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates a master branch" {
    $OJO init
    run $OJO branch list
    assert_output "* master"
}

@test "new branch" {
    $OJO init
    $OJO branch new new-branch
    run $OJO branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  new-branch"
}

@test "new branch with existing name" {
    $OJO init
    $OJO branch new new-branch
    run $OJO branch new new-branch
    assert_failure
    assert_output "Error: The branch \"new-branch\" already exists"
}

@test "branch list is alphabetized" {
    $OJO init
    $OJO branch new zebra
    $OJO branch new aardvark
    run $OJO branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "* master"
    assert_line --index 2 "  zebra"
}

@test "branch switch" {
    $OJO init
    $OJO branch new zebra
    $OJO branch new aardvark

    $OJO branch switch aardvark
    run $OJO branch list
    assert_line --index 0 "* aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "  zebra"

    $OJO branch switch zebra
    run $OJO branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "* zebra"
}

@test "delete branch" {
    $OJO init
    $OJO branch new zebra
    $OJO branch new aardvark
    $OJO branch delete aardvark
    run $OJO branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  zebra"
}

@test "delete branch on active branch" {
    $OJO init
    $OJO branch new zebra
    $OJO branch new aardvark
    run $OJO branch delete master
    assert_failure
    assert_output "Error: \"master\" is the current branch"
}

@test "new branch creates empty file" {
    $OJO init
    echo "content" >> ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $OJO patch apply "$HASH"

    $OJO branch new aardvark
    $OJO branch switch aardvark
    $OJO render
    [ -f ojo_file.txt ]
    ! [ -s ojo_file.txt ]
}

@test "clone branch copies the file" {
    $OJO init
    echo "content" >> ojo_file.txt
    HASH=`$OJO patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $OJO patch apply "$HASH"

    $OJO branch clone aardvark
    $OJO branch switch aardvark
    $OJO render
    [ -f ojo_file.txt ]
    run cat ojo_file.txt
    assert_output "content"
}


