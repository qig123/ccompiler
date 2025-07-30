//src/frontend/reslove_var.rs
use std::collections::HashMap;

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{
        Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement,
    },
};

pub struct ResloveVar<'a> {
    variable_map: Vec<HashMap<String, String>>, //env chain
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
            let new_f = self.reslove_function(f)?;
            fs.push(new_f);
        }
        Ok(Program { functions: fs })
    }
    fn reslove_function(&mut self, f: &FunDecl) -> Result<FunDecl, String> {
        let b = self.reslove_block(&f.body.clone().unwrap())?;
        Ok(FunDecl {
            name: f.name.clone(),
            parameters: f.parameters.clone(),
            body: Some(b),
        })
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
            Declaration::Variable(f) => {
                if self.check_variable_in_current_env(&f.name) {
                    return Err("Duplicate variable declaration".to_string());
                }
                let new_name = self.name_gen.new_variable_name(f.name.clone());
                self.insert_new_variable(f.name.clone(), new_name.clone());
                panic!()
            }
            _ => panic!(),
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
                // let new_d = self.reslove_dec(d)?;
                // Ok(ForInit::InitDecl(new_d))
                panic!()
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
                if let Some(item) = self.find_variable_in_env(id) {
                    return Ok(Expression::Var(item));
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
            _ => panic!(),
        }
    }
    fn find_variable_in_env(&self, name: &str) -> Option<String> {
        for m in self.variable_map.iter().rev() {
            if m.contains_key(name) {
                return m.get(name).cloned();
            }
        }
        None
    }
    fn check_variable_in_current_env(&self, name: &str) -> bool {
        let m = self.variable_map.last();
        if let Some(item) = m {
            return item.contains_key(name);
        }
        false
    }

    fn insert_new_variable(&mut self, old: String, new: String) {
        let m = self.variable_map.last_mut();
        if let Some(item) = m {
            item.insert(old, new);
        }
    }
}
