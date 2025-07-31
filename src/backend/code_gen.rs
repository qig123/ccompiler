// backend/code_gen.rs

use crate::backend::assembly_ast::{
    BinaryOp, ConditionCode, Function, Instruction, Operand, Program, Reg, UnaryOp,
};
use crate::frontend::type_checking::SymbolInfo;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};

// 将本地标签前缀定义为常量，便于修改。
const LOCAL_LABEL_PREFIX: &str = ".L";

/// x86-64 指令后缀（表示操作数大小）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionSuffix {
    Byte, // 8位，例如 %al
    Long, // 32位，例如 %eax (对应 'l' 后缀)
    Q,    //64
}

pub struct CodeGenerator<'a> {
    tables: &'a HashMap<String, SymbolInfo>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(tables: &'a HashMap<String, SymbolInfo>) -> Self {
        CodeGenerator { tables }
    }

    pub fn generate_program_to_file(
        &self,
        program: &Program,
        file_name: &str,
    ) -> Result<(), String> {
        let file = File::create(file_name).map_err(|e| format!("无法创建文件: {}", e))?;
        let mut writer = BufWriter::new(file);
        self.emit_program(program, &mut writer)
            .map_err(|e| e.to_string())
    }

    fn emit_program(&self, program: &Program, writer: &mut impl Write) -> io::Result<()> {
        for function in &program.functions {
            self.emit_function(function, writer)?;
            writeln!(writer)?; // 函数之间添加空行以提高可读性
        }
        // 这个指令告诉链接器栈是不可执行的，这是一个好的安全实践。
        writeln!(writer, "    .section .note.GNU-stack,\"\",@progbits")?;
        Ok(())
    }

    fn emit_function(&self, function: &Function, writer: &mut impl Write) -> io::Result<()> {
        // --- 函数元信息 ---
        writeln!(writer, "    .globl {}", function.name)?;
        writeln!(writer, "{}:", function.name)?;

        // --- 函数序言 ---
        self.emit_indented("pushq %rbp", writer)?;
        self.emit_indented("movq %rsp, %rbp", writer)?;

        // --- 函数体 ---
        for instruction in &function.instructions {
            self.emit_instruction(instruction, writer)?;
        }

        Ok(())
    }

    fn emit_instruction(
        &self,
        instruction: &Instruction,
        writer: &mut impl Write,
    ) -> io::Result<()> {
        match instruction {
            Instruction::Mov { src, dst } => {
                // 特殊情况：movzbl %al, %eax
                // 这是我们将字节零扩展为长整型的方式。
                if let (Operand::Register(Reg::AX), Operand::Register(Reg::AX)) = (src, dst) {
                    self.emit_indented("movzbl %al, %eax", writer)
                } else {
                    // movl 用于32位（Long）操作数。
                    let line = format!(
                        "movl {}, {}",
                        self.format_operand(src, InstructionSuffix::Long),
                        self.format_operand(dst, InstructionSuffix::Long)
                    );
                    self.emit_indented(&line, writer)
                }
            }
            Instruction::Unary { op, operand } => {
                let (mnemonic, suffix) = match op {
                    UnaryOp::Neg => ("neg", "l"),
                    UnaryOp::Complement => ("not", "l"),
                };
                let line = format!(
                    "{}{} {}",
                    mnemonic,
                    suffix,
                    self.format_operand(operand, InstructionSuffix::Long)
                );
                self.emit_indented(&line, writer)
            }
            Instruction::AllocateStack(size) => {
                // 栈分配/释放使用64位（Quad）寄存器。
                self.emit_indented(&format!("subq ${}, %rsp", size), writer)
            }
            Instruction::Ret => {
                // 这是函数尾声
                self.emit_indented("movq %rbp, %rsp", writer)?;
                self.emit_indented("popq %rbp", writer)?;
                self.emit_indented("ret", writer)
            }
            Instruction::Binary {
                op,
                left_operand,
                right_operand,
            } => {
                let (mnemonic, suffix) = match op {
                    BinaryOp::Add => ("add", "l"),
                    BinaryOp::Subtract => ("sub", "l"),
                    BinaryOp::Multiply => ("imul", "l"),
                };
                let src = self.format_operand(left_operand, InstructionSuffix::Long);
                let dst = self.format_operand(right_operand, InstructionSuffix::Long);
                self.emit_indented(&format!("{}{} {}, {}", mnemonic, suffix, src, dst), writer)
            }
            Instruction::Idiv(operand) => {
                let opr = self.format_operand(operand, InstructionSuffix::Long);
                self.emit_indented(&format!("idivl {}", opr), writer)
            }
            Instruction::Cdq => self.emit_indented("cdq", writer),
            Instruction::Cmp { operand1, operand2 } => {
                let opr1 = self.format_operand(operand1, InstructionSuffix::Long);
                let opr2 = self.format_operand(operand2, InstructionSuffix::Long);
                self.emit_indented(&format!("cmpl {}, {}", opr1, opr2), writer)
            }
            Instruction::Jmp(name) => {
                self.emit_indented(&format!("jmp {}{}", LOCAL_LABEL_PREFIX, name), writer)
            }
            Instruction::JmpCC { condtion, target } => {
                let c = self.format_condition(condtion);
                self.emit_indented(&format!("j{} {}{}", c, LOCAL_LABEL_PREFIX, target), writer)
            }
            Instruction::SetCC { conditin, operand } => {
                // SetCC 现在只对寄存器的字节形式进行操作。
                let c = self.format_condition(conditin);
                let opr = self.format_operand(operand, InstructionSuffix::Byte);
                self.emit_indented(&format!("set{} {}", c, opr), writer)
            }
            Instruction::Label(t) => {
                // 标签不缩进。
                writeln!(writer, "{}{}:", LOCAL_LABEL_PREFIX, t)
            }
            Instruction::DeallocateStack(i) => {
                self.emit_indented(&format!("addq ${} ,%rsp", i), writer)
            }
            Instruction::Push(operand) => {
                let opr = self.format_operand(operand, InstructionSuffix::Q);
                self.emit_indented(&format!("pushq {} ", opr), writer)
            }
            Instruction::Call(name) => {
                if self.tables.contains_key(name) {
                    let r = self.tables.get(name).unwrap();
                    if r.defined {
                        self.emit_indented(&format!("call {} ", name), writer)
                    } else {
                        self.emit_indented(&format!("call {}@PLT", name), writer)
                    }
                } else {
                    self.emit_indented(&format!("call {}@PLT", name), writer)
                }
            }
        }
    }

    // --- 辅助函数 ---

    /// 写入带标准缩进的一行。
    fn emit_indented(&self, line: &str, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "    {}", line)
    }

    /// 格式化操作数以用于汇编输出。
    fn format_operand(&self, operand: &Operand, size: InstructionSuffix) -> String {
        match operand {
            Operand::Imm(val) => format!("${}", val),
            Operand::Register(reg) => self.format_reg(reg, size),
            Operand::Stack(offset) => format!("{}(%rbp)", offset),
            Operand::Pseudo(_) => {
                panic!("伪寄存器不应出现在最终代码生成阶段");
            }
        }
    }

    /// 返回条件码对应的汇编后缀。
    fn format_condition(&self, code: &ConditionCode) -> &'static str {
        match code {
            ConditionCode::E => "e",
            ConditionCode::NE => "ne",
            ConditionCode::G => "g",
            ConditionCode::GE => "ge",
            ConditionCode::L => "l",
            ConditionCode::LE => "le",
        }
    }

    /// 根据大小格式化寄存器，返回正确的名称。
    pub fn format_reg(&self, reg: &Reg, size: InstructionSuffix) -> String {
        let name = match (reg, size) {
            // --- 64-bit (Quad-word) Registers ---
            (Reg::AX, InstructionSuffix::Q) => "%rax",
            (Reg::CX, InstructionSuffix::Q) => "%rcx",
            (Reg::DX, InstructionSuffix::Q) => "%rdx",
            (Reg::DI, InstructionSuffix::Q) => "%rdi",
            (Reg::SI, InstructionSuffix::Q) => "%rsi",
            (Reg::R8, InstructionSuffix::Q) => "%r8",
            (Reg::R9, InstructionSuffix::Q) => "%r9",
            (Reg::R10, InstructionSuffix::Q) => "%r10",
            (Reg::R11, InstructionSuffix::Q) => "%r11",

            // --- 32-bit (Long-word) Registers ---
            (Reg::AX, InstructionSuffix::Long) => "%eax",
            (Reg::CX, InstructionSuffix::Long) => "%ecx",
            (Reg::DX, InstructionSuffix::Long) => "%edx",
            (Reg::DI, InstructionSuffix::Long) => "%edi",
            (Reg::SI, InstructionSuffix::Long) => "%esi",
            (Reg::R8, InstructionSuffix::Long) => "%r8d",
            (Reg::R9, InstructionSuffix::Long) => "%r9d",
            (Reg::R10, InstructionSuffix::Long) => "%r10d",
            (Reg::R11, InstructionSuffix::Long) => "%r11d",

            // --- 8-bit (Byte) Registers ---
            (Reg::AX, InstructionSuffix::Byte) => "%al",
            (Reg::CX, InstructionSuffix::Byte) => "%cl",
            (Reg::DX, InstructionSuffix::Byte) => "%dl",
            (Reg::DI, InstructionSuffix::Byte) => "%dil",
            (Reg::SI, InstructionSuffix::Byte) => "%sil",
            (Reg::R8, InstructionSuffix::Byte) => "%r8b",
            (Reg::R9, InstructionSuffix::Byte) => "%r9b",
            (Reg::R10, InstructionSuffix::Byte) => "%r10b",
            (Reg::R11, InstructionSuffix::Byte) => "%r11b",
            // 注意：BP和SP没有标准的8位版本(bpl/spl需要特殊REX前缀，通常不直接这样用)
            // 所以我们不在这里包含它们，让它 fall through 到 panic

            // 捕获所有未处理的组合，这样如果未来添加新寄存器或大小，
            // 编译器会强制我们在这里处理它。
            // _ => panic!(
            //     "Unsupported register/size combination: {:?}/{:?}",
            //     reg, size
            // ),
        };
        name.to_string()
    }
}
