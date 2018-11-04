#!/bin/env sh

TESTS_DIR="${BASH_SOURCE%/*}"

exec "${TESTS_DIR}/libs/bats-core/bin/bats" ${TESTS_DIR}
