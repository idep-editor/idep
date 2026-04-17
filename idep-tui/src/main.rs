use anyhow::Result;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
        MouseEvent, MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use idep_core::buffer::{Buffer, Cursor};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use signal_hook::consts::SIGINT;
use signal_hook::flag::register;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Guard to ensure terminal is restored even on panic or early exit.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(DisableMouseCapture);
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

/// Setup signal handlers for graceful shutdown.
fn setup_signal_handler(running: Arc<AtomicBool>) -> Result<()> {
    register(SIGINT, running)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
            Mode::Command => write!(f, "COMMAND"),
        }
    }
}

struct App {
    buffer: Buffer,
    mode: Mode,
    filename: Option<PathBuf>,
    scroll_offset: usize,
    modified: bool,
    pending_g: bool,
    pending_d: bool,
    command_buffer: String,
    should_quit: bool,
    status_message: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            mode: Mode::Normal,
            filename: None,
            scroll_offset: 0,
            modified: false,
            pending_g: false,
            pending_d: false,
            command_buffer: String::new(),
            should_quit: false,
            status_message: None,
        }
    }

    fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            buffer: Buffer::with_text(&content),
            mode: Mode::Normal,
            filename: Some(path),
            scroll_offset: 0,
            modified: false,
            pending_g: false,
            pending_d: false,
            command_buffer: String::new(),
            should_quit: false,
            status_message: None,
        })
    }

    fn cursor(&self) -> Cursor {
        self.buffer.cursor()
    }

    fn move_cursor(&mut self, dx: isize, dy: isize) {
        let cursor = self.buffer.cursor();
        let lines = self.buffer.lines();
        let line_count = lines.len().max(1);

        let new_line = if dy < 0 {
            cursor.line.saturating_sub((-dy) as usize)
        } else {
            (cursor.line + dy as usize).min(line_count.saturating_sub(1))
        };

        let line_len = lines.get(new_line).map(|l| l.chars().count()).unwrap_or(0);
        let max_col = line_len;

        let new_col = if dx < 0 {
            cursor.column.saturating_sub((-dx) as usize)
        } else {
            (cursor.column + dx as usize).min(max_col)
        };

        self.buffer.set_cursor(new_line, new_col);
    }

    fn line_number_width(&self) -> u16 {
        let line_count = self.buffer.lines().len().max(1);
        (line_count.to_string().len() + 1).max(4) as u16
    }

    fn insert_char(&mut self, c: char) {
        let pos = self.buffer.cursor_char_index();
        self.buffer.insert(pos, &c.to_string());
        self.modified = true;
    }

    fn delete_char(&mut self) {
        let pos = self.buffer.cursor_char_index();
        if pos > 0 {
            self.buffer.delete(pos - 1..pos);
            self.modified = true;
        }
    }

    fn undo(&mut self) {
        if self.buffer.undo() {
            self.modified = true;
            self.status_message = Some("undone".to_string());
        } else {
            self.status_message = Some("nothing to undo".to_string());
        }
    }

    fn redo(&mut self) {
        if self.buffer.redo() {
            self.modified = true;
            self.status_message = Some("redone".to_string());
        } else {
            self.status_message = Some("nothing to redo".to_string());
        }
    }

    fn move_word_forward(&mut self) {
        let cursor = self.buffer.cursor();
        let lines = self.buffer.lines();
        let mut line_idx = cursor.line;
        let mut col = cursor.column;

        while let Some(line) = lines.get(line_idx) {
            let chars: Vec<char> = line.chars().collect();

            // Skip current word (if any)
            while col < chars.len() && !chars[col].is_whitespace() {
                col += 1;
            }

            // Skip whitespace
            while col < chars.len() && chars[col].is_whitespace() {
                col += 1;
            }

            // If we found a non-whitespace char, stop here
            if col < chars.len() {
                self.buffer.set_cursor(line_idx, col);
                return;
            }

            // Move to next line
            line_idx += 1;
            col = 0;
        }

        // End of file - stay at last position
        let last_line = lines.len().saturating_sub(1);
        let last_col = lines.get(last_line).map(|l| l.chars().count()).unwrap_or(0);
        self.buffer.set_cursor(last_line, last_col);
    }

    fn move_word_backward(&mut self) {
        let cursor = self.buffer.cursor();
        let lines = self.buffer.lines();
        let mut line_idx = cursor.line;
        let mut col = cursor.column;

        // Handle starting at beginning of a line
        if col == 0 {
            if line_idx == 0 {
                return; // Already at start of file
            }
            line_idx -= 1;
            col = lines.get(line_idx).map(|l| l.chars().count()).unwrap_or(0);
        } else {
            col = col.saturating_sub(1);
        }

        while let Some(line) = lines.get(line_idx) {
            let chars: Vec<char> = line.chars().collect();

            // Skip whitespace going backward
            while col > 0 && chars.get(col).map(|c| c.is_whitespace()).unwrap_or(false) {
                col -= 1;
            }

            // If we're at a word char, skip the word going backward
            if col > 0 && chars.get(col).map(|c| !c.is_whitespace()).unwrap_or(false) {
                while col > 0
                    && chars
                        .get(col - 1)
                        .map(|c| !c.is_whitespace())
                        .unwrap_or(false)
                {
                    col -= 1;
                }
                self.buffer.set_cursor(line_idx, col);
                return;
            }

            // Move to previous line
            if line_idx == 0 {
                self.buffer.set_cursor(0, 0);
                return;
            }
            line_idx -= 1;
            col = lines.get(line_idx).map(|l| l.chars().count()).unwrap_or(0);
        }
    }

    fn move_to_line_start(&mut self) {
        let cursor = self.buffer.cursor();
        self.buffer.set_cursor(cursor.line, 0);
    }

    fn move_to_line_end(&mut self) {
        let cursor = self.buffer.cursor();
        let lines = self.buffer.lines();
        let line_len = lines
            .get(cursor.line)
            .map(|l| l.chars().count())
            .unwrap_or(0);
        self.buffer.set_cursor(cursor.line, line_len);
    }

    fn move_to_file_start(&mut self) {
        self.buffer.set_cursor(0, 0);
    }

    fn move_to_file_end(&mut self) {
        let lines = self.buffer.lines();
        let last_line = lines.len().saturating_sub(1);
        let last_col = lines.last().map(|l| l.chars().count()).unwrap_or(0);
        self.buffer.set_cursor(last_line, last_col);
    }

    fn delete_line(&mut self) {
        let cursor = self.buffer.cursor();
        let rope = self.buffer.rope();
        let line_idx = cursor.line.min(rope.len_lines().saturating_sub(1));
        let line = rope.line(line_idx);
        let line_char_count = line.len_chars();

        let start_pos = rope.line_to_char(line_idx);
        let end_pos = if line_idx == rope.len_lines().saturating_sub(1) {
            // Last line: delete to end of buffer
            start_pos + line_char_count
        } else {
            // Not last line: include the newline
            start_pos + line_char_count
        };

        self.buffer.delete(start_pos..end_pos);
        self.modified = true;

        // Move cursor to start of line (or previous line if this was the last)
        let new_line = line_idx.min(self.buffer.lines().len().saturating_sub(1));
        self.buffer.set_cursor(new_line, 0);
    }

    fn save(&mut self) -> Result<()> {
        if let Some(ref path) = self.filename {
            std::fs::write(path, self.buffer.to_string())?;
            self.modified = false;
            self.status_message = Some(format!("saved {}", path.display()));
        } else {
            self.status_message = Some("error: no filename".to_string());
        }
        Ok(())
    }

    fn update_scroll(&mut self, viewport_height: usize) {
        let cursor = self.buffer.cursor();
        if cursor.line < self.scroll_offset {
            self.scroll_offset = cursor.line;
        } else if cursor.line >= self.scroll_offset + viewport_height {
            self.scroll_offset = cursor.line.saturating_sub(viewport_height - 1);
        }
    }

    fn execute_command(&mut self) -> Result<()> {
        let cmd = self.command_buffer.trim();
        if cmd.is_empty() {
            // Empty command - just return to normal mode silently
            self.command_buffer.clear();
            self.mode = Mode::Normal;
            return Ok(());
        }
        match cmd {
            "w" => {
                self.save()?;
            }
            "q" => {
                if self.modified {
                    self.status_message =
                        Some("error: unsaved changes (use :q! to force)".to_string());
                } else {
                    self.should_quit = true;
                }
            }
            "wq" => {
                self.save()?;
                if self.filename.is_some() {
                    self.should_quit = true;
                }
            }
            "q!" => {
                self.should_quit = true;
            }
            _ => {
                self.status_message = Some(format!("unknown command: {}", cmd));
            }
        }
        self.command_buffer.clear();
        self.mode = Mode::Normal;
        Ok(())
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut app = if args.len() > 1 {
        App::from_file(PathBuf::from(&args[1]))?
    } else {
        App::new()
    };

    let mut terminal = setup_terminal()?;
    let _guard = TerminalGuard;
    let mut last_viewport_height: usize = 0;

    // Setup signal handler for graceful Ctrl+C handling
    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone())?;

    while !app.should_quit && running.load(Ordering::SeqCst) {
        // Compute layout to get viewport height for scroll update
        let size = terminal.size()?;
        let viewport_height = size.height.saturating_sub(1) as usize;
        if viewport_height != last_viewport_height {
            app.update_scroll(viewport_height);
            last_viewport_height = viewport_height;
        }

        terminal.draw(|f| render(&app, f))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(&mut app, key)?;
                        // Update scroll after any cursor-moving operation
                        app.update_scroll(viewport_height);
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse_event(&mut app, mouse, viewport_height);
                    // Update scroll after mouse scroll to ensure cursor stays visible
                    app.update_scroll(viewport_height);
                }
                Event::Resize(_, _) => {
                    // Terminal resized - scroll update will happen on next loop iteration
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Handle mouse events for scrolling and cursor positioning.
fn handle_mouse_event(app: &mut App, mouse: MouseEvent, viewport_height: usize) {
    let line_num_width = app.line_number_width();

    match mouse.kind {
        MouseEventKind::ScrollDown => {
            // Scroll down by 3 lines
            let lines = app.buffer.lines();
            let max_scroll = lines.len().saturating_sub(viewport_height);
            app.scroll_offset = (app.scroll_offset + 3).min(max_scroll);
        }
        MouseEventKind::ScrollUp => {
            // Scroll up by 3 lines
            app.scroll_offset = app.scroll_offset.saturating_sub(3);
        }
        MouseEventKind::Down(_) => {
            // Click to position cursor
            // Mouse column includes line number gutter, so adjust
            if mouse.column >= line_num_width {
                let editor_col = mouse.column - line_num_width;
                let clicked_line = app.scroll_offset + mouse.row as usize;
                let lines = app.buffer.lines();

                if clicked_line < lines.len() {
                    let line_len = lines
                        .get(clicked_line)
                        .map(|l| l.chars().count())
                        .unwrap_or(0);
                    let new_col = (editor_col as usize).min(line_len);
                    app.buffer.set_cursor(clicked_line, new_col);
                }
            }
        }
        _ => {}
    }
}

fn handle_key_event(app: &mut App, key: event::KeyEvent) -> Result<()> {
    // Handle Ctrl+S (save) in both Normal and Insert modes
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('\u{0013}') => {
                app.save()?;
                return Ok(());
            }
            _ => {}
        }
    }

    match app.mode {
        Mode::Command => match key.code {
            KeyCode::Enter => {
                app.execute_command()?;
            }
            KeyCode::Esc => {
                app.command_buffer.clear();
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                app.command_buffer.pop();
            }
            KeyCode::Char(c) => {
                // Only accept printable ASCII characters (32-126)
                if c.is_ascii_graphic() || c == ' ' {
                    app.command_buffer.push(c);
                }
            }
            _ => {}
        },
        Mode::Normal => {
            // Clear any status message on keypress
            app.status_message = None;
            match key.code {
                KeyCode::Char(':') => {
                    app.pending_g = false;
                    app.pending_d = false;
                    app.mode = Mode::Command;
                    app.command_buffer.clear();
                }
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('i') => {
                    app.pending_g = false;
                    app.pending_d = false;
                    app.mode = Mode::Insert;
                }
                KeyCode::Char('h') | KeyCode::Left => app.move_cursor(-1, 0),
                KeyCode::Char('j') | KeyCode::Down => app.move_cursor(0, 1),
                KeyCode::Char('k') | KeyCode::Up => app.move_cursor(0, -1),
                KeyCode::Char('l') | KeyCode::Right => app.move_cursor(1, 0),
                KeyCode::Char('w') => app.move_word_forward(),
                KeyCode::Char('b') => app.move_word_backward(),
                KeyCode::Char('0') => app.move_to_line_start(),
                KeyCode::Char('$') => app.move_to_line_end(),
                KeyCode::Char('g') if app.pending_g => {
                    app.move_to_file_start();
                    app.pending_g = false;
                }
                KeyCode::Char('g') => app.pending_g = true,
                KeyCode::Char('G') => app.move_to_file_end(),
                KeyCode::Char('d') if app.pending_d => {
                    app.delete_line();
                    app.pending_d = false;
                }
                KeyCode::Char('d') => app.pending_d = true,
                KeyCode::Char('u') => app.undo(),
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => app.redo(),
                KeyCode::Esc => {
                    app.pending_g = false;
                    app.pending_d = false;
                }
                _ => {
                    app.pending_g = false;
                    app.pending_d = false;
                }
            }
        }
        Mode::Insert => {
            // Clear any status message on keypress
            app.status_message = None;
            match key.code {
                KeyCode::Esc => app.mode = Mode::Normal,
                KeyCode::Char(c) => app.insert_char(c),
                KeyCode::Backspace => app.delete_char(),
                KeyCode::Enter => app.insert_char('\n'),
                _ => {}
            }
        }
    }
    Ok(())
}

