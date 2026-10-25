pub const BUILTINS: &[&str] = &["q", "checkhealth"];

pub fn known_commands_hint() -> String {
    let names: Vec<String> = BUILTINS.iter().map(|c| format!(":{c}")).collect();
    names.join(" or ")
}
