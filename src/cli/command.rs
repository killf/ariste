use colored::Colorize;
use rustyline::hint::{Hint, Hinter};
use rustyline::Context;
use rustyline_derive::{Completer, Helper, Highlighter, Validator};
use std::collections::HashSet;

#[derive(Hash, Debug, PartialEq, Eq)]
pub struct CommandHint {
    text: &'static str,
    display: String,
}

impl CommandHint {
    fn new(text: &'static str) -> Self {
        Self {
            text,
            display: text.cyan().italic().to_string(),
        }
    }

    fn suffix(&self, strip_chars: usize) -> Self {
        Self::new(&self.text[strip_chars..])
    }
}

impl Hint for CommandHint {
    fn display(&self) -> &str {
        self.display.as_str()
    }
    fn completion(&self) -> Option<&str> {
        Some(self.text)
    }
}

#[derive(Completer, Helper, Validator, Highlighter)]
pub struct AgentHinter {
    hints: HashSet<CommandHint>,
}

impl AgentHinter {
    pub fn new() -> Self {
        let mut hints = HashSet::new();
        hints.insert(CommandHint::new("/quit"));
        hints.insert(CommandHint::new("/clear"));
        hints.insert(CommandHint::new("/help"));
        AgentHinter { hints }
    }
}

impl Hinter for AgentHinter {
    type Hint = CommandHint;
    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        if line.is_empty() || pos < line.len() {
            return None;
        }

        self.hints
            .iter()
            .filter_map(|hint| {
                if hint.text.starts_with(line) {
                    Some(hint.suffix(pos))
                } else {
                    None
                }
            })
            .next()
    }
}
