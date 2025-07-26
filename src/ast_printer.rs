/// 用于管理打印时的缩进
pub struct PrettyPrinter {
    pub indent_level: usize,
}

impl PrettyPrinter {
    pub fn new() -> Self {
        PrettyPrinter { indent_level: 0 }
    }

    /// 增加缩进
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// 减少缩进
    pub fn unindent(&mut self) {
        self.indent_level -= 1;
    }

    /// 生成当前缩进级别的字符串
    pub fn prefix(&self) -> String {
        "  ".repeat(self.indent_level) // 使用两个空格作为一级缩进
    }

    /// 打印一行带缩进的文本
    pub fn writeln(&self, text: &str) {
        println!("{}{}", self.prefix(), text);
    }
}
