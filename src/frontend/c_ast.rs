// src/frontend/c_ast.rs

use crate::common::{AstNode, PrettyPrinter};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<String>,
    pub body: Vec<BlockItem>,
}
#[derive(Debug, Clone)]
pub enum BlockItem {
    S(Statement),
    D(Declaration),
}
#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub init: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Return(Expression),
    Expression(Expression),
    Null,
    If {
        condition: Expression,
        then_stmt: Box<Statement>,
        else_stmt: Option<Box<Statement>>,
    },
}

#[derive(Debug, Clone)]
pub enum Expression {
    Constant(i64),
    Unary {
        op: UnaryOp,
        exp: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Var(String),
    Assignment {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        left: Box<Expression>,
        right: Box<Expression>,
    },
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Complement,
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    And,
    Or,
    EqualEqual,
    BangEqual,
    LessEqual,
    GreaterEqual,
    Less,
    Greater,
}

// --- Display Trait 实现 (与您版本相同，此处省略) ---
impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnaryOp::Complement => write!(f, "~"),
            UnaryOp::Negate => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Subtract => write!(f, "-"),
            BinaryOp::Multiply => write!(f, "*"),
            BinaryOp::Divide => write!(f, "/"),
            BinaryOp::Remainder => write!(f, "%"),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::BangEqual => write!(f, "!="),
            BinaryOp::EqualEqual => write!(f, "=="),
            BinaryOp::Greater => write!(f, ">"),
            BinaryOp::GreaterEqual => write!(f, ">="),
            BinaryOp::Less => write!(f, "<"),
            BinaryOp::LessEqual => write!(f, "<="),
        }
    }
}

// --- AstNode Trait (Pretty Printer) 实现 ---

impl AstNode for Program {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("Program").unwrap();
        printer.indent();
        for function in &self.functions {
            function.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for Function {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        let params_str = if self.parameters.is_empty() {
            "void".to_string()
        } else {
            self.parameters.join(", ")
        };
        printer
            .writeln(&format!(
                "Function(name: \"{}\", params: [{}])",
                self.name, params_str
            ))
            .unwrap();

        printer.indent();
        if !self.body.is_empty() {
            printer.writeln("Body").unwrap();
            printer.indent();
            for item in &self.body {
                item.pretty_print(printer); // 打印 BlockItem
            }
            printer.unindent();
        }
        printer.unindent();
    }
}

// 优化: 为 BlockItem 实现 pretty_print
impl AstNode for BlockItem {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            // 直接委托给内部的 Statement 或 Declaration
            BlockItem::S(s) => s.pretty_print(printer),
            BlockItem::D(d) => d.pretty_print(printer),
        }
    }
}

// 优化: 为 Declaration 实现 pretty_print
impl AstNode for Declaration {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        // 检查是否有初始化表达式
        if let Some(init_expr) = &self.init {
            // 如果有，打印一个更详细的节点信息
            printer
                .writeln(&format!("Declare(name: \"{}\", with init)", self.name))
                .unwrap();
            printer.indent();
            init_expr.pretty_print(printer); // 打印初始化表达式的子树
            printer.unindent();
        } else {
            // 如果没有，只打印变量名
            printer
                .writeln(&format!("Declare(name: \"{}\")", self.name))
                .unwrap();
        }
    }
}

impl AstNode for Statement {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Statement::Return(expr) => {
                printer.writeln("Return").unwrap();
                printer.indent();
                expr.pretty_print(printer);
                printer.unindent();
            }
            Statement::Expression(e) => {
                // 优化: 明确这是一个表达式语句
                printer.writeln("ExpressionStatement").unwrap();
                printer.indent();
                e.pretty_print(printer);
                printer.unindent();
            }
            Statement::Null => {
                // 优化: 使用更具描述性的名称
                printer.writeln("NullStatement(;)").unwrap();
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                printer.writeln("IfStatement").unwrap();
                printer.indent(); // 整体缩进

                // 1. 打印 Condition 分支
                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();

                // 2. 打印 Then 分支
                printer.writeln("Then").unwrap();
                printer.indent();
                then_stmt.pretty_print(printer);
                printer.unindent();

                // 3. 打印 Else 分支 (如果存在)
                if let Some(else_s) = else_stmt {
                    printer.writeln("Else").unwrap();
                    printer.indent();
                    else_s.pretty_print(printer);
                    printer.unindent();
                }

                printer.unindent(); // 恢复整体缩进
            }
        }
    }
}

impl AstNode for Expression {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            Expression::Constant(value) => {
                printer.writeln(&format!("Constant({})", value)).unwrap();
            }
            Expression::Unary { op, exp } => {
                printer.writeln(&format!("Unary(op: '{}')", op)).unwrap();
                printer.indent();
                exp.pretty_print(printer);
                printer.unindent();
            }
            Expression::Binary { op, left, right } => {
                printer.writeln(&format!("Binary(op: '{}')", op)).unwrap();
                printer.indent();
                left.pretty_print(printer);
                right.pretty_print(printer);
                printer.unindent();
            }
            Expression::Var(n) => {
                printer.writeln(&format!("Var(name: \"{}\")", n)).unwrap();
            }
            Expression::Assignment { left, right } => {
                printer.writeln("Assignment(op: '=')").unwrap();
                printer.indent();
                left.pretty_print(printer);
                right.pretty_print(printer);
                printer.unindent();
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                // 1. 打印节点本身的类型
                printer.writeln("Conditional(op: '? :')").unwrap();
                // 2. 增加一级缩进，为所有子节点做准备
                printer.indent();
                // 3. 打印 Condition 分支，并为其子树增加额外缩进
                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();
                // 4. 打印 Then 分支 (left)，并为其子树增加额外缩进
                printer.writeln("Then").unwrap();
                printer.indent();
                left.pretty_print(printer);
                printer.unindent();
                // 5. 打印 Else 分支 (right)，并为其子树增加额外缩进
                printer.writeln("Else").unwrap();
                printer.indent();
                right.pretty_print(printer);
                printer.unindent();
                // 6. 恢复到上一级缩进
                printer.unindent();
            }
        }
    }
}
