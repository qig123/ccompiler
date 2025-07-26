use crate::{
    backend::ass_ast::{Function, Instructions, Operand, Program},
    frontend::c_ast::{
        Expression, Function as C_Ast_Function, Program as C_Ast_Program,
        Statement as C_Ast_Statement,
    },
};

pub struct AssGen {}
impl AssGen {
    pub fn new() -> Self {
        AssGen {}
    }
    pub fn generate_ass_ast(&mut self, c_ast: C_Ast_Program) -> Result<Program, String> {
        let mut fs: Vec<Function> = Vec::new();

        for item in &c_ast.functions {
            let ins = self.parse_functions(item)?;
            let f = Function {
                name: item.name.clone(),
                instructions: ins,
            };
            fs.push(f);
        }
        Ok(Program { functions: fs })
    }

    fn parse_functions(&mut self, f: &C_Ast_Function) -> Result<Vec<Instructions>, String> {
        let mut fs: Vec<Instructions> = Vec::new();

        for item in &f.body {
            let i = self.parse_ins(&item)?;
            fs.extend(i);
        }
        Ok(fs)
    }
    fn parse_ins(&mut self, s: &C_Ast_Statement) -> Result<Vec<Instructions>, String> {
        let mut ins: Vec<Instructions> = Vec::new();
        match s {
            C_Ast_Statement::Return(e) => {
                let i1 = match e {
                    Expression::Constant(n) => Operand::Imm(*n),
                };
                ins.push(Instructions::Mov {
                    src: i1,
                    dst: Operand::Register(),
                });
                ins.push(Instructions::Ret);
            }
        }
        Ok(ins)
    }
}
