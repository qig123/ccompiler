// src/common.rs

pub trait AstNode {
    fn pretty_print(&self, printer: &mut PrettyPrinter);
}

pub struct PrettyPrinter {
    pub indent_level: usize,
}

impl PrettyPrinter {
    pub fn new() -> Self {
        PrettyPrinter { indent_level: 0 }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn unindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    pub fn prefix(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    pub fn writeln(&self, text: &str) {
        println!("{}{}", self.prefix(), text);
    }
}
