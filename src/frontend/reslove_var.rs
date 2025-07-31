//src/frontend/reslove_var.rs
use std::collections::HashMap;

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{
        Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement, VarDecl,
    },
};
#[derive(Debug)]
pub struct Info {
    has_linkage: bool,
    name: String,
}
#[derive(Debug)]
pub struct ResloveVar<'a> {
    env_vec: Vec<HashMap<String, Info>>,
    name_gen: &'a mut UniqueNameGenerator,
}
impl<'a> ResloveVar<'a> {
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        ResloveVar {
            env_vec: Vec::new(),
            name_gen: g,
        }
    }
    pub fn reslove_prgram(&mut self, ast: &Program) -> Result<Program, String> {
        let mut fs: Vec<FunDecl> = Vec::new();
        //我们必须添加一个顶层环境,感觉这个顶层环境不用pop,你觉得？
        self.env_vec.push(HashMap::new());
        for f in &ast.functions {
            let new_f = self.reslove_function_decl(f)?;
            fs.push(new_f);
        }
        self.env_vec.pop();
        Ok(Program { functions: fs })
    }
    fn reslove_function_decl(&mut self, f: &FunDecl) -> Result<FunDecl, String> {
        let (result, is_from_current) = self.find_variable_in_env(&f.name);
        if let Some(i) = result {
            if !i.has_linkage && is_from_current {
                return Err(format!("重复声明! {}", f.name));
            } else {
                //这里是什么情况？map中已经有一个条目，已经确定不是变量,那一定是函数，那么这意味着什么呢，意味着出现了多个同名字的函数声明
                //这里的处理是也添加到map中，但是不生成新名字,因为函数是唯一实体对应，同名的函数声明一定是要兼容的，指向唯一实体,所以覆盖也是正确的
                self.insert_new_variable(
                    f.name.clone(),
                    Info {
                        has_linkage: true,
                        name: f.name.clone(),
                    },
                );
            }
        } else {
            self.insert_new_variable(
                f.name.clone(),
                Info {
                    has_linkage: true,
                    name: f.name.clone(),
                },
            );
        }
        //解析函数参数，要新开作用域
        let env_params = HashMap::new();
        self.env_vec.push(env_params);
        let mut new_params = Vec::new();
        //这里要怎样解析？
        for p in &f.parameters {
            if self.check_variable_in_current_env(&p) {
                return Err(format!(
                    "Duplicate variable declaration in {} params",
                    f.name.clone()
                ));
            }
            let new_name = self.name_gen.new_variable_name(p.clone());
            self.insert_new_variable(
                p.clone(),
                Info {
                    has_linkage: false,
                    name: new_name.clone(),
                },
            );
            new_params.push(new_name);
        }

        let new_body = if let Some(b) = &f.body {
            // We are now in the function's scope (which contains parameters).
            // We resolve the body's items in this *same* scope,
            // instead of calling `reslove_block` which would create a new, separate scope.
            let mut bs: Vec<BlockItem> = Vec::new();
            for item in &b.0 {
                let new_item = self.reslove_blockitem(item)?;
                bs.push(new_item);
            }
            Some(Block(bs))
        } else {
            None
        };

        self.env_vec.pop(); // Pop the combined scope for parameters and function body.

        Ok(FunDecl {
            name: f.name.clone(),
            parameters: new_params,
            body: new_body,
        })
    }
    fn reslove_block(&mut self, blocks: &Block) -> Result<Block, String> {
        let map = HashMap::new();
        self.env_vec.push(map);
        let mut bs: Vec<BlockItem> = Vec::new();

        for b in &blocks.0 {
            let b = self.reslove_blockitem(&b)?;
            bs.push(b);
        }
        self.env_vec.pop();
        Ok(Block(bs))
    }
    fn reslove_blockitem(&mut self, b: &BlockItem) -> Result<BlockItem, String> {
        match b {
            BlockItem::D(d) => {
                let new_d = self.reslove_dec(d)?;
                Ok(BlockItem::D(new_d))
            }
            BlockItem::S(s) => {
                let news = self.reslove_statement(s)?;
                Ok(BlockItem::S(news))
            }
        }
    }
    fn reslove_dec(&mut self, d: &Declaration) -> Result<Declaration, String> {
        match d {
            Declaration::Variable(v) => {
                let new_v = self.resolve_var_decl(v)?;
                Ok(Declaration::Variable(new_v))
            }
            Declaration::Fun(f) => {
                if f.body.is_some() {
                    // 这是一个嵌套函数定义，非法！
                    return Err(format!(
                        "Nested function definitions are not allowed: {}",
                        f.name
                    ));
                }
                // 这是一个函数内的函数声明，是合法的
                let new_f = self.reslove_function_decl(f)?;
                Ok(Declaration::Fun(new_f))
            }
        }
    }
    fn resolve_var_decl(&mut self, v: &VarDecl) -> Result<VarDecl, String> {
        //这里有个严重的问题，比如 "int foo(int a) {int a = 5;return a;}",这样是不允许的,
        println!("resolve_var_decl {:?}", self.env_vec);
        //因为这里只检查了当前环境，这里的问题是要向上查找，但是好像又不能查找全局环境,只能找这个函数内的环境？
        if self.check_variable_in_current_env(&v.name) {
            return Err(format!("Duplicate variable declaration: {}", v.name));
        }
        let new_name = self.name_gen.new_variable_name(v.name.clone());
        self.insert_new_variable(
            v.name.clone(),
            Info {
                has_linkage: false,
                name: new_name.clone(),
            },
        );
        let new_init = match &v.init {
            Some(e) => Some(self.reslove_exp(e)?),
            None => None,
        };
        Ok(VarDecl {
            name: new_name,
            init: new_init,
        })
    }
    fn reslove_statement(&mut self, d: &Statement) -> Result<Statement, String> {
        match d {
            Statement::Expression(e) => {
                let new_exp = self.reslove_exp(e)?;
                Ok(Statement::Expression(new_exp))
            }
            Statement::Null => Ok(Statement::Null),
            Statement::Return(e) => {
                let new_exp = self.reslove_exp(e)?;
                Ok(Statement::Return(new_exp))
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                let new_c = self.reslove_exp(condition)?;
                let new_left = self.reslove_statement(then_stmt)?;
                let new_right;
                if else_stmt.is_none() {
                    new_right = None;
                } else {
                    let s = self.reslove_statement(&else_stmt.clone().unwrap())?;
                    new_right = Some(Box::new(s));
                }
                Ok(Statement::If {
                    condition: new_c,
                    then_stmt: Box::new(new_left),
                    else_stmt: new_right,
                })
            }
            Statement::Compound(b) => {
                let b = self.reslove_block(b)?;
                Ok(Statement::Compound(b))
            }
            Statement::Break(n) => Ok(Statement::Break(n.clone())),
            Statement::Continue(n) => Ok(Statement::Continue(n.clone())),
            Statement::While {
                condition, body, ..
            } => {
                let new_c = self.reslove_exp(condition)?;
                let new_body = self.reslove_statement(body)?;
                Ok(Statement::While {
                    condition: new_c,
                    body: Box::new(new_body),
                    label: None,
                })
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                let new_c = self.reslove_exp(condition)?;
                let new_body = self.reslove_statement(body)?;
                Ok(Statement::DoWhile {
                    body: Box::new(new_body),
                    condition: new_c,
                    label: None,
                })
            }
            Statement::For {
                init,
                condition,
                post,
                body,
                ..
            } => {
                let env_for = HashMap::new();
                self.env_vec.push(env_for);
                let new_init = self.reslove_forinit(init)?;
                let new_c;
                if let Some(item_c) = condition {
                    new_c = Some(self.reslove_exp(item_c)?);
                } else {
                    new_c = None;
                }
                let new_post;
                if let Some(item_post) = post {
                    new_post = Some(self.reslove_exp(item_post)?);
                } else {
                    new_post = None;
                }
                let new_body = self.reslove_statement(&body)?;
                self.env_vec.pop();
                Ok(Statement::For {
                    init: new_init,
                    condition: new_c,
                    post: new_post,
                    body: Box::new(new_body),
                    label: None,
                })
            }
        }
    }
    fn reslove_forinit(&mut self, init: &ForInit) -> Result<ForInit, String> {
        match init {
            ForInit::InitDecl(d) => {
                let new_d = self.resolve_var_decl(d)?;
                Ok(ForInit::InitDecl(new_d))
            }
            ForInit::InitExp(e) => {
                if let Some(item) = e {
                    let new_e = self.reslove_exp(item)?;
                    Ok(ForInit::InitExp(Some(new_e)))
                } else {
                    Ok(ForInit::InitExp(None))
                }
            }
        }
    }

    fn reslove_exp(&mut self, e: &Expression) -> Result<Expression, String> {
        match e {
            Expression::Assignment { left, right } => match &**left {
                Expression::Var(_) => {
                    let new_l = self.reslove_exp(left)?;
                    let new_r = self.reslove_exp(right)?;
                    Ok(Expression::Assignment {
                        left: Box::new(new_l),
                        right: Box::new(new_r),
                    })
                }
                _ => {
                    return Err("Invalid lvaue!".to_string());
                }
            },
            Expression::Var(id) => {
                let (info, _) = self.find_variable_in_env(id);
                if let Some(item) = info {
                    return Ok(Expression::Var(item.name.clone()));
                } else {
                    return Err("Undeclared variable!".to_string());
                }
            }
            Expression::Binary { op, left, right } => {
                let new_l = self.reslove_exp(left)?;
                let new_r = self.reslove_exp(right)?;
                Ok(Expression::Binary {
                    op: op.clone(),
                    left: Box::new(new_l),
                    right: Box::new(new_r),
                })
            }
            Expression::Unary { op, exp } => {
                let new_e = self.reslove_exp(exp)?;
                Ok(Expression::Unary {
                    op: op.clone(),
                    exp: Box::new(new_e),
                })
            }
            Expression::Constant(i) => Ok(Expression::Constant(*i)),
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                let new_c = self.reslove_exp(condition)?;
                let new_left = self.reslove_exp(left)?;
                let new_right = self.reslove_exp(right)?;

                Ok(Expression::Conditional {
                    condition: Box::new(new_c),
                    left: Box::new(new_left),
                    right: Box::new(new_right),
                })
            }
            Expression::FuncCall { name, args } => {
                let (info, _) = self.find_variable_in_env(name);
                if let Some(r) = info {
                    let new_name = r.name.clone();
                    let mut new_args = Vec::new();
                    for arg in args {
                        let new_e = self.reslove_exp(arg)?;
                        new_args.push(new_e);
                    }
                    return Ok(Expression::FuncCall {
                        name: new_name.clone(),
                        args: new_args,
                    });
                } else {
                    return Err(format!("未声明函数!"));
                }
            }
        }
    }
    fn find_variable_in_env(&self, name: &str) -> (Option<&Info>, bool) {
        // 检查当前作用域
        if let Some(current_scope) = self.env_vec.last() {
            if let Some(info) = current_scope.get(name) {
                return (Some(info), true); // 在当前作用域找到
            }
        }
        // 检查外部作用域
        for scope in self.env_vec.iter().rev().skip(1) {
            if let Some(info) = scope.get(name) {
                return (Some(info), false); // 在外部作用域找到
            }
        }
        (None, false) // 任何地方都没找到
    }
    fn check_variable_in_current_env(&self, name: &str) -> bool {
        let m = self.env_vec.last();
        if let Some(item) = m {
            return item.contains_key(name);
        }
        false
    }

    fn insert_new_variable(&mut self, old: String, new: Info) {
        let m = self.env_vec.last_mut();
        if let Some(item) = m {
            item.insert(old, new);
        }
    }
}

