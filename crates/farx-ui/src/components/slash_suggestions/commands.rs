use super::command::SlashCommand;
use super::commands_a::PART_A;
use super::commands_b::PART_B;

const TOTAL: usize = PART_A.len() + PART_B.len();

const fn build_all() -> [SlashCommand; TOTAL] {
    let placeholder = SlashCommand {
        command: "",
        description: "",
    };
    let mut out = [placeholder; TOTAL];
    let mut i = 0;
    while i < PART_A.len() {
        out[i] = PART_A[i];
        i += 1;
    }
    let mut j = 0;
    while j < PART_B.len() {
        out[PART_A.len() + j] = PART_B[j];
        j += 1;
    }
    out
}

const ALL: [SlashCommand; TOTAL] = build_all();

/// All available slash commands with their descriptions.
pub const SLASH_COMMANDS: &[SlashCommand] = &ALL;
