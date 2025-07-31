//src/frontend/reslove_var.rs
use std::collections::HashMap;

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{
        Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement, VarDecl,
    },
};
pub struct Info {
    has_linkage: bool,
    name: String,
}

pub struct ResloveVar<'a> {
    variable_map: Vec<HashMap<String, Info>>,
    name_gen: &'a mut UniqueNameGenerator,
}
impl<'a> ResloveVar<'a> {
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        ResloveVar {
            variable_map: Vec::new(),
            name_gen: g,
        }
    }
    pub fn reslove_prgram(&mut self, ast: &Program) -> Result<Program, String> {
        let mut fs: Vec<FunDecl> = Vec::new();
        for f in &ast.functions {
            let new_f = self.reslove_function_decl(f)?;
            fs.push(new_f);
        }
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
        self.variable_map.push(env_params);
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

        if let Some(b) = &f.body {
            let b = self.reslove_block(b)?;
            self.variable_map.pop();
            Ok(FunDecl {
                name: f.name.clone(),
                parameters: new_params,
                body: Some(b),
            })
        } else {
            self.variable_map.pop();
            Ok(FunDecl {
                name: f.name.clone(),
                parameters: new_params,
                body: None,
            })
        }
    }
    fn reslove_block(&mut self, blocks: &Block) -> Result<Block, String> {
        let map = HashMap::new();
        self.variable_map.push(map);
        let mut bs: Vec<BlockItem> = Vec::new();

        for b in &blocks.0 {
            let b = self.reslove_blockitem(&b)?;
            bs.push(b);
        }
        self.variable_map.pop();
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
                if self.check_variable_in_current_env(&v.name) {
                    return Err("Duplicate variable declaration".to_string());
                }
                let new_name = self.name_gen.new_variable_name(v.name.clone());
                self.insert_new_variable(
                    v.name.clone(),
                    Info {
                        has_linkage: false,
                        name: new_name.clone(),
                    },
                );
                match &v.init {
                    None => Ok(Declaration::Variable(VarDecl {
                        name: new_name,
                        init: None,
                    })),
                    Some(box_e) => {
                        let new_e = self.reslove_exp(&box_e)?;
                        Ok(Declaration::Variable(VarDecl {
                            name: new_name,
                            init: Some(new_e),
                        }))
                    }
                }
            }
            //这是函数内的函数声明,这里能否判断出一定是函数声明，而不存在函数定义？
            Declaration::Fun(f) => {
                let new_f = self.reslove_function_decl(f)?;
                Ok(Declaration::Fun(new_f))
            }
        }
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
                self.variable_map.push(env_for);
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
                self.variable_map.pop();
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
                //这里应该是调用 reslove_dec的，但是我不会调用，参数好像不兼容
                if self.check_variable_in_current_env(&d.name) {
                    return Err("Duplicate variable declaration".to_string());
                }
                let new_name = self.name_gen.new_variable_name(d.name.clone());
                self.insert_new_variable(
                    d.name.clone(),
                    Info {
                        has_linkage: false,
                        name: new_name.clone(),
                    },
                );
                match &d.init {
                    None => Ok(ForInit::InitDecl(VarDecl {
                        name: new_name,
                        init: None,
                    })),
                    Some(box_e) => {
                        let new_e = self.reslove_exp(&box_e)?;
                        Ok(ForInit::InitDecl(VarDecl {
                            name: new_name,
                            init: Some(new_e),
                        }))
                    }
                }
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
        let mut find_count = 0;
        for m in self.variable_map.iter().rev() {
            find_count += 1;
            if m.contains_key(name) {
                let is_from_current;
                if find_count > 1 {
                    is_from_current = false;
                } else {
                    is_from_current = true;
                }
                return (m.get(name), is_from_current);
            }
        }
        let is_from_current;
        if find_count > 1 {
            is_from_current = false;
        } else {
            is_from_current = true;
        }
        return (None, is_from_current);
    }
    fn check_variable_in_current_env(&self, name: &str) -> bool {
        let m = self.variable_map.last();
        if let Some(item) = m {
            return item.contains_key(name);
        }
        false
    }

    fn insert_new_variable(&mut self, old: String, new: Info) {
        let m = self.variable_map.last_mut();
        if let Some(item) = m {
            item.insert(old, new);
        }
    }
}
