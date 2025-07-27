use crate::common::PrettyPrinter;

//src/frontend/c_ast.rs
pub trait AstNode {
    fn pretty_print(&self, printer: &mut PrettyPrinter);
}
#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>, // Will be empty for "void"
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Return(Expression),
}

#[derive(Debug)]
pub enum Expression {
    Constant(i64),
}

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("Program");
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
        }
        printer.unindent();
    }
}
impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        // 构建函数签名的字符串
        let params = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            self.parameters.join(", ")
        };
        printer.writeln(&format!(
            "Function(name: {}, params: [{}])",
            self.name, params
        ));

        // 打印函数体
        printer.indent();
        printer.writeln("Body");
        printer.indent();
        for statement in &self.body {
            statement.pretty_print(printer);
        }
        printer.unindent();
        printer.unindent();
    }
}

impl AstNode for Statement {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Statement::Return(expr) => {
                printer.writeln("Return");
                printer.indent();
                expr.pretty_print(printer);
                printer.unindent();
            }
        }
    }
}

impl AstNode for Expression {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Expression::Constant(value) => {
                printer.writeln(&format!("Constant(value: {})", value));
            }
        }
    }
}
