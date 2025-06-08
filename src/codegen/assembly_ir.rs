// codegen/assembly_ir.rs (示例定义)

#[derive(Debug, PartialEq, Clone)]
pub enum Reg {
    AX, // Represents EAX (32-bit) or RAX (64-bit), depends on context/later lowering
    DX,
    R10, // Represents R10D (32-bit) or R10 (64-bit)
    R11, //
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnaryOperator {
    Neg, // Negate (e.g., negl)
    Not, // Bitwise Not (e.g., notl)
}
#[derive(Debug, PartialEq, Clone)]

pub enum BinaryOperator {
    Add,
    Sub,
    Mult,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operand {
    Imm(i64),       // Immediate integer constant
    Reg(Reg),       // Register
    Pseudo(String), // Pseudoregister (temporary, will be replaced by Stack)
    Stack(i64),     // Stack address (offset relative to RBP)
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    Mov {
        src: Operand,
        dst: Operand,
    },
    Unary {
        op: UnaryOperator,
        operand: Operand,
    }, // Unary operation on the operand (in-place)

    Binary {
        op: BinaryOperator,
        left_operand: Operand,
        right_operand: Operand,
    },
    Idiv(Operand),
    Cdq,
    AllocateStack(i64), // Allocate space on the stack (argument is the positive size needed)
    Ret,                // Return from function
}

#[derive(Debug, PartialEq, Clone)]
pub struct AssFunction {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

// Your ASDL Program definition seems to contain only one function
#[derive(Debug, PartialEq, Clone)]
pub struct Assemble {
    pub function: AssFunction,
}
