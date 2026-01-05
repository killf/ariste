use colored::Colorize;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

// Claude Code 风格的 ASCII spinner 字符
const SPINNER_CHARS: &[&str] = &["·", "✻", "✽", "✶", "✳", "✢"];

// 状态消息
const STATUS_MESSAGES: &[&str] = &[
    "Sparkling",
    "Sketching",
    "Thinking",
    "Polishing",
    "Crafting",
];

// 思考块的装饰字符
const THINKING_BORDER: &str = "│";
const THINKING_CORNER_TL: &str = "┌";
const THINKING_CORNER_TR: &str = "┐";
const THINKING_CORNER_BL: &str = "└";
const THINKING_CORNER_BR: &str = "┘";

// 计算字符串的显示宽度（去除ANSI颜色代码）
fn display_width(s: &str) -> usize {
    let mut width = 0;
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // 跳过ANSI转义序列
            if chars.next() == Some('[') {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

pub struct UI {
    spinner_index: usize,
    status_index: usize,
    last_update: Instant,
}

impl UI {
    pub fn new() -> Self {
        Self {
            spinner_index: 0,
            status_index: 0,
            last_update: Instant::now(),
        }
    }

    /// 打印欢迎信息 - Claude Code 风格
    pub fn welcome(workdir: &std::path::Path) {
        println!();
        println!("{} {}", "✦".bright_yellow(), "Welcome to".dimmed());
        println!(
            "{}",
            "  ╔════════════════════════════════════════╗".bright_yellow()
        );
        println!("{}", "  ║".bright_yellow());
        println!(
            "{}",
            format!(
                "  {}  {}  ",
                "║".bright_yellow(),
                "Ariste AI Agent".bright_cyan().bold()
            )
        );
        println!("{}", "  ║".bright_yellow());
        println!(
            "{}",
            "  ╚════════════════════════════════════════╝".bright_yellow()
        );
        println!();
        println!(
            "{} {}",
            "│".dimmed(),
            format!("Working directory: {}", workdir.display()).bright_white()
        );
        println!();
        Self::print_available_commands();
    }

    /// 打印可用命令
    fn print_available_commands() {
        println!("{}", "Available commands:".dimmed());
        println!(
            "  {}{}  {}",
            "/".bright_green(),
            "help".bright_green(),
            "Show this help message".dimmed()
        );
        println!(
            "  {}{}  {}",
            "/".bright_green(),
            "clear".bright_green(),
            "Clear the terminal screen".dimmed()
        );
        println!(
            "  {}{}  {}",
            "/".bright_green(),
            "quit".bright_green(),
            "Exit the program".dimmed()
        );
        println!();
    }

    /// 打印用户输入提示符 - Claude Code 风格
    pub fn prompt() -> String {
        format!("{} ", "⟩".bright_cyan())
    }

    /// 显示正在思考状态 - 带 spinner 动画
    pub fn thinking_start(&mut self) {
        let spinner = SPINNER_CHARS[self.spinner_index];
        let status = STATUS_MESSAGES[self.status_index];

        print!(
            "\r{} {}{} ",
            spinner.bright_yellow(),
            status.bright_yellow(),
            "…".dimmed()
        );
        stdout().flush().ok();

        // 更新 spinner 索引
        if self.last_update.elapsed() >= Duration::from_millis(150) {
            self.spinner_index = (self.spinner_index + 1) % SPINNER_CHARS.len();
            // 偶尔切换状态消息
            if self.spinner_index == 0 {
                self.status_index = (self.status_index + 1) % STATUS_MESSAGES.len();
            }
            self.last_update = Instant::now();
        }
    }

    /// 重置 spinner 状态
    pub fn reset_spinner(&mut self) {
        self.spinner_index = 0;
        self.status_index = 0;
        self.last_update = Instant::now();
    }

    /// 清除当前行
    pub fn clear_line() {
        print!("\r\x1b[2K\r");
        stdout().flush().ok();
    }

    /// 清除上一行（用于退出时清除 prompt）
    pub fn clear_previous_line() {
        print!("\r\x1b[1A\x1b[2K\r");
        stdout().flush().ok();
    }

    /// 显示响应开始
    pub fn response_start() {
    }

    /// 显示响应结束
    pub fn response_end() {
    }

    /// 显示思考块开始 - Claude Code 风格
    pub fn thinking_block_start() {
        println!(
            "{}",
            format!(
                "{} {}",
                THINKING_CORNER_TL.dimmed(),
                "Thinking".dimmed().italic()
            )
        );
    }

    /// 显示思考块内容
    pub fn thinking_block_content(content: &str) {
        // 确保内容正确缩进
        for line in content.lines() {
            println!("{} {}", THINKING_BORDER.dimmed(), line.dimmed().italic());
        }
    }

    /// 显示思考块结束
    pub fn thinking_block_end() {
        println!("{}", format!("{}", THINKING_CORNER_BL.dimmed()));
    }

    /// 显示工具调用开始 - Claude Code 风格
    pub fn tool_start(tool_name: &str, args: Option<&str>) {
        // 格式化参数，使其更紧凑
        let formatted_args = if let Some(args_str) = args {
            // 移除换行和多余空格，使 JSON 更紧凑
            let compact = args_str
                .lines()
                .map(|line| line.trim())
                .collect::<Vec<_>>()
                .join(" ");
            Some(compact)
        } else {
            None
        };

        match formatted_args {
            Some(args) if !args.is_empty() && args != "null" => {
                print!(
                    "{} {} {}",
                    "🔨".bright_magenta(),
                    tool_name.bright_magenta(),
                    args.dimmed()
                );
            }
            _ => {
                print!(
                    "{} {}",
                    "🔨".bright_magenta(),
                    tool_name.bright_magenta()
                );
            }
        }
        stdout().flush().ok();
    }

    /// 显示工具调用内容
    pub fn tool_content(content: &str) {
        // 在同一行显示结果
        let trimmed = content
            .lines()
            .map(|l| l.trim())
            .collect::<Vec<_>>()
            .join(" ");
        if !trimmed.is_empty() {
            println!(" {} {}", "=".bright_black(), trimmed.bright_green());
        } else {
            println!();
        }
    }

    /// 显示工具调用结束
    pub fn tool_end() {
        // 不需要额外显示，结果已在 tool_content 中显示
    }

    /// 显示工具调用错误
    pub fn tool_error(error: &str) {
        println!("{} {}", "✖".bright_red(), error.bright_red());
    }

    /// 打印错误信息 - Claude Code 风格
    pub fn error(msg: &str) {
        println!("\n{} {}", "✖".bright_red(), msg.bright_red());
    }

    /// 打印信息提示
    pub fn info(msg: &str) {
        println!("{} {}", "ℹ".bright_blue(), msg.bright_blue());
    }

    /// 打印成功信息
    pub fn success(msg: &str) {
        println!("{} {}", "✓".bright_green(), msg.bright_green());
    }

    /// 打印警告信息
    pub fn warning(msg: &str) {
        println!("{} {}", "⚠".bright_yellow(), msg.bright_yellow());
    }

    /// 清除屏幕
    pub fn clear() {
        print!("\x1b[2J\x1b[H");
        stdout().flush().ok();
    }

    /// 打印分隔线
    pub fn separator() {
        println!("{}", "─".repeat(60).dimmed());
    }

    /// 显示退出信息
    pub fn goodbye() {
        println!("{} {}", "✦".bright_yellow(), "Goodbye!".bright_yellow());
    }
}

impl Default for UI {
    fn default() -> Self {
        Self::new()
    }
}
