# c_rust

# 编译
cargo build 
# ,要测试的文件 .c程序 目前暂时放到和 生成程序的同一目录(为了对接书提供的测试) 运行
./target/debug/rust_c_compiler ./target/debug/hello.c


# 书提供的测试
https://github.com/nlsandler/writing-a-c-compiler-tests


./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1 --stage lex
./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1 --stage parse
./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1 --stage codegen
./test_compiler ../rust_c_compiler/target/debug/rust_c_compiler --chapter 1


