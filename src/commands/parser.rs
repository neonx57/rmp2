#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExCommand {
    Empty,
    Quit,
    CheckHealth,
    Unknown(String),
}

pub fn parse_ex_command(line: &str) -> ExCommand {
    match line.trim() {
        "" => ExCommand::Empty,
        "q" | "quit" => ExCommand::Quit,
        "checkhealth" => ExCommand::CheckHealth,
        other => ExCommand::Unknown(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quit_and_checkhealth_commands() {
        assert_eq!(parse_ex_command("q"), ExCommand::Quit);
        assert_eq!(parse_ex_command(" quit "), ExCommand::Quit);
        assert_eq!(parse_ex_command("checkhealth"), ExCommand::CheckHealth);
        assert_eq!(parse_ex_command(""), ExCommand::Empty);
        assert_eq!(parse_ex_command("   "), ExCommand::Empty);
        assert_eq!(
            parse_ex_command("bogus"),
            ExCommand::Unknown("bogus".to_string())
        );
    }
}
