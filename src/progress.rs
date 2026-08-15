use std::{
    io::{Write, stdout},
    sync::Mutex,
};

const FILLED: char = '=';
const HEAD: char = '>';
const EMPTY: char = '-';

const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
const RESET: &str = "\x1b[0m";

const DEFAULT_WIDTH: usize = 80;

#[derive(Debug)]
struct State {
    pos: u64,
    message: String,
}

#[derive(Debug)]
pub struct ProgressBar {
    total: u64,
    state: Mutex<State>,
}

impl ProgressBar {
    pub fn new(total: u64) -> Self {
        let pb = Self {
            total,
            state: Mutex::new(State {
                pos: 0,
                message: String::new(),
            }),
        };
        pb.draw(0, "");
        pb
    }

    pub fn inc(&self, message: Option<String>) {
        let mut state = self.state.lock().unwrap();
        state.pos = (state.pos + 1).min(self.total);
        if let Some(message) = message {
            state.message = message;
        }
        self.draw(state.pos, &state.message);
    }

    pub fn finish_and_clear(&self) {
        let mut out = stdout();

        let _ = write!(out, "\r\x1b[K");
        let _ = out.flush();
    }

    fn draw(&self, pos: u64, message: &str) {
        let width = terminal_width();
        let total = self.total;

        let suffix = format!(" {pos}/{total} | saved {message} ");

        let bar_width = width.saturating_sub(suffix.len() + 2).max(1);

        let filled = if total == 0 {
            bar_width
        } else {
            ((bar_width as u64 * pos) / total).min(bar_width as u64) as usize
        };

        let has_head = pos < total && filled < bar_width;
        let head_len = usize::from(has_head);
        let empty_len = bar_width - filled - head_len;

        let mut bar = String::with_capacity(bar_width + 2 * (CYAN.len() + RESET.len()));
        bar.push_str(CYAN);
        for _ in 0..filled {
            bar.push(FILLED);
        }
        if has_head {
            bar.push(HEAD);
        }
        bar.push_str(RESET);
        bar.push_str(BLUE);
        for _ in 0..empty_len {
            bar.push(EMPTY);
        }
        bar.push_str(RESET);

        let line = format!("[{bar}]{suffix}");
        let mut out = stdout();

        let _ = write!(out, "\r{line}\x1b[K");
        let _ = out.flush();
    }
}

fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(width, _)| width.0 as usize)
        .unwrap_or(DEFAULT_WIDTH)
}
