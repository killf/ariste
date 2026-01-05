mod agent;
mod command;
mod config;
mod error;
mod image;
mod ollama;

use crate::agent::Agent;
use crate::command::AgentHinter;
use crate::error::Error;
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

    // 2. 创建Agent
    let mut agent = Agent::load_from_config(workdir.clone()).await?;
    println!("工作目录: {}", agent.workdir.display());

    let mut rl: Editor<AgentHinter, DefaultHistory> = Editor::new()?;
    if rl.load_history(".ariste/history.txt").is_err() {
        println!("No previous history.");
    }
    rl.set_helper(Some(AgentHinter::new()));

    // 3. 聊天对话
    loop {
        match rl.readline("> ") {
            Ok(line) => {
                let line = line.trim();
                rl.add_history_entry(line)?;

                if line == "/q" || line == "/quit" || line == "/exit" {
                    agent.quit().await?;
                    break;
                } else if line.starts_with("/") {
                    continue;
                }

                if let Err(e) = agent.invoke(line).await {
                    println!("{}", e);
                }
            }
            Err(ReadlineError::Interrupted) => {
                agent.quit().await?;
                break;
            }
            Err(ReadlineError::Eof) => {
                agent.quit().await?;
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                continue;
            }
        }
    }

    // 4. 保存历史信息
    rl.save_history(".ariste/history.txt")?;

    Ok(())
}