fn render(app: &App, frame: &mut Frame) {
    let size = frame.size();
    let line_num_width = app.line_number_width();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    let content_area = chunks[0];
    let status_area = chunks[1];

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(line_num_width), Constraint::Min(1)])
        .split(content_area.inner(&Margin {
            horizontal: 0,
            vertical: 0,
        }));

    let line_numbers_area = content_chunks[0];
    let editor_area = content_chunks[1];

    let viewport_height = editor_area.height as usize;
    let lines = app.buffer.lines();
    let visible_lines: Vec<String> = lines
        .iter()
        .skip(app.scroll_offset)
        .take(viewport_height)
        .cloned()
        .collect();

    let line_number_width = app.line_number_width() as usize;

    let line_numbers: Vec<Line> = (app.scroll_offset + 1..=app.scroll_offset + visible_lines.len())
        .map(|i| {
            let style = if i - 1 == app.cursor().line {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Line::from(Span::styled(
                format!("{:>width$} ", i, width = line_number_width - 1),
                style,
            ))
        })
        .collect();

    let text_content: Vec<Line> = visible_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_idx = app.scroll_offset + i;
            let cursor = app.cursor();
            if line_idx == cursor.line {
                let mut spans = vec![];
                let before: String = line.chars().take(cursor.column).collect();
                let at_cursor: String = line
                    .chars()
                    .skip(cursor.column)
                    .take(1)
                    .collect::<String>()
                    .replace('\0', " ");
                let after: String = line.chars().skip(cursor.column + 1).collect();

                if !before.is_empty() {
                    spans.push(Span::raw(before));
                }

                let cursor_char = if at_cursor.is_empty() {
                    " ".to_string()
                } else {
                    at_cursor
                };
                spans.push(Span::styled(
                    cursor_char,
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ));

                if !after.is_empty() {
                    spans.push(Span::raw(after));
                }

                Line::from(spans)
            } else {
                Line::raw(line.clone())
            }
        })
        .collect();

    let line_numbers_widget = Paragraph::new(line_numbers).block(
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(line_numbers_widget, line_numbers_area);

    let editor_widget = Paragraph::new(text_content).block(
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(editor_widget, editor_area);

    let filename_str = app
        .filename
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No Name]".to_string());

    let modified_str = if app.modified { " [+]" } else { "" };
    let cursor = app.cursor();

    let status_spans = if app.mode == Mode::Command {
        vec![Span::styled(
            format!(":{}", app.command_buffer),
            Style::default().fg(Color::White),
        )]
    } else if let Some(ref msg) = app.status_message {
        let is_error = msg.starts_with("error:");
        let style = if is_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        vec![Span::styled(msg.clone(), style)]
    } else {
        vec![
            Span::styled(
                format!("{}:{}", cursor.line + 1, cursor.column + 1),
                Style::default().fg(Color::White),
            ),
            Span::raw(" | "),
            Span::styled(
                app.mode.to_string(),
                if app.mode == Mode::Insert {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD)
                },
            ),
            Span::raw(" | "),
            Span::styled(
                format!("{}{}", filename_str, modified_str),
                Style::default().fg(Color::White),
            ),
        ]
    };

    let status_bar = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, status_area);
}
