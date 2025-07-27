// src/common.rs

use std::io;

pub trait AstNode {
    fn pretty_print(&self, printer: &mut PrettyPrinter);
}

pub struct PrettyPrinter<'a> {
    indent_level: usize,
    writer: &'a mut dyn io::Write,
}

impl<'a> PrettyPrinter<'a> {
    pub fn new(writer: &'a mut dyn io::Write) -> Self {
        PrettyPrinter {
            indent_level: 0,
            writer,
        }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn unindent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    fn prefix(&self) -> String {
        "  ".repeat(self.indent_level)
    }

    pub fn writeln(&mut self, text: &str) -> io::Result<()> {
        writeln!(self.writer, "{}{}", self.prefix(), text)
    }

    // pub fn write_raw(&mut self, text: &str) -> io::Result<()> {
    //     write!(self.writer, "{}", text)
    // }
}
