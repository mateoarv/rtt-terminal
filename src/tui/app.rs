use std::collections::VecDeque;

pub const DEFAULT_MAX_LOGS: usize = 10_000;

pub struct AppState {
    pub probe_name: String,
    pub target_name: String,
    pub core_status: String,
    pub active_channel: usize,
    pub eol: String,
    pub timestamps_enabled: bool,
    pub auto_reconnect_enabled: bool,
    pub log_file: Option<String>,

    // Log buffer
    pub logs: VecDeque<String>,
    pub max_logs: usize,
    pub scroll_offset: usize, // Lines scrolled up from the bottom (0 = at the latest line)
    pub auto_scroll: bool,
    incoming_buffer: String,

    // Input prompt state
    pub input: String,
    pub cursor_position: usize, // Char index in input string

    // Command history
    pub history: Vec<String>,
    pub history_index: Option<usize>,

    pub should_quit: bool,
}

impl AppState {
    pub fn new(
        probe_name: String,
        target_name: String,
        active_channel: usize,
        eol: String,
    ) -> Self {
        Self {
            probe_name,
            target_name,
            core_status: "RUNNING".to_string(),
            active_channel,
            eol,
            timestamps_enabled: true,
            auto_reconnect_enabled: true,
            log_file: None,

            logs: VecDeque::with_capacity(1024),
            max_logs: DEFAULT_MAX_LOGS,
            scroll_offset: 0,
            auto_scroll: true,
            incoming_buffer: String::new(),

            input: String::new(),
            cursor_position: 0,

            history: Vec::new(),
            history_index: None,

            should_quit: false,
        }
    }

    /// Appends incoming raw bytes from RTT, splitting by newline.
    pub fn add_log_bytes(&mut self, bytes: &[u8]) {
        let text = String::from_utf8_lossy(bytes);
        self.incoming_buffer.push_str(&text);

        while let Some(pos) = self.incoming_buffer.find('\n') {
            let mut line = self.incoming_buffer[..pos].to_string();
            if line.ends_with('\r') {
                line.pop();
            }
            self.incoming_buffer = self.incoming_buffer[pos + 1..].to_string();
            self.add_log_line(line);
        }
    }

    /// Appends a single line to the log buffer.
    pub fn add_log_line(&mut self, line: String) {
        if self.logs.len() >= self.max_logs {
            self.logs.pop_front();
        }
        self.logs.push_back(line);

        if self.auto_scroll {
            self.scroll_offset = 0;
        }
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.auto_scroll = false;
        let max_scroll = self.logs.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        if self.scroll_offset <= amount {
            self.scroll_offset = 0;
            self.auto_scroll = true;
        } else {
            self.scroll_offset -= amount;
        }
    }

    #[allow(dead_code)]
    pub fn scroll_to_top(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.logs.len().saturating_sub(1);
    }

    #[allow(dead_code)]
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }

    // Input line manipulation
    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.char_to_byte_index(self.cursor_position);
        self.input.insert(byte_pos, c);
        self.cursor_position += 1;
    }

    pub fn delete_char_before(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            let byte_pos = self.char_to_byte_index(self.cursor_position);
            self.input.remove(byte_pos);
        }
    }

    pub fn delete_char_at(&mut self) {
        if self.cursor_position < self.input.chars().count() {
            let byte_pos = self.char_to_byte_index(self.cursor_position);
            self.input.remove(byte_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.input.chars().count() {
            self.cursor_position += 1;
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_position = self.input.chars().count();
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_idx = match self.history_index {
            None => self.history.len().saturating_sub(1),
            Some(idx) => idx.saturating_sub(1),
        };
        self.history_index = Some(next_idx);
        if let Some(cmd) = self.history.get(next_idx) {
            self.input = cmd.clone();
            self.cursor_position = self.input.chars().count();
        }
    }

    pub fn history_down(&mut self) {
        if let Some(idx) = self.history_index {
            if idx + 1 < self.history.len() {
                let next_idx = idx + 1;
                self.history_index = Some(next_idx);
                if let Some(cmd) = self.history.get(next_idx) {
                    self.input = cmd.clone();
                    self.cursor_position = self.input.chars().count();
                }
            } else {
                self.history_index = None;
                self.input.clear();
                self.cursor_position = 0;
            }
        }
    }

    pub fn submit_input(&mut self) -> String {
        let trimmed = self.input.trim().to_string();
        if !trimmed.is_empty() {
            // Avoid consecutive duplicates in history
            if self.history.last().map_or(true, |last| last != &trimmed) {
                self.history.push(trimmed.clone());
            }
        }
        let submitted = std::mem::take(&mut self.input);
        self.cursor_position = 0;
        self.history_index = None;
        submitted
    }

    fn char_to_byte_index(&self, char_idx: usize) -> usize {
        self.input
            .char_indices()
            .nth(char_idx)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_limit() {
        let mut state = AppState::new(
            "TestProbe".to_string(),
            "nRF52840_xxAA".to_string(),
            0,
            "CRLF".to_string(),
        );
        state.max_logs = 5;

        for i in 0..10 {
            state.add_log_line(format!("Line {}", i));
        }

        assert_eq!(state.logs.len(), 5);
        assert_eq!(state.logs.front().unwrap(), "Line 5");
        assert_eq!(state.logs.back().unwrap(), "Line 9");
    }

    #[test]
    fn test_input_manipulation() {
        let mut state = AppState::new(
            "TestProbe".to_string(),
            "nRF52840_xxAA".to_string(),
            0,
            "CRLF".to_string(),
        );

        state.insert_char('h');
        state.insert_char('e');
        state.insert_char('l');
        state.insert_char('p');
        assert_eq!(state.input, "help");
        assert_eq!(state.cursor_position, 4);

        state.move_cursor_left();
        state.insert_char('x');
        assert_eq!(state.input, "helxp");
        assert_eq!(state.cursor_position, 4);

        state.delete_char_before();
        assert_eq!(state.input, "help");

        let submitted = state.submit_input();
        assert_eq!(submitted, "help");
        assert!(state.input.is_empty());
        assert_eq!(state.history.len(), 1);
    }
}
