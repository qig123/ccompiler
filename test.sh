#!/bin/bash

set -e

echo "--- 正在构建编译器... ---"
cargo build

echo "--- 正在运行测试... ---"

# 定义要执行的命令
TEST_COMMAND="../writing-a-c-compiler-tests/test_compiler ./target/debug/ccompiler --chapter 7 --stage validate "

# 打印命令
echo "$ $TEST_COMMAND"

# 执行命令
$TEST_COMMAND

echo "--- 所有测试通过！---"