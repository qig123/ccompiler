# C Compiler 

## 📌 当前进度  
✅ **Chapter 1: A Minimal Compiler**  
✅ Chapter 2: UNARY OPERATORS  
✅ Chapter 3: BINARY OPERATORS  
⬜ Chapter 4: LOGICAL AND RELATIONAL OPERATORS  

---

## 🛠 编译与运行

### 构建项目

cargo build

### 运行编译器
./target/debug/ccompiler ./target/debug/hello.c
📝 注意：测试用的 .c 文件需放在与生成程序相同的目录


🧪 测试套件
[官方测试仓库](https://github.com/nlsandler/writing-a-c-compiler-tests)

./test_compiler ../ccompiler/target/debug/ccompiler --chapter 1 --stage lex
./test_compiler ../ccompiler/target/debug/ccompiler --chapter 2 --stage tacky
./test_compiler ../ccompiler/target/debug/ccompiler --chapter 3 
./test_compiler ../ccompiler/target/debug/ccompiler --chapter 4   --stage lex