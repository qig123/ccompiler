# ccompiler
# test
./run-tests.sh

TackyIR_Program
  main:
    Copy 0 a.0
    Copy 1 b.1
    JumpIfZero a.0 label3
    JumpIfZero b.1 label3
    Copy 1 tmp5
    Jump label4
  label3:
    Copy 0 tmp5
  label4:
    Copy tmp5 x.2
    return x.2
    return 0
    
int main(void) {
    int a = 0, b = 1;
    int x = a && b;  // 由于 a=0（假），逻辑与（&&）短路，b 不会被计算
    return x;        // x 的值是 0
}


