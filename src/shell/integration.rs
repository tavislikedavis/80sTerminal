use log::debug;

/// Shell integration for command tracking and marks
/// Similar to iTerm2's shell integration feature
pub struct ShellIntegration {
    current_command: Option<String>,
    current_directory: String,
    command_history: Vec<CommandRecord>,
    marks: Vec<Mark>,
}

#[derive(Clone)]
pub struct CommandRecord {
    pub command: String,
    pub start_row: usize,
    pub end_row: Option<usize>,
    pub exit_code: Option<i32>,
    pub working_directory: String,
}

#[derive(Clone)]
pub struct Mark {
    pub row: usize,
    pub mark_type: MarkType,
}

#[derive(Clone, Copy, PartialEq)]
pub enum MarkType {
    PromptStart,
    CommandStart,
    CommandEnd,
    OutputEnd,
}

impl ShellIntegration {
    pub fn new() -> Self {
        Self {
            current_command: None,
            current_directory: String::from("~"),
            command_history: Vec::new(),
            marks: Vec::new(),
        }
    }

    /// Process OSC escape sequences for shell integration
    /// iTerm2 uses OSC 133 for shell integration
    pub fn process_osc(&mut self, params: &[&[u8]]) {
        if params.is_empty() {
            return;
        }

        // Check for FinalTerm/iTerm2 shell integration sequences
        // OSC 133 ; A ST - Prompt started
        // OSC 133 ; B ST - Command started
        // OSC 133 ; C ST - Command executed
        // OSC 133 ; D ; exit_code ST - Command finished

        if let Some(first) = params.get(0) {
            if *first == b"133" {
                if let Some(code) = params.get(1) {
                    match code.as_ref() {
                        b"A" => {
                            debug!("Shell integration: Prompt started");
                            // Mark prompt start
                        }
                        b"B" => {
                            debug!("Shell integration: Command started");
                            // Mark command input area
                        }
                        b"C" => {
                            debug!("Shell integration: Command executed");
                            // Command is being executed
                        }
                        b"D" => {
                            debug!("Shell integration: Command finished");
                            // Command finished, exit code in params[2]
                            if let Some(exit_code) = params.get(2) {
                                if let Ok(code_str) = std::str::from_utf8(exit_code) {
                                    if let Ok(code) = code_str.parse::<i32>() {
                                        self.finish_command(code);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // OSC 7 - Current working directory
        if params.get(0) == Some(&b"7".as_ref()) {
            if let Some(url) = params.get(1) {
                if let Ok(url_str) = std::str::from_utf8(url) {
                    // Parse file:// URL to get path
                    if url_str.starts_with("file://") {
                        let path_start = url_str.find('/').unwrap_or(7);
                        self.current_directory = url_str[path_start..].to_string();
                        debug!("Working directory: {}", self.current_directory);
                    }
                }
            }
        }
    }

    pub fn start_command(&mut self, command: String, row: usize) {
        self.current_command = Some(command.clone());
        self.command_history.push(CommandRecord {
            command,
            start_row: row,
            end_row: None,
            exit_code: None,
            working_directory: self.current_directory.clone(),
        });
    }

    pub fn finish_command(&mut self, exit_code: i32) {
        if let Some(record) = self.command_history.last_mut() {
            record.exit_code = Some(exit_code);
        }
        self.current_command = None;
    }

    pub fn add_mark(&mut self, row: usize, mark_type: MarkType) {
        self.marks.push(Mark { row, mark_type });
    }

    pub fn current_directory(&self) -> &str {
        &self.current_directory
    }

    pub fn command_history(&self) -> &[CommandRecord] {
        &self.command_history
    }

    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    /// Navigate to previous command mark
    pub fn prev_command_mark(&self, current_row: usize) -> Option<usize> {
        self.marks
            .iter()
            .filter(|m| m.mark_type == MarkType::CommandStart && m.row < current_row)
            .last()
            .map(|m| m.row)
    }

    /// Navigate to next command mark
    pub fn next_command_mark(&self, current_row: usize) -> Option<usize> {
        self.marks
            .iter()
            .find(|m| m.mark_type == MarkType::CommandStart && m.row > current_row)
            .map(|m| m.row)
    }
}

impl Default for ShellIntegration {
    fn default() -> Self {
        Self::new()
    }
}
