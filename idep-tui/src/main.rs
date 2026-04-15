use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use idep_core::buffer::{Buffer, Cursor};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io::{self, stdout};
use std::path::PathBuf;

/// Guard to ensure terminal is restored even on panic or early exit.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Normal => write!(f, "NORMAL"),
            Mode::Insert => write!(f, "INSERT"),
        }
    }
}

struct App {
    buffer: Buffer,
    mode: Mode,
    filename: Option<PathBuf>,
    scroll_offset: usize,
    modified: bool,
}

impl App {
    fn new() -> Self {
        Self {
            buffer: Buffer::new(),
            mode: Mode::Normal,
            filename: None,
            scroll_offset: 0,
            modified: false,
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
        // Allow cursor at end of line for insertion
        let max_col = if new_line == line_count.saturating_sub(1) {
            line_len
        } else {
            line_len
        };

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

    fn save(&mut self) -> Result<()> {
        if let Some(ref path) = self.filename {
            std::fs::write(path, self.buffer.to_string())?;
            self.modified = false;
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
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
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
    let mut should_quit = false;
    let mut last_viewport_height: usize = 0;

    while !should_quit {
        // Compute layout to get viewport height for scroll update
        let size = terminal.size()?;
        let viewport_height = size.height.saturating_sub(1) as usize;
        if viewport_height != last_viewport_height {
            app.update_scroll(viewport_height);
            last_viewport_height = viewport_height;
        }

        terminal.draw(|f| render(&app, f))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    should_quit = handle_key_event(&mut app, key)?;
                    // Update scroll after any cursor-moving operation
                    app.update_scroll(viewport_height);
                }
            }
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: event::KeyEvent) -> Result<bool> {
    // Handle Ctrl+S (save) in both modes
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('\u{0013}') => {
                app.save()?;
                return Ok(false);
            }
            _ => {}
        }
    }

    match app.mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('i') => app.mode = Mode::Insert,
            KeyCode::Char('h') | KeyCode::Left => app.move_cursor(-1, 0),
            KeyCode::Char('j') | KeyCode::Down => app.move_cursor(0, 1),
            KeyCode::Char('k') | KeyCode::Up => app.move_cursor(0, -1),
            KeyCode::Char('l') | KeyCode::Right => app.move_cursor(1, 0),
            KeyCode::Esc => {}
            _ => {}
        },
        Mode::Insert => match key.code {
            KeyCode::Esc => app.mode = Mode::Normal,
            KeyCode::Char(c) => app.insert_char(c),
            KeyCode::Backspace => app.delete_char(),
            KeyCode::Enter => app.insert_char('\n'),
            _ => {}
        },
    }
    Ok(false)
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

    let line_numbers_widget = Paragraph::new(line_numbers)
        .block(Block::default().borders(Borders::NONE).style(Style::default().bg(Color::Black)));
    frame.render_widget(line_numbers_widget, line_numbers_area);

    let editor_widget = Paragraph::new(text_content)
        .block(Block::default().borders(Borders::NONE).style(Style::default().bg(Color::Black)));
    frame.render_widget(editor_widget, editor_area);

    let filename_str = app
        .filename
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "[No Name]".to_string());

    let modified_str = if app.modified { " [+]" } else { "" };
    let cursor = app.cursor();

    let status_spans = vec![
        Span::styled(
            format!("{}:{}", cursor.line + 1, cursor.column + 1),
            Style::default().fg(Color::White),
        ),
        Span::raw(" | "),
        Span::styled(
            app.mode.to_string(),
            if app.mode == Mode::Insert {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            },
        ),
        Span::raw(" | "),
        Span::styled(
            format!("{}{}", filename_str, modified_str),
            Style::default().fg(Color::White),
        ),
    ];

    let status_bar = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, status_area);
}
