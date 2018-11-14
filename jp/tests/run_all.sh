#!/bin/env sh

TESTS_DIR="${BASH_SOURCE%/*}"

echo ${TESTS_DIR}
exec "${TESTS_DIR}/libs/bats-core/bin/bats" ${TESTS_DIR}
