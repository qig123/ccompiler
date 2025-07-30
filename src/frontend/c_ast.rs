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
    pub body: Block,
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
pub struct Block(pub Vec<BlockItem>);
#[derive(Debug, Clone)]
pub enum ForInit {
    InitDecl(Declaration),
    InitExp(Option<Expression>),
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
    Compound(Block),
    Break(String),
    Continue(String),
    While {
        condition: Expression,
        body: Box<Statement>,
        label: Option<String>,
    },
    DoWhile {
        body: Box<Statement>,
        condition: Expression,
        label: Option<String>,
    },
    For {
        init: ForInit,
        condition: Option<Expression>,
        post: Option<Expression>,
        body: Box<Statement>,
        label: Option<String>,
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
        self.body.pretty_print(printer);
        printer.unindent();
    }
}
impl AstNode for Block {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        printer.writeln("Block").unwrap();
        printer.indent();
        for item in &self.0 {
            item.pretty_print(printer);
        }
        printer.unindent();
    }
}

impl AstNode for BlockItem {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            BlockItem::S(s) => s.pretty_print(printer),
            BlockItem::D(d) => d.pretty_print(printer),
        }
    }
}

impl AstNode for Declaration {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        if let Some(init_expr) = &self.init {
            printer
                .writeln(&format!("Declare(name: \"{}\", with init)", self.name))
                .unwrap();
            printer.indent();
            init_expr.pretty_print(printer);
            printer.unindent();
        } else {
            printer
                .writeln(&format!("Declare(name: \"{}\")", self.name))
                .unwrap();
        }
    }
}
impl AstNode for ForInit {
    fn pretty_print(&self, printer: &mut PrettyPrinter) {
        match self {
            ForInit::InitDecl(decl) => {
                printer.writeln("ForInitDecl").unwrap();
                printer.indent();
                decl.pretty_print(printer);
                printer.unindent();
            }
            ForInit::InitExp(opt_expr) => {
                printer.writeln("ForInitExp").unwrap();
                printer.indent();
                if let Some(expr) = opt_expr {
                    expr.pretty_print(printer);
                } else {
                    printer.writeln("EmptyInit").unwrap();
                }
                printer.unindent();
            }
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
                printer.writeln("ExpressionStatement").unwrap();
                printer.indent();
                e.pretty_print(printer);
                printer.unindent();
            }
            Statement::Null => {
                printer.writeln("NullStatement(;)").unwrap();
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                printer.writeln("IfStatement").unwrap();
                printer.indent();

                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();

                printer.writeln("Then").unwrap();
                printer.indent();
                then_stmt.pretty_print(printer);
                printer.unindent();

                if let Some(else_s) = else_stmt {
                    printer.writeln("Else").unwrap();
                    printer.indent();
                    else_s.pretty_print(printer);
                    printer.unindent();
                }

                printer.unindent();
            }
            Statement::Compound(b) => {
                printer.writeln("CompoundStatement").unwrap();
                printer.indent();
                b.pretty_print(printer);
                printer.unindent();
            }
            Statement::Break(label) => {
                printer
                    .writeln(&format!("BreakStatement(->{})", label))
                    .unwrap();
            }
            Statement::Continue(label) => {
                printer
                    .writeln(&format!("ContinueStatement(->{})", label))
                    .unwrap();
            }
            Statement::While {
                condition,
                body,
                label,
            } => {
                let label_str = label.as_deref().unwrap_or("unlabeled");
                printer
                    .writeln(&format!("WhileStatement(label:{})", label_str))
                    .unwrap();
                printer.indent();

                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();

                printer.writeln("Body").unwrap();
                printer.indent();
                body.pretty_print(printer);
                printer.unindent();

                printer.unindent();
            }
            Statement::DoWhile {
                body,
                condition,
                label,
            } => {
                let label_str = label.as_deref().unwrap_or("unlabeled");
                printer
                    .writeln(&format!("DoWhileStatement(label:{})", label_str))
                    .unwrap();
                printer.indent();

                printer.writeln("Body").unwrap();
                printer.indent();
                body.pretty_print(printer);
                printer.unindent();

                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();

                printer.unindent();
            }
            Statement::For {
                init,
                condition,
                post,
                body,
                label,
            } => {
                let label_str = label.as_deref().unwrap_or("unlabeled");
                printer
                    .writeln(&format!("ForStatement(label:{})", label_str))
                    .unwrap();
                printer.indent();

                printer.writeln("Init").unwrap();
                printer.indent();
                init.pretty_print(printer);
                printer.unindent();

                printer.writeln("Condition").unwrap();
                printer.indent();
                if let Some(cond_expr) = condition {
                    cond_expr.pretty_print(printer);
                } else {
                    printer.writeln("EmptyCondition").unwrap();
                }
                printer.unindent();

                printer.writeln("Post-Expression").unwrap();
                printer.indent();
                if let Some(post_expr) = post {
                    post_expr.pretty_print(printer);
                } else {
                    printer.writeln("EmptyPostExpression").unwrap();
                }
                printer.unindent();

                printer.writeln("Body").unwrap();
                printer.indent();
                body.pretty_print(printer);
                printer.unindent();

                printer.unindent();
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
                printer.writeln("Conditional(op: '? :')").unwrap();
                printer.indent();

                printer.writeln("Condition").unwrap();
                printer.indent();
                condition.pretty_print(printer);
                printer.unindent();

                printer.writeln("Then").unwrap();
                printer.indent();
                left.pretty_print(printer);
                printer.unindent();

                printer.writeln("Else").unwrap();
                printer.indent();
                right.pretty_print(printer);
                printer.unindent();

                printer.unindent();
            }
        }
    }
}
