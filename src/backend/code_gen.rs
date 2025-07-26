use crate::backend::ass_ast::Program;

pub struct GenerCode {}
impl GenerCode {
    pub fn new() -> Self {
        GenerCode {}
    }
    fn code_gen(&mut self, ast: Program) -> Result<(), String> {
        for item in ast.functions {}
    }
}
