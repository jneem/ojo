load 'libs/bats-support/load'
load 'libs/bats-assert/load'

setup() {
    export TOPLEVEL="$BATS_TEST_DIRNAME/../.."
    export OJO="$TOPLEVEL/target/debug/ojo"

    # Ensure that the build is up-to-date.
    cargo build --all

    # Run everything in a clean tmpdir.
    export TEST_WORKING_DIR=$(mktemp -d)
    cd "$TEST_WORKING_DIR"
}

teardown() {
    rm -fr "$TEST_WORKING_DIR"
}

