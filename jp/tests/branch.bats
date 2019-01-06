#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates a master branch" {
    $JP init
    run $JP branch list
    assert_output "* master"
}

@test "new branch" {
    $JP init
    $JP branch new new-branch
    run $JP branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  new-branch"
}

@test "new branch with existing name" {
    $JP init
    $JP branch new new-branch
    run $JP branch new new-branch
    assert_failure
    assert_output "Error: The branch \"new-branch\" already exists"
}

@test "branch list is alphabetized" {
    $JP init
    $JP branch new zebra
    $JP branch new aardvark
    run $JP branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "* master"
    assert_line --index 2 "  zebra"
}

@test "branch switch" {
    $JP init
    $JP branch new zebra
    $JP branch new aardvark

    $JP branch switch aardvark
    run $JP branch list
    assert_line --index 0 "* aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "  zebra"

    $JP branch switch zebra
    run $JP branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "* zebra"
}

@test "delete branch" {
    $JP init
    $JP branch new zebra
    $JP branch new aardvark
    $JP branch delete aardvark
    run $JP branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  zebra"
}

@test "delete branch on active branch" {
    $JP init
    $JP branch new zebra
    $JP branch new aardvark
    run $JP branch delete master
    assert_failure
    assert_output "Error: \"master\" is the current branch"
}

@test "new branch creates empty file" {
    $JP init
    echo "content" >> jp_file.txt
    HASH=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply "$HASH"

    $JP branch new aardvark
    $JP branch switch aardvark
    $JP render
    [ -f jp_file.txt ]
    ! [ -s jp_file.txt ]
}

@test "clone branch copies the file" {
    $JP init
    echo "content" >> jp_file.txt
    HASH=`$JP patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $JP patch apply "$HASH"

    $JP branch clone aardvark
    $JP branch switch aardvark
    $JP render
    [ -f jp_file.txt ]
    run cat jp_file.txt
    assert_output "content"
}


