# c_rust

编译步骤:
cargo build 
./target/debug/rust_c_compiler ./target/debug/hello.c


书提供的测试用例 

./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1 --stage lex
./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1 --stage parse