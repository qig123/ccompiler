// src/frontend/resolve_ident.rs

//! **标识符解析（Identifier Resolution）**
//!
//! 该模块负责在抽象语法树（AST）上执行语义分析的核心任务之一：解析所有标识符的引用。
//! 这意味着，对于在代码中使用的每个变量或函数名，我们需要明确它指向的是哪个声明。
//!
//! ## 主要职责
//!
//! 1.  **作用域管理**:
//!     -   编译器为代码中的每个作用域（例如，全局范围、函数体、代码块）维护一个独立的符号表。
//!     -   这些作用域以栈的形式（`env_vec`）进行管理。进入新作用域（如函数或块）时，新的符号表会被压入栈顶；退出时则弹出。
//!
//! 2.  **标识符声明与查找**:
//!     -   当遇到变量或函数声明时，会将其信息（`IdentifierInfo`）添加到当前作用域的符号表中。
//!     -   在解析表达式中的标识符时，会从当前作用域开始，逐级向上（向外层作用域）查找其声明。
//!     -   此过程确保了局部变量可以“遮蔽”（shadow）外部同名变量。
//!
//! 3.  **名称修饰（Name Mangling）**:
//!     -   为了避免不同作用域中的同名局部变量在后续处理（如代码生成）中发生冲突，我们为每个非全局变量生成一个唯一的内部名称（例如，`a` -> `a.0`, `a.1`）。
//!     -   `UniqueNameGenerator` 负责生成这些不会重复的名称。
//!
//! 4.  **错误处理**:
//!     -   捕捉常见的语义错误，例如：
//!         -   在同一作用域内重复定义变量或函数。
//!         -   引用未声明的变量。
//!         -   在函数参数和函数体顶层作用域之间重复定义变量。
//!         -   非法地在函数内部定义另一个函数。

use std::collections::HashMap;

use crate::{
    UniqueNameGenerator,
    frontend::c_ast::{
        Block, BlockItem, Declaration, Expression, ForInit, FunDecl, Program, Statement,
        StorageClass, VarDecl,
    },
};
#[derive(Debug, PartialEq, Clone)]
pub enum ScopeKind {
    File,  // 文件作用域 (全局作用域)
    Block, // 块作用域 (函数内、循环内等)
}

/// 存储在符号表中的标识符信息。
#[derive(Debug, Clone)]
pub struct IdentifierInfo {
    /// 该标识符是否具有链接属性。
    /// - `true` 表示它是一个函数或全局变量，在整个程序中是唯一的。
    /// - `false` 表示它是一个局部变量（包括函数参数），仅在当前作用域内有效。
    has_linkage: bool,
    /// 经过名称修饰后的唯一标识符。
    mangled_name: String,
}

/// 标识符解析器的状态机。
#[derive(Debug)]
pub struct IdentifierResolver<'a> {
    /// 环境栈，用于管理作用域。每个 `HashMap` 代表一个作用域的符号表。
    /// `String` 是原始的标识符名称，`IdentifierInfo` 是其解析后的信息。
    env_stack: Vec<HashMap<String, IdentifierInfo>>,
    /// 用于生成唯一变量名的工具。
    name_generator: &'a mut UniqueNameGenerator,
}

impl<'a> IdentifierResolver<'a> {
    /// 创建一个新的标识符解析器。
    pub fn new(name_generator: &'a mut UniqueNameGenerator) -> Self {
        IdentifierResolver {
            env_stack: Vec::new(),
            name_generator,
        }
    }

    /// 解析整个程序（即AST的根节点）。
    pub fn resolve_program(&mut self, ast: &Program) -> Result<Program, String> {
        // 创建并推入全局作用域
        self.env_stack.push(HashMap::new());

        let mut resolved_functions: Vec<Declaration> = Vec::new();
        for f in &ast.declarations {
            let resolved_f = self.resolve_declaration(f, ScopeKind::File)?;
            resolved_functions.push(resolved_f);
        }

        // 完成解析后，弹出全局作用域
        self.env_stack.pop();
        Ok(Program {
            declarations: resolved_functions,
        })
    }

