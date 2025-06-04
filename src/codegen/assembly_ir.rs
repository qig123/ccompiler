#[derive(Debug)]
pub enum Operand {
    Imm(i64),
    Register(String), // 现在显式使用寄存器名
}

#[derive(Debug)]
pub enum Instruction {
    Mov { src: Operand, dst: Operand },
    Ret,
}

#[derive(Debug)]
pub struct AssFunction {
    pub name: String,
    pub instructions: Vec<Instruction>,
}

#[derive(Debug)]
pub struct Assemble {
    pub function: Vec<AssFunction>,
}
