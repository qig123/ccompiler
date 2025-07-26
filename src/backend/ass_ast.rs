#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub instructions: Vec<Instructions>,
}
#[derive(Debug)]
pub enum Instructions {
    Mov { src: Operand, dst: Operand },
    Ret,
}
#[derive(Debug)]
pub enum Operand {
    Imm(i64),
    Register(),
}