    /// 解析函数声明或定义。
    fn resolve_function_decl(
        &mut self,
        f: &FunDecl,
        scope_kind: ScopeKind,
    ) -> Result<FunDecl, String> {
        // 根据文本要求，检查块作用域内的 static 函数声明
        if scope_kind == ScopeKind::Block {
            if let Some(StorageClass::Static) = f.storage_class {
                return Err(
                    "Semantic Error: 'static' function declaration is not allowed inside a block."
                        .to_string(),
                );
            }
            // 同时，块作用域内也不允许有函数体
            if f.body.is_some() {
                return Err(
                    "Semantic Error: Function definition is not allowed inside a block."
                        .to_string(),
                );
            }
        }
        if scope_kind == ScopeKind::File {
            // 只有文件作用域的函数声明才应该被插入到全局（最外层）作用域
            let existing_entry = self.find_identifier_in_current_scope(&f.name); // 假设当前是全局作用域
            if let Some(info) = existing_entry {
                if !info.has_linkage {
                    return Err(format!(
                        "Semantic Error: Redeclaration of '{}' as a different kind of symbol.",
                        f.name
                    ));
                }
            } else {
                self.insert_identifier(
                    f.name.clone(),
                    IdentifierInfo {
                        has_linkage: true,
                        mangled_name: f.name.clone(),
                    },
                );
            }
        } else { // scope_kind == ScopeKind::Block
            // 对于块作用域的函数声明 (e.g., extern int foo();)，它是一个引用。
            // 我们不需要在当前块作用域的符号表里为它创建新条目。
            // 因为对它的引用解析，会通过 `resolve_expression` 中的函数调用解析逻辑，
            // 从内到外查找到文件作用域的那个声明。
            // 所以，对于块作用域的函数声明，我们在这里其实什么都不用做。
        }
        // --- 创建函数作用域 ---
        // 此作用域将包含函数参数和函数体的所有局部变量。
        self.env_stack.push(HashMap::new());

        // 解析函数参数
        let mut resolved_params = Vec::new();
        for p_name in &f.parameters {
            // 检查参数名是否在当前（函数）作用域内重复。
            if self.is_identifier_in_current_scope(p_name) {
                return Err(format!(
                    "Semantic Error: Duplicate parameter name '{}' in function '{}'.",
                    p_name, f.name
                ));
            }
            // 为参数生成唯一的内部名称并存入符号表。
            let mangled_name = self.name_generator.new_variable_name(p_name.clone());
            self.insert_identifier(
                p_name.clone(),
                IdentifierInfo {
                    has_linkage: false,
                    mangled_name: mangled_name.clone(),
                },
            );
            resolved_params.push(mangled_name);
        }

        // 解析函数体
        let resolved_body = if let Some(body_block) = &f.body {
            // 直接在包含参数的同一作用域内解析函数体中的条目。
            // 这样可以正确检测出函数体内的变量声明与参数名之间的冲突。
            let mut resolved_items: Vec<BlockItem> = Vec::new();
            for item in &body_block.0 {
                let resolved_item = self.resolve_block_item(item)?;
                resolved_items.push(resolved_item);
            }
            Some(Block(resolved_items))
        } else {
            // 函数只有声明，没有函数体。
            None
        };

        // --- 退出函数作用域 ---
        self.env_stack.pop();

        Ok(FunDecl {
            name: f.name.clone(),
            parameters: resolved_params,
            body: resolved_body,
            storage_class: None,
        })
    }

    /// 解析代码块（Block）。
    /// 一个块会引入一个新的作用域。
    fn resolve_block(&mut self, block: &Block) -> Result<Block, String> {
        self.env_stack.push(HashMap::new()); // 进入新作用域
        let mut resolved_items: Vec<BlockItem> = Vec::new();

        for item in &block.0 {
            let resolved_item = self.resolve_block_item(item)?;
            resolved_items.push(resolved_item);
        }

        self.env_stack.pop(); // 退出作用域
        Ok(Block(resolved_items))
    }

