use crate::{
    codegen::ast::{AssFunction, Assemble, Instruction, Operand},
    error::CodegenError,
    parse::expr::{Expr, Function, LiteralExpr, Program, Stmt},
};

pub struct AssemblyGenerator<'a> {
    source: &'a str,
}

impl<'a> AssemblyGenerator<'a> {
    pub fn new(source: &'a str) -> Self {
        AssemblyGenerator { source }
    }
    pub fn generate(&mut self, ast: Program) -> Result<Assemble, CodegenError> {
        let mut fs: Vec<AssFunction> = Vec::new();
        for function in ast.functions {
            // 生成函数的汇编代码
            let func_asm = self.generate_function(&function)?;
            fs.push(func_asm);
        }
        Ok(Assemble { function: fs })
    }
    fn generate_function(&self, function: &Function) -> Result<AssFunction, CodegenError> {
        let mut instructions = Vec::new();
        for s in &function.body {
            let i = self.generate_expr(&s)?;
            for ii in i {
                instructions.push(ii);
            }
        }
        // self.generate_expr(&function.body, &mut instructions)?;

        Ok(AssFunction {
            name: function.name.get_lexeme(self.source).to_string(),
            instructions,
        })
    }

    fn generate_expr(&self, s: &Stmt) -> Result<Vec<Instruction>, CodegenError> {
        match s {
            Stmt::Return { keyword: _, value } => {
                if let Some(exp) = value {
                    //生成常量
                    match exp {
                        Expr::Literal(lit) => {
                            match lit {
                                // 这里可以扩展更多的常量类型
                                LiteralExpr::Integer(i) => {
                                    let mut is: Vec<Instruction> = Vec::new();
                                    let mov = Instruction::Mov {
                                        src: Operand::Imm(i.clone()),
                                        dst: Operand::Register("rax".to_string()),
                                    };
                                    let ret = Instruction::Ret;
                                    // println!("Register name: {:?} {:?}", mov, ret);
                                    is.push(mov);
                                    is.push(ret);
                                    Ok(is)
                                }
                            }
                        }
                    }
                } else {
                    Err(CodegenError {
                        message: "unsupport".to_string(),
                    })
                }
            }
        }
    }
}
