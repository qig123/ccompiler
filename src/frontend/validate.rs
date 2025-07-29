use std::collections::HashMap;

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{BlockItem, Declaration, Expression, Function, Program, Statement},
};

//src/frontend/validate.rs
pub struct Validate<'a> {
    variable_map: HashMap<String, String>,
    name_gen: &'a mut UniqueNameGenerator,
}
impl<'a> Validate<'a> {
    pub fn new(g: &'a mut UniqueNameGenerator) -> Self {
        Validate {
            variable_map: HashMap::new(),
            name_gen: g,
        }
    }
    pub fn reslove_prgram(&mut self, ast: &Program) -> Result<Program, String> {
        let mut fs: Vec<Function> = Vec::new();
        for f in &ast.functions {
            let new_f = self.reslove_function(f)?;
            fs.push(new_f);
        }
        Ok(Program { functions: fs })
    }
    fn reslove_function(&mut self, f: &Function) -> Result<Function, String> {
        let mut bs: Vec<BlockItem> = Vec::new();

        for b in &f.body {
            let b = self.reslove_blockitem(b)?;
            bs.push(b);
        }
        Ok(Function {
            name: f.name.clone(),
            parameters: f.parameters.clone(),
            body: bs,
        })
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
        if self.variable_map.contains_key(&d.name) {
            return Err("Duplicate variable declaration".to_string());
        }
        let new_name = self.name_gen.new_variable_name(d.name.clone());
        self.variable_map.insert(d.name.clone(), new_name.clone());
        match &d.init {
            None => Ok(Declaration {
                name: new_name,
                init: None,
            }),
            Some(box_e) => {
                let new_e = self.reslove_exp(&*box_e)?;
                Ok(Declaration {
                    name: new_name.clone(),
                    init: Some(Box::new(new_e)),
                })
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
            _ => panic!(),
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
                if self.variable_map.contains_key(id) {
                    let new_id = self.variable_map.get(id).unwrap();
                    return Ok(Expression::Var(new_id.clone()));
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
            _ => {
                panic!()
            }
        }
    }
}
