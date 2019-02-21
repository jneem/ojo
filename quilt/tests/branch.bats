#!./libs/bats-core/bin/bats

load 'libs/setup'

@test "init creates a master branch" {
    $QL init
    run $QL branch list
    assert_output "* master"
}

@test "new branch" {
    $QL init
    $QL branch new new-branch
    run $QL branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  new-branch"
}

@test "new branch with existing name" {
    $QL init
    $QL branch new new-branch
    run $QL branch new new-branch
    assert_failure
    assert_output "Error: The branch \"new-branch\" already exists"
}

@test "branch list is alphabetized" {
    $QL init
    $QL branch new zebra
    $QL branch new aardvark
    run $QL branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "* master"
    assert_line --index 2 "  zebra"
}

@test "branch switch" {
    $QL init
    $QL branch new zebra
    $QL branch new aardvark

    $QL branch switch aardvark
    run $QL branch list
    assert_line --index 0 "* aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "  zebra"

    $QL branch switch zebra
    run $QL branch list
    assert_line --index 0 "  aardvark"
    assert_line --index 1 "  master"
    assert_line --index 2 "* zebra"
}

@test "delete branch" {
    $QL init
    $QL branch new zebra
    $QL branch new aardvark
    $QL branch delete aardvark
    run $QL branch list
    assert_line --index 0 "* master"
    assert_line --index 1 "  zebra"
}

@test "delete branch on active branch" {
    $QL init
    $QL branch new zebra
    $QL branch new aardvark
    run $QL branch delete master
    assert_failure
    assert_output "Error: \"master\" is the current branch"
}

@test "new branch creates empty file" {
    $QL init
    echo "content" >> quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $QL patch apply "$HASH"

    $QL branch new aardvark
    $QL branch switch aardvark
    $QL render
    [ -f quilt_file.txt ]
    ! [ -s quilt_file.txt ]
}

@test "clone branch copies the file" {
    $QL init
    echo "content" >> quilt_file.txt
    HASH=`$QL patch create -a Author -m Msg 2>&1 | cut -d " " -f 3`
    $QL patch apply "$HASH"

    $QL branch clone aardvark
    $QL branch switch aardvark
    $QL render
    [ -f quilt_file.txt ]
    run cat quilt_file.txt
    assert_output "content"
}


