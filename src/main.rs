mod agent;
mod command;
mod config;
mod error;
mod image;
mod ollama;
mod tools;
mod ui;

use crate::agent::Agent;
use crate::command::AgentHinter;
use crate::error::Error;
use crate::ui::UI;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    workdir: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();

    // 1. 指定工作目录
    let workdir: PathBuf = args.workdir.into();
    if !workdir.exists() {
        tokio::fs::create_dir_all(&workdir).await?;
    }

    let ariste_folder: PathBuf = ".ariste".into();
    if !ariste_folder.exists() {
        tokio::fs::create_dir_all(&ariste_folder).await?;
    }

    // 2. 创建Agent和UI
    let mut agent = Agent::load_from_config(workdir.clone()).await?;
    let mut ui = UI::new();

    // 3. 显示欢迎信息
    UI::welcome(&workdir);

    let mut rl: Editor<AgentHinter, DefaultHistory> = Editor::new()?;
    let history_file = ariste_folder.join("history.txt");
    if history_file.exists() {
        rl.load_history(&history_file)?;
    }
    rl.set_helper(Some(AgentHinter::new()));

    // 4. 聊天对话
    loop {
        let prompt = UI::prompt();
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                rl.add_history_entry(line)?;

                // 处理命令
                match line {
                    "/q" | "/quit" | "/exit" => {
                        UI::clear_previous_line();
                        agent.quit().await?;
                        UI::goodbye();
                        break;
                    }
                    "/help" => {
                        UI::welcome(&workdir);
                        continue;
                    }
                    "/clear" => {
                        agent.clear_history();
                        UI::clear();
                        UI::welcome(&workdir);
                        UI::info("Conversation history cleared");
                        continue;
                    }
                    cmd if cmd.starts_with('/') => {
                        UI::warning(&format!("Unknown command: {}", cmd));
                        UI::info("Type /help to see available commands");
                        continue;
                    }
                    _ => {
                        // 执行 AI 调用
                        ui.reset_spinner();
                        if let Err(e) = agent.invoke(line).await {
                            UI::error(&e.to_string());
                        }
                        UI::response_end();
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                UI::info("Use /quit to exit the program");
                continue;
            }
            Err(ReadlineError::Eof) => {
                UI::clear_previous_line();
                agent.quit().await?;
                UI::goodbye();
                break;
            }
            Err(err) => {
                UI::error(&format!("Error: {:?}", err));
                continue;
            }
        }
    }

    // 5. 保存历史信息
    rl.save_history(&history_file)?;

    Ok(())
}
