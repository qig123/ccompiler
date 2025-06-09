//analysis.rs
use crate::{common_ids, error::SemanticError, parser::c_ast::*};
use std::collections::HashMap;

fn generate_unique_variable_name(ori: String) -> String {
    common_ids::generate_analysis_variable_name(ori)
}

// Semantic Analyzer 结构体，可能需要持有对原始源代码的引用
// 以便在错误报告中获取 Token 的文本和位置
pub struct SemanticAnalyzer<'a> {
    source: &'a str,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(source: &'a str) -> Self {
        SemanticAnalyzer { source }
    }

    // 入口函数：分析整个 Program
    pub fn analyze(&mut self, program: Program) -> Result<Program, SemanticError> {
        let mut analyzed_functions = Vec::new();
        for func in program.functions {
            let analyzed_func = self.analyze_function(func)?;
            analyzed_functions.push(analyzed_func);
        }
        Ok(Program {
            functions: analyzed_functions,
        })
    }

    // 分析单个 Function
    fn analyze_function(&mut self, function: Function) -> Result<Function, SemanticError> {
        // 为每个函数创建一个新的变量映射 (局部作用域)
        let mut variable_map: HashMap<String, String> = HashMap::new();
        // 局部变量计数器也可以在函数内部管理，但为了简单，我们使用了全局计数器
        // 如果你想限制计数器作用域，可以在这里初始化并传递 mutable reference

        let analyzed_body = self.analyze_body(function.body, &mut variable_map)?;

        // 函数名不需要重命名，但可以在这里检查冲突（如果支持全局变量或其他函数的话）
        // let function_name = function.name.get_lexeme(self.source);
        // println!("Analyzing function: {}", function_name); // 调试用

        Ok(Function {
            name: function.name, // 函数名 Token 不变
            body: analyzed_body,
        })
    }

    // 分析函数体 (Vec<Block>)
    fn analyze_body(
        &mut self,
        body: Vec<Block>,
        variable_map: &mut HashMap<String, String>,
    ) -> Result<Vec<Block>, SemanticError> {
        let mut analyzed_blocks = Vec::new();
        // 遍历 body 中的 Block，处理声明和语句
        for block in body {
            let analyzed_block = self.analyze_block(block, variable_map)?;
            analyzed_blocks.push(analyzed_block);
        }
        Ok(analyzed_blocks)
    }

    // 分析单个 Block (Declaration 或 Stmt)
    fn analyze_block(
        &mut self,
        block: Block,
        variable_map: &mut HashMap<String, String>,
    ) -> Result<Block, SemanticError> {
        match block {
            Block::Declaration(decl) => {
                let analyzed_decl = self.analyze_declaration(decl, variable_map)?;
                Ok(Block::Declaration(analyzed_decl))
            }
            Block::Stmt(stmt) => {
                // 对于语句，variable_map 只会被读取，所以可以传递不可变引用，
                // 但为了简化 analyze_block 的签名，我们继续传递可变引用，
                // analyze_statement 内部会使用不可变引用。
                let analyzed_stmt = self.analyze_statement(stmt, variable_map)?;
                Ok(Block::Stmt(analyzed_stmt))
            }
        }
    }

    // 分析 Declaration
    fn analyze_declaration(
        &mut self,
        declaration: Declaration,
        variable_map: &mut HashMap<String, String>,
    ) -> Result<Declaration, SemanticError> {
        let user_name = declaration.name.get_lexeme(self.source).to_string();

        // 伪代码 ❶: 检查重复声明
        if variable_map.contains_key(&user_name) {
            return Err(SemanticError::DuplicateDeclaration { name: user_name });
        }

        // 伪代码 ❷: 生成唯一名称并添加到 map
        let unique_name = generate_unique_variable_name(user_name.clone());
        variable_map.insert(user_name, unique_name.clone());

        // 伪代码 ❸: 解析初始化表达式 (如果存在)
        let analyzed_init = if let Some(init_expr) = declaration.init {
            let analyzed_expr = self.analyze_expression(*init_expr, variable_map)?; // analyze_expression 只读 map
            Some(Box::new(analyzed_expr))
        } else {
            None
        };

        // 伪代码 ❹: 返回新的 Declaration 节点
        Ok(Declaration {
            name: declaration.name, // 保留原始 Token
            unique_name,            // 存储生成的唯一名称
            init: analyzed_init,
        })
    }

    // 分析 Statement
    fn analyze_statement(
        &mut self,
        statement: Stmt,
        variable_map: &HashMap<String, String>,
    ) -> Result<Stmt, SemanticError> {
        match statement {
            Stmt::Return { keyword, value } => {
                let analyzed_value = if let Some(return_expr) = value {
                    let analyzed_expr = self.analyze_expression(*return_expr, variable_map)?;
                    Some(Box::new(analyzed_expr))
                } else {
                    None
                };
                Ok(Stmt::Return {
                    keyword,
                    value: analyzed_value,
                })
            }
            Stmt::Expression { exp } => {
                let analyzed_exp = self.analyze_expression(*exp, variable_map)?;
                Ok(Stmt::Expression {
                    exp: Box::new(analyzed_exp),
                })
            }
            Stmt::Null => Ok(Stmt::Null),
        }
    }

    // 分析 Expression
    fn analyze_expression(
        &mut self,
        expression: Expr,
        variable_map: &HashMap<String, String>,
    ) -> Result<Expr, SemanticError> {
        match expression {
            // 伪代码 | Literal(lit)
            Expr::Literal(lit) => Ok(Expr::Literal(lit)),

            // 伪代码 | Unary(op, exp)
            Expr::Unary { operator, right } => {
                let analyzed_right = self.analyze_expression(*right, variable_map)?;
                Ok(Expr::Unary {
                    operator,
                    right: Box::new(analyzed_right),
                })
            }

            // 伪代码 | Grouping(exp)
            Expr::Grouping { expression } => {
                let analyzed_expression = self.analyze_expression(*expression, variable_map)?;
                Ok(Expr::Grouping {
                    expression: Box::new(analyzed_expression),
                })
            }

            // 伪代码 | Binary(op, left, right)
            Expr::Binary {
                operator,
                left,
                right,
            } => {
                let analyzed_left = self.analyze_expression(*left, variable_map)?;
                let analyzed_right = self.analyze_expression(*right, variable_map)?;
                Ok(Expr::Binary {
                    operator,
                    left: Box::new(analyzed_left),
                    right: Box::new(analyzed_right),
                })
            }

            // 伪代码 | Var(v)
            Expr::Var {
                name,
                unique_name: _,
            } => {
                let user_name = name.get_lexeme(self.source).to_string();

                // 伪代码: check if v is in variable_map
                match variable_map.get(&user_name) {
                    Some(unique_name) => {
                        // 伪代码: return Var(variable_map.get(v))

                        Ok(Expr::Var {
                            name,                             // 保留原始 Token
                            unique_name: unique_name.clone(), // 添加对应的唯一名称
                        })
                    }
                    None => {
                        // 伪代码: fail("Undeclared variable!")
                        Err(SemanticError::UndeclaredVariable { name: user_name })
                    }
                }
            }

            // 伪代码 | Assignment(left, right)
            Expr::Assignment { left, right } => {
                // 检查左值
                let (is_valid_lvalue, lvalue_description) = match &*left {
                    Expr::Var { .. } => (true, String::new()), // 变量是合法的左值
                    Expr::Literal(_) => (false, "Cannot assign to a literal constant".to_string()),
                    Expr::Binary { .. } => (
                        false,
                        "Cannot assign to the result of a binary operation".to_string(),
                    ),
                    Expr::Unary { .. } => (
                        false,
                        "Cannot assign to the result of a unary operation".to_string(),
                    ),
                    Expr::Grouping { .. } => {
                        (false, "Cannot assign to a grouped expression".to_string())
                    }
                    // 对于 Expr::Assignment 作为左值的情况 (e.g. (a=b)=c)，
                    // C 语言通常允许，因为赋值表达式本身有值且是右值（在某些定义下可以是左值，但会复杂化）。
                    // 简单起见，我们可以先禁止它，或者允许它但需要更复杂的左值检查。
                    // 这里暂时将其视为非法左值以简化。
                    Expr::Assignment { .. } => (
                        false,
                        "Cannot assign to an assignment expression".to_string(),
                    ),
                };

                if !is_valid_lvalue {
                    return Err(SemanticError::InvalidLvalue {
                        description: lvalue_description,
                    });
                }

                // 如果左值合法 (目前只考虑 Var)，继续分析
                // 注意：analyze_expression(*left, ...) 会确保 Var 的 unique_name 被填充
                let analyzed_left = self.analyze_expression(*left, variable_map)?;
                let analyzed_right = self.analyze_expression(*right, variable_map)?;

                Ok(Expr::Assignment {
                    left: Box::new(analyzed_left),
                    right: Box::new(analyzed_right),
                })
            }
        }
    }
}