    /// 解析块内的单个条目（声明或语句）。
    fn resolve_block_item(&mut self, item: &BlockItem) -> Result<BlockItem, String> {
        match item {
            BlockItem::D(d) => {
                let new_d = self.resolve_declaration(d, ScopeKind::Block)?;
                Ok(BlockItem::D(new_d))
            }
            BlockItem::S(s) => {
                let new_s = self.resolve_statement(s)?;
                Ok(BlockItem::S(new_s))
            }
        }
    }

    /// 解析声明（变量或函数）。
    fn resolve_declaration(
        &mut self,
        d: &Declaration,
        scope_kind: ScopeKind,
    ) -> Result<Declaration, String> {
        match d {
            Declaration::Variable(v) => {
                let new_v = self.resolve_variable_declaration(v, scope_kind)?;
                Ok(Declaration::Variable(new_v))
            }
            Declaration::Fun(f) => {
                // C语言标准禁止在函数内部定义另一个函数。
                if f.body.is_some() {
                    return Err(format!(
                        "Semantic Error: Nested function definitions are not allowed (function '{}').",
                        f.name
                    ));
                }
                // 函数内的函数声明（原型）是允许的。
                let new_f = self.resolve_function_decl(f, scope_kind)?;
                Ok(Declaration::Fun(new_f))
            }
        }
    }

    /// 解析变量声明。
    fn resolve_variable_declaration(
        &mut self,
        v: &VarDecl,
        scope_kind: ScopeKind,
    ) -> Result<VarDecl, String> {
        match scope_kind {
            ScopeKind::Block => {
                // 检查当前作用域是否已经有同名声明
                if let Some(prev_info) = self.find_identifier_in_current_scope(&v.name) {
                    // 找到了，说明在同一个块里有重复声明，需要检查冲突
                    let current_is_extern = matches!(v.storage_class, Some(StorageClass::Extern));
                    let prev_has_linkage = prev_info.has_linkage;

                    // 唯一的合法情况是：前一个有链接，现在这个也是 extern
                    // 其他情况都冲突：
                    // 1. prev无链接, curr是任何东西 -> 冲突 (e.g., int x; int x; 或 int x; extern int x;)
                    // 2. prev有链接, curr不是extern -> 冲突 (e.g., extern int x; int x;)
                    if !(prev_has_linkage && current_is_extern) {
                        return Err(format!(
                            "Semantic Error: Conflicting declarations for '{}' in the same scope",
                            v.name
                        ));
                    }
                    // 如果兼容 (都是 extern)，我们其实什么都不用做，因为符号表里已经有正确的信息了。
                    // 直接返回即可，或者更新一下AST节点。
                    return Ok(v.clone());
                }

                // 如果当前作用域没有同名声明，我们现在添加它
                match v.storage_class {
                    Some(StorageClass::Extern) => {
                        // 这是一个新的 extern 声明。它有链接性，不重命名。
                        self.insert_identifier(
                            v.name.clone(),
                            IdentifierInfo {
                                has_linkage: true,
                                mangled_name: v.name.clone(),
                            },
                        );
                        Ok(VarDecl {
                            name: v.name.clone(),
                            init: None, // extern 在块作用域不能有 init
                            storage_class: v.storage_class.clone(),
                        })
                    }
                    Some(StorageClass::Static) | None => {
                        // 这是一个新的局部变量（普通或 static）。它无链接，需要重命名。
                        // 它会遮蔽外层同名变量，但这是合法的。
                        let mangled_name = self.name_generator.new_variable_name(v.name.clone());
                        self.insert_identifier(
                            v.name.clone(),
                            IdentifierInfo {
                                has_linkage: false,
                                mangled_name: mangled_name.clone(),
                            },
                        );
                        let new_init = match &v.init {
                            Some(e) => Some(self.resolve_expression(e)?),
                            None => None,
                        };
                        Ok(VarDecl {
                            name: mangled_name,
                            init: new_init,
                            storage_class: v.storage_class.clone(),
                        })
                    }
                }
            }
            ScopeKind::File => {
                if let Some(_prev_info) = self.find_identifier_in_current_scope(&v.name) {
                    // 已经存在一个文件作用域的声明了
                    // 在标识符解析阶段，我们假设它们是兼容的。
                    // 真正的链接性冲突检查（static vs extern）留给类型检查阶段。
                    // 所以这里可以什么都不做，或者只更新AST。
                } else {
                    // 这是第一次遇到这个名字的文件作用域声明
                    self.insert_identifier(
                        v.name.clone(),
                        IdentifierInfo {
                            has_linkage: true,
                            mangled_name: v.name.clone(),
                        },
                    );
                }

                Ok(VarDecl {
                    name: v.name.clone(),
                    init: v.init.clone(),
                    storage_class: v.storage_class.clone(),
                })
            }
        }
    }