//  src/frontend/reslove_var.rs

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UniqueNameGenerator;
    use crate::frontend::c_ast::Program;
    use crate::frontend::{lexer::Lexer, parser::Parser};

    // 这是一个辅助函数，它将C代码字符串走完 词法分析 -> 语法分析 -> 变量解析 的完整流程
    // 这比只测试 ResloveVar 更接近集成测试，能发现更多问题。
    fn run_resolver_on_string(c_code: &str) -> Result<Program, String> {
        // 1. 词法分析
        let lexer = Lexer::new();
        let tokens = lexer.lex(c_code)?;

        // 2. 语法分析
        let parser = Parser::new(tokens);
        let ast = parser.parse()?;

        // 3. 变量解析 (这是我们真正要测试的部分)
        let mut name_gen = UniqueNameGenerator::new();
        let mut resolver = ResloveVar::new(&mut name_gen);
        resolver.reslove_prgram(&ast)
    }

    // --- 成功案例 (Happy Paths) ---

    #[test]
    fn test_simple_variable() {
        let result = run_resolver_on_string("int main() { int a = 1; return a; }");
        assert!(result.is_ok(), "解析应成功，但失败了: {:?}", result);

        // 我们可以更进一步，检查AST是否真的被修改了
        let resolved_ast = result.unwrap();
        let main_func = &resolved_ast.functions[0];
        let body = main_func.body.as_ref().unwrap();

        // 检查变量声明
        if let BlockItem::D(Declaration::Variable(var_decl)) = &body.0[0] {
            assert_ne!(var_decl.name, "a", "变量 'a' 应该被重命名");
        } else {
            panic!("期望第一个块内元素是变量声明");
        }

        // 检查 return 语句
        if let BlockItem::S(Statement::Return(Expression::Var(var_name))) = &body.0[1] {
            assert_ne!(var_name, "a", "return 语句中的 'a' 应该被重命名");
        } else {
            panic!("期望第二个块内元素是 return 语句");
        }
    }

    #[test]
    fn test_scope_shadowing() {
        let result = run_resolver_on_string("int main() { int a = 1; { int a = 2; } return a; }");
        assert!(result.is_ok(), "解析应成功，但失败了: {:?}", result);

        // 断言 return a 返回的是外部的 a
        let resolved_ast = result.unwrap();
        let main_func = &resolved_ast.functions[0];
        let main_body = main_func.body.as_ref().unwrap();

        let outer_a_new_name =
            if let BlockItem::D(Declaration::Variable(var_decl)) = &main_body.0[0] {
                var_decl.name.clone()
            } else {
                panic!("Expected outer variable declaration");
            };

        let returned_var_name =
            if let BlockItem::S(Statement::Return(Expression::Var(var_name))) = &main_body.0[2] {
                var_name.clone()
            } else {
                panic!("Expected return statement");
            };

        assert_eq!(
            outer_a_new_name, returned_var_name,
            "Return 语句应该引用外部作用域的 'a'"
        );
    }

    #[test]
    fn test_legal_function_redeclaration() {
        let code = "int foo(); int foo(); int main() { return foo(); }";
        let result = run_resolver_on_string(code);
        assert!(
            result.is_ok(),
            "合法的函数重声明不应报错，但出错了: {:?}",
            result
        );
    }

    // --- 失败案例 (Error Cases) ---

    #[test]
    fn test_duplicate_variable_in_same_scope() {
        let result = run_resolver_on_string("int main() { int a; int a; }");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Duplicate variable declaration")
        );
    }

    #[test]
    fn test_function_shadows_variable_in_same_scope() {
        let result = run_resolver_on_string("int main() { int foo; int foo(); }");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "重复声明! foo");
    }

    #[test]
    fn test_variable_shadows_function_in_same_scope() {
        let result = run_resolver_on_string("int main() { int foo(); int foo; }");
        assert!(result.is_err());
        // 这里的错误信息取决于你的实现，"Duplicate variable declaration" 是合理的
        assert!(
            result
                .unwrap_err()
                .contains("Duplicate variable declaration")
        );
    }

    #[test]
    fn test_duplicate_parameter_name() {
        let result = run_resolver_on_string("int add(int x, int x) { return 1; }");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Duplicate variable declaration in add params")
        );
    }

    #[test]
    fn test_use_undeclared_variable() {
        let result = run_resolver_on_string("int main() { return x; }");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Undeclared variable!");
    }

    #[test]
    fn test_call_undeclared_function() {
        let result = run_resolver_on_string("int main() { return foo(); }");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "未声明函数!");
    }

    #[test]
    fn test_nested_function_definition_is_illegal() {
        // 前提：你已经在 reslove_dec 中添加了对嵌套函数定义的检查
        let result = run_resolver_on_string("int main() { int bar() { return 1; } }");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Nested function definitions are not allowed")
        );
    }
}
