// 1. unary_operator = Complement | Negate
#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Complement, // 按位取反 (~)
    Negate,     // 算术取负 (-)
    Bang,       //不等于 (!)
}
#[derive(Debug, PartialEq)]

pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    // And,
    // Or,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

// 2. val = Constant(int) | Var(identifier)
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    Constant(i64), // 整型常量 (使用 i64 以匹配常见的寄存器大小，虽然示例用了 32 位)
    Var(String),   // 变量，使用字符串作为标识符
}

// 3. instruction = Return(val) | Unary(unary_operator, val src, val dst)
#[derive(Debug, PartialEq)]
pub enum Instruction {
    Return(Value),
    // 这是一个三地址码形式的单目运算指令： dst = op src
    Unary {
        op: UnaryOperator,
        src: Value,
        dst: Value,
    },
    Binary {
        op: BinaryOperator,
        src1: Value,
        src2: Value,
        dst: Value,
    },
    Copy {
        src: Value,
        dst: Value,
    },
    Jump {
        target: String,
    },
    JumpIfZero {
        condition: Value,
        target: String,
    },
    JumpIfNotZero {
        condition: Value,
        target: String,
    },
    Lable {
        name: String,
    },
    // 在这个最小的 IR 里，常量似乎只能作为 Unary 指令的 src。
    // Unary(Negate, Constant(2), Var("temp1")) 可以实现 temp1 = -2 的效果。
    // 如果需要 dst = src，可能需要引入一个新的指令变体，但这超出了给定的 ASDL 范围。
}

// 4. function_definition = Function(identifier, instruction* body)
#[derive(Debug, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,           // 函数名
    pub body: Vec<Instruction>, // 函数体，指令列表
}

// 5. program = Program(function_definition)
#[derive(Debug, PartialEq)]
pub struct Program {
    pub definition: FunctionDefinition, // 程序包含一个函数定义
}