    /// 解析语句。
    fn resolve_statement(&mut self, stmt: &Statement) -> Result<Statement, String> {
        match stmt {
            Statement::Expression(e) => {
                let new_exp = self.resolve_expression(e)?;
                Ok(Statement::Expression(new_exp))
            }
            Statement::Return(e) => {
                let new_exp = self.resolve_expression(e)?;
                Ok(Statement::Return(new_exp))
            }
            Statement::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                let new_c = self.resolve_expression(condition)?;
                let new_then = self.resolve_statement(then_stmt)?;
                let new_else = if let Some(es) = else_stmt {
                    Some(Box::new(self.resolve_statement(es)?))
                } else {
                    None
                };
                Ok(Statement::If {
                    condition: new_c,
                    then_stmt: Box::new(new_then),
                    else_stmt: new_else,
                })
            }
            Statement::Compound(b) => {
                // 复合语句（即用 `{}` 包围的块）会创建一个新的作用域。
                let new_b = self.resolve_block(b)?;
                Ok(Statement::Compound(new_b))
            }
            Statement::While {
                condition, body, ..
            } => {
                let new_c = self.resolve_expression(condition)?;
                let new_body = self.resolve_statement(body)?;
                Ok(Statement::While {
                    condition: new_c,
                    body: Box::new(new_body),
                    label: None, // 标签在后续阶段处理
                })
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                let new_body = self.resolve_statement(body)?;
                let new_c = self.resolve_expression(condition)?;
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
                // `for` 循环的初始化部分可以声明变量，它位于一个新的作用域内。
                self.env_stack.push(HashMap::new());
                let new_init = self.resolve_for_init(init)?;
                let new_c = match condition {
                    Some(c) => Some(self.resolve_expression(c)?),
                    None => None,
                };
                let new_post = match post {
                    Some(p) => Some(self.resolve_expression(p)?),
                    None => None,
                };
                let new_body = self.resolve_statement(body)?;
                self.env_stack.pop(); // 退出 `for` 循环作用域

                Ok(Statement::For {
                    init: new_init,
                    condition: new_c,
                    post: new_post,
                    body: Box::new(new_body),
                    label: None,
                })
            }
            // 对于简单语句，无需特殊处理，直接返回克隆即可。
            Statement::Null => Ok(Statement::Null),
            Statement::Break(n) => Ok(Statement::Break(n.clone())),
            Statement::Continue(n) => Ok(Statement::Continue(n.clone())),
        }
    }

    /// 解析 `for` 循环的初始化部分。
    fn resolve_for_init(&mut self, init: &ForInit) -> Result<ForInit, String> {
        match init {
            ForInit::InitDecl(d) => {
                let new_d = self.resolve_variable_declaration(d, ScopeKind::Block)?;
                Ok(ForInit::InitDecl(new_d))
            }
            ForInit::InitExp(Some(e)) => {
                let new_e = self.resolve_expression(e)?;
                Ok(ForInit::InitExp(Some(new_e)))
            }
            ForInit::InitExp(None) => Ok(ForInit::InitExp(None)),
        }
    }

    /// 解析表达式。
    fn resolve_expression(&mut self, e: &Expression) -> Result<Expression, String> {
        match e {
            Expression::Assignment { left, right } => {
                // 确保赋值操作的左侧是一个有效的左值（l-value）。
                // 在我们的简化C语言中，只有变量是有效的左值。
                if !matches!(**left, Expression::Var(_)) {
                    return Err(
                        "Semantic Error: Expression is not assignable (not a valid l-value)."
                            .to_string(),
                    );
                }
                let new_l = self.resolve_expression(left)?;
                let new_r = self.resolve_expression(right)?;
                Ok(Expression::Assignment {
                    left: Box::new(new_l),
                    right: Box::new(new_r),
                })
            }
            Expression::Var(id) => {
                // 这是解析的核心：查找变量的声明。
                let (info, _) = self.find_identifier_in_all_scopes(id);
                if let Some(item) = info {
                    // 查找到后，将AST中的变量名替换为其唯一的、修饰后的名称。
                    Ok(Expression::Var(item.mangled_name.clone()))
                } else {
                    Err(format!(
                        "Semantic Error: Use of undeclared identifier '{}'.",
                        id
                    ))
                }
            }
            Expression::FuncCall { name, args } => {
                // 查找函数声明。
                let (info, _) = self.find_identifier_in_all_scopes(name);
                if let Some(r) = info {
                    // 确保被调用的标识符确实是一个函数。
                    if !r.has_linkage {
                        return Err(format!(
                            "Semantic Error: Called object '{}' is not a function.",
                            name
                        ));
                    }
                    let new_name = r.mangled_name.clone();
                    let mut new_args = Vec::new();
                    for arg in args {
                        new_args.push(self.resolve_expression(arg)?);
                    }
                    Ok(Expression::FuncCall {
                        name: new_name,
                        args: new_args,
                    })
                } else {
                    Err(format!(
                        "Semantic Error: Call to undeclared function '{}'.",
                        name
                    ))
                }
            }
            // 对于其他复合表达式，递归地解析其子表达式。
            Expression::Binary { op, left, right } => {
                let new_l = self.resolve_expression(left)?;
                let new_r = self.resolve_expression(right)?;
                Ok(Expression::Binary {
                    op: op.clone(),
                    left: Box::new(new_l),
                    right: Box::new(new_r),
                })
            }
            Expression::Unary { op, exp } => {
                let new_e = self.resolve_expression(exp)?;
                Ok(Expression::Unary {
                    op: op.clone(),
                    exp: Box::new(new_e),
                })
            }
            Expression::Conditional {
                condition,
                left,
                right,
            } => {
                let new_c = self.resolve_expression(condition)?;
                let new_l = self.resolve_expression(left)?;
                let new_r = self.resolve_expression(right)?;
                Ok(Expression::Conditional {
                    condition: Box::new(new_c),
                    left: Box::new(new_l),
                    right: Box::new(new_r),
                })
            }
            // 常量表达式不需要解析。
            Expression::Constant(i) => Ok(Expression::Constant(*i)),
        }
    }

    // --- 作用域和符号表辅助函数 ---

    /// 从内到外查找所有作用域中的标识符。
    /// 返回找到的标识符信息以及一个布尔值，该值指示是否在最内层作用域找到。
    fn find_identifier_in_all_scopes(&self, name: &str) -> (Option<&IdentifierInfo>, bool) {
        if let Some(current_scope) = self.env_stack.last() {
            if let Some(info) = current_scope.get(name) {
                return (Some(info), true); // 在当前作用域找到
            }
        }
        for scope in self.env_stack.iter().rev().skip(1) {
            if let Some(info) = scope.get(name) {
                return (Some(info), false); // 在外部作用域找到
            }
        }
        (None, false) // 未找到
    }

    /// 仅在当前（最内层）作用域中查找标识符。
    fn find_identifier_in_current_scope(&self, name: &str) -> Option<&IdentifierInfo> {
        self.env_stack.last()?.get(name)
    }

    /// 检查标识符是否存在于当前作用域。
    fn is_identifier_in_current_scope(&self, name: &str) -> bool {
        self.env_stack
            .last()
            .map_or(false, |scope| scope.contains_key(name))
    }

    /// 在当前作用域中插入一个新的标识符。
    fn insert_identifier(&mut self, name: String, info: IdentifierInfo) {
        if let Some(current_scope) = self.env_stack.last_mut() {
            current_scope.insert(name, info);
        }
    }
}
