#!/bin/bash

# set -e: 如果任何命令失败，脚本将立即退出。
# 这可以防止在构建失败时还尝试运行测试。
set -e

echo "--- 正在构建编译器... ---"
cargo build

echo "--- 正在运行测试... ---"
../writing-a-c-compiler-tests/test_compiler ./target/debug/ccompiler --chapter 2 --stage lex

echo "--- 所有测试通过！---"