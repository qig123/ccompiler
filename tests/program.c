// a.c

// 函数声明 (在 TACKY 生成时会被忽略)
int add(int x, int y);

// main 函数，程序的入口
int main(void) {
    int a = 5;
    int b = 10;
    int result;

    // 调用 add 函数，参数是变量 a 和 b
    result = add(a, b);

    // 再次调用，参数是常量和表达式
    // 注意：add(result, 2) 中的 'result' 是一个变量
    result = add(result, 2);

    return result;
}

// add 函数的定义
int add(int x, int y) {
    return x + y;
}