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
use idep_lsp::{client::LspClient, document::DocumentManager};
use lsp_types::{self, Diagnostic};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag::register;
use std::io::{self, stdout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

mod highlight;
use highlight::{highlight_to_style, Highlighter};

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
    register(SIGINT, Arc::clone(&running))?;
    register(SIGTERM, running)?;
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
    pending_space: bool,
    command_buffer: String,
    should_quit: bool,
    status_message: Option<String>,
    highlighter: Option<Highlighter>,
    /// LSP document manager for language server protocol integration
    document_manager: Option<DocumentManager>,
    /// Current diagnostics for the active file
    diagnostics: Vec<Diagnostic>,
    /// Whether the diagnostic detail panel is visible
    diagnostic_panel_visible: bool,
    /// Timer for debouncing didChange notifications (500ms)
    #[allow(dead_code)]
    lsp_debounce_timer: Option<Instant>,
    /// LSP server initialization state
    lsp_initialized: bool,
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
            pending_space: false,
            command_buffer: String::new(),
            should_quit: false,
            status_message: None,
            highlighter: None,
            document_manager: None,
            diagnostics: Vec::new(),
            diagnostic_panel_visible: false,
            lsp_debounce_timer: None,
            lsp_initialized: false,
        }
    }

    fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let highlighter = Highlighter::new(&path).ok();
        Ok(Self {
            buffer: Buffer::with_text(&content),
            mode: Mode::Normal,
            filename: Some(path),
            scroll_offset: 0,
            modified: false,
            pending_g: false,
            pending_d: false,
            pending_space: false,
            command_buffer: String::new(),
            should_quit: false,
            status_message: None,
            highlighter,
            document_manager: None,
            diagnostics: Vec::new(),
            diagnostic_panel_visible: false,
            lsp_debounce_timer: None,
            lsp_initialized: false,
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
        // Trigger debounced didChange notification (not yet implemented)
        // self.send_did_change();
    }

    fn delete_char(&mut self) {
        let pos = self.buffer.cursor_char_index();
        if pos > 0 {
            self.buffer.delete(pos - 1..pos);
            self.modified = true;
            // Trigger debounced didChange notification (not yet implemented)
            // self.send_did_change();
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
            // Send didSave notification if LSP is connected
            self.send_did_save();
        } else {
            self.status_message = Some("error: no filename".to_string());
        }
        Ok(())
    }

    /// Attempt to start LSP server based on file extension
    fn maybe_start_lsp(&mut self) {
        if self.lsp_initialized {
            return;
        }
        let Some(ref path) = self.filename else {
            return;
        };
        let extension = path.extension().and_then(|e| e.to_str());
        let language_id = match extension {
            Some("rs") => "rust",
            Some("ts") | Some("tsx") => "typescript",
            Some("js") | Some("jsx") => "javascript",
            Some("py") => "python",
            Some("toml") => "toml",
            Some("md") => "markdown",
            _ => return, // No LSP for unknown extensions
        };
        // Try to spawn appropriate LSP server
        let (server_cmd, server_args): (&str, &[&str]) = match extension {
            Some("rs") => ("rust-analyzer", &[]),
            Some("ts") | Some("tsx") | Some("js") | Some("jsx") => {
                ("typescript-language-server", &["--stdio"])
            }
            Some("py") => ("pylsp", &[]),
            _ => return,
        };
        // Create tokio runtime for async LSP operations
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                self.status_message = Some(format!("LSP runtime failed: {}", e));
                return;
            }
        };
        // Spawn LSP client in the runtime
        let client = match rt.block_on(async { LspClient::spawn(server_cmd, server_args) }) {
            Ok(client) => Arc::new(tokio::sync::Mutex::new(client)),
            Err(e) => {
                self.status_message = Some(format!("LSP spawn failed: {}", e));
                return;
            }
        };
        let mut doc_manager = DocumentManager::new(client);
        // Send didOpen for the current file
        let uri = lsp_types::Url::from_file_path(path).ok();
        let text = self.buffer.to_string();
        if let Some(uri) = uri {
            rt.block_on(async {
                doc_manager
                    .did_open(uri, language_id.to_string(), text)
                    .await
                    .ok();
            });
        }
        self.document_manager = Some(doc_manager);
        self.lsp_initialized = true;
    }

    /// Send didOpen notification to LSP server
    #[allow(dead_code)]
    fn send_did_open(&mut self) {
        if let Some(ref mut doc_manager) = self.document_manager {
            if let Some(ref path) = self.filename {
                let uri = lsp_types::Url::from_file_path(path).ok();
                let text = self.buffer.to_string();
                let language_id = match path.extension().and_then(|e| e.to_str()) {
                    Some("rs") => "rust",
                    Some("ts") | Some("tsx") => "typescript",
                    Some("js") | Some("jsx") => "javascript",
                    Some("py") => "python",
                    _ => "text",
                };
                if let Some(uri) = uri {
                    let rt = tokio::runtime::Runtime::new().ok();
                    if let Some(rt) = rt {
                        rt.block_on(async {
                            doc_manager
                                .did_open(uri, language_id.to_string(), text)
                                .await
                                .ok();
                        });
                    }
                }
            }
        }
    }

    /// Send didChange notification (called on buffer mutation)
    #[allow(dead_code)]
    fn send_did_change(&mut self) {
        self.lsp_debounce_timer = Some(Instant::now());
    }

    /// Send didSave notification to LSP server
    fn send_did_save(&mut self) {
        if let Some(ref mut doc_manager) = self.document_manager {
            if let Some(ref path) = self.filename {
                let uri = lsp_types::Url::from_file_path(path).ok();
                if let Some(uri) = uri {
                    let rt = tokio::runtime::Runtime::new().ok();
                    if let Some(rt) = rt {
                        rt.block_on(async {
                            doc_manager.did_save(uri).await.ok();
                        });
                    }
                }
            }
        }
    }

    fn poll_diagnostics(&mut self) {
        let new_diagnostics = if let Some(ref doc_manager) = self.document_manager {
            if let Some(ref path) = self.filename {
                let uri = lsp_types::Url::from_file_path(path).ok();
                if let Some(uri) = uri {
                    doc_manager.get_diagnostics(&uri).to_vec()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        self.diagnostics = new_diagnostics;
    }

    /// Count errors and warnings for status bar
    fn diagnostic_counts(&self) -> (usize, usize) {
        let mut errors = 0;
        let mut warnings = 0;
        for diag in &self.diagnostics {
            match diag.severity {
                Some(lsp_types::DiagnosticSeverity::ERROR) => errors += 1,
                Some(lsp_types::DiagnosticSeverity::WARNING) => warnings += 1,
                _ => {}
            }
        }
        (errors, warnings)
    }

    /// Toggle diagnostic panel visibility
    fn toggle_diagnostics(&mut self) {
        self.diagnostic_panel_visible = !self.diagnostic_panel_visible;
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
                    self.status_message = Some(
                        "error: unsaved changes (use :wq to save or :q! to force)".to_string(),
                    );
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

    // Start LSP server for the opened file (if supported language)
    app.maybe_start_lsp();

    let mut terminal = setup_terminal()?;
    let _guard = TerminalGuard;
    let mut last_viewport_height: usize = 0;

    // Setup signal handler for graceful Ctrl+C handling
    let running = Arc::new(AtomicBool::new(true));
    setup_signal_handler(running.clone())?;

    while !app.should_quit && running.load(Ordering::SeqCst) {
        // Check debounced LSP didChange notifications (not yet implemented)
        // app.check_debounce();
        // Poll for diagnostics from LSP
        app.poll_diagnostics();

        // Compute layout to get viewport height for scroll update
        let size = terminal.size()?;
        let viewport_height = size.height.saturating_sub(1) as usize;
        if viewport_height != last_viewport_height {
            app.update_scroll(viewport_height);
            last_viewport_height = viewport_height;
        }

        terminal.draw(|f| render(&mut app, f))?;

        if event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(&mut app, key)?;
                    // Update scroll after any cursor-moving operation
                    app.update_scroll(viewport_height);
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
        MouseEventKind::Down(_) if mouse.column >= line_num_width => {
            // Click to position cursor
            // Mouse column includes line number gutter, so adjust
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
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                // Only accept printable ASCII characters (32-126)
                app.command_buffer.push(c);
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
                KeyCode::Char('d') if app.pending_space => {
                    app.toggle_diagnostics();
                    app.pending_space = false;
                }
                KeyCode::Char('d') => app.pending_d = true,
                KeyCode::Char(' ') => app.pending_space = true,
                KeyCode::Char('u') => app.undo(),
                KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => app.redo(),
                KeyCode::Esc => {
                    app.pending_g = false;
                    app.pending_d = false;
                    app.pending_space = false;
                }
                _ => {
                    app.pending_g = false;
                    app.pending_d = false;
                    app.pending_space = false;
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

/// Get highlight spans for visible lines, organized by line index.
/// Returns spans with char offsets (not byte offsets) for direct use with string slicing.
fn get_highlight_spans(
    app: &mut App,
    visible_lines: &[String],
) -> std::collections::HashMap<usize, Vec<highlight::HighlightedSpan>> {
    use std::collections::HashMap;

    let mut result: HashMap<usize, Vec<highlight::HighlightedSpan>> = HashMap::new();

    if let Some(ref mut highlighter) = app.highlighter {
        if !highlighter.has_highlighting() {
            return result;
        }

        // Build full source from visible lines to get byte offsets correct
        let source = visible_lines.join("\n");
        let all_spans = highlighter.highlight(&source);

        // Distribute spans to their respective lines, converting byte offsets to char offsets
        let mut byte_offset = 0;
        for (line_idx, line) in visible_lines.iter().enumerate() {
            let line_start = byte_offset;
            let line_end = byte_offset + line.len();

            for span in &all_spans {
                // Check if span intersects with this line
                if span.end >= line_start && span.start < line_end {
                    // Clamp span to line bounds (in byte offsets)
                    let clamped_start_byte = span.start.max(line_start) - line_start;
                    let clamped_end_byte = span.end.min(line_end) - line_start;

                    if clamped_start_byte < clamped_end_byte {
                        // Convert byte offsets to char offsets for correct string slicing
                        let clamped_start_char = line[..clamped_start_byte].chars().count();
                        let clamped_end_char = line[..clamped_end_byte].chars().count();

                        result
                            .entry(line_idx)
                            .or_default()
                            .push(highlight::HighlightedSpan::new(
                                clamped_start_char,
                                clamped_end_char,
                                span.highlight.clone(),
                            ));
                    }
                }
            }

            byte_offset = line_end + 1; // +1 for newline
        }
    }

    result
}

/// Build inline diagnostic marker spans for a given file line index
fn diagnostic_markers_for_line(app: &App, line_idx: usize) -> Vec<Span<'static>> {
    let mut markers = vec![];
    for diag in &app.diagnostics {
        let start_line = diag.range.start.line as usize;
        if start_line == line_idx {
            let (marker, color) = match diag.severity {
                Some(lsp_types::DiagnosticSeverity::ERROR) => (" ● E", Color::Red),
                Some(lsp_types::DiagnosticSeverity::WARNING) => (" ● W", Color::Yellow),
                _ => continue,
            };
            markers.push(Span::styled(
                marker.to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
    }
    markers
}

fn render(app: &mut App, frame: &mut Frame) {
    let size = frame.size();
    let line_num_width = app.line_number_width();

    let (content_area, status_area, diagnostics_area) = if app.diagnostic_panel_visible {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(15),
                Constraint::Length(1),
            ])
            .split(size);
        (chunks[0], chunks[2], Some(chunks[1]))
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(size);
        (chunks[0], chunks[1], None)
    };

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

    // Get highlight spans for the visible content
    let highlight_spans = get_highlight_spans(app, &visible_lines);

    let text_content: Vec<Line> = visible_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_idx = app.scroll_offset + i;
            let cursor = app.cursor();

            // Build highlighted spans for this line
            let mut spans = vec![];
            let line_spans = highlight_spans.get(&i).cloned().unwrap_or_default();

            if line_idx == cursor.line {
                // Cursor line: apply highlighting then insert cursor
                let before_col = cursor.column.min(line.chars().count());
                let mut char_idx = 0;
                let mut cursor_rendered = false;

                for span in line_spans {
                    // span.start and span.end are already char offsets from get_highlight_spans
                    let span_start_char = span.start;
                    let span_end_char = span.end;

                    // Add text before this span
                    if span_start_char > char_idx && span_start_char <= before_col {
                        let text: String = line
                            .chars()
                            .skip(char_idx)
                            .take(span_start_char - char_idx)
                            .collect();
                        if !text.is_empty() {
                            spans.push(Span::raw(text));
                        }
                    }

                    // Check if cursor is in this span
                    if before_col >= span_start_char && before_col < span_end_char {
                        // Split span around cursor
                        let before_cursor: String = line
                            .chars()
                            .skip(span_start_char)
                            .take(before_col - span_start_char)
                            .collect();
                        let at_cursor = line
                            .chars()
                            .nth(before_col)
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| " ".to_string());
                        let after_cursor: String = line
                            .chars()
                            .skip(before_col + 1)
                            .take(span_end_char - before_col - 1)
                            .collect();

                        if !before_cursor.is_empty() {
                            spans.push(Span::styled(
                                before_cursor,
                                highlight_to_style(&span.highlight),
                            ));
                        }
                        spans.push(Span::styled(
                            at_cursor,
                            Style::default()
                                .bg(Color::White)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ));
                        cursor_rendered = true;
                        if !after_cursor.is_empty() {
                            spans.push(Span::styled(
                                after_cursor,
                                highlight_to_style(&span.highlight),
                            ));
                        }
                    } else if span_end_char <= before_col {
                        // Span is entirely before cursor
                        let text: String = line
                            .chars()
                            .skip(span_start_char)
                            .take(span_end_char - span_start_char)
                            .collect();
                        if !text.is_empty() {
                            spans.push(Span::styled(text, highlight_to_style(&span.highlight)));
                        }
                    } else if span_start_char > before_col {
                        // Span is entirely after cursor - need to render cursor first if not already done
                        // Add text from char_idx to cursor position
                        if char_idx < before_col {
                            let before_cursor_text: String = line
                                .chars()
                                .skip(char_idx)
                                .take(before_col - char_idx)
                                .collect();
                            if !before_cursor_text.is_empty() {
                                spans.push(Span::raw(before_cursor_text));
                            }
                        }
                        // Add cursor character
                        let at_cursor = line
                            .chars()
                            .nth(before_col)
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| " ".to_string());
                        spans.push(Span::styled(
                            at_cursor,
                            Style::default()
                                .bg(Color::White)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD),
                        ));
                        cursor_rendered = true;
                        // Add text from cursor+1 to span start
                        if before_col + 1 < span_start_char {
                            let after_cursor_text: String = line
                                .chars()
                                .skip(before_col + 1)
                                .take(span_start_char - before_col - 1)
                                .collect();
                            if !after_cursor_text.is_empty() {
                                spans.push(Span::raw(after_cursor_text));
                            }
                        }
                        // Now add the span itself
                        let text: String = line
                            .chars()
                            .skip(span_start_char)
                            .take(span_end_char - span_start_char)
                            .collect();
                        if !text.is_empty() {
                            spans.push(Span::styled(text, highlight_to_style(&span.highlight)));
                        }
                        // Mark cursor as handled by setting char_idx past it
                        char_idx = span_end_char.max(before_col + 1);
                    }

                    char_idx = span_end_char.max(char_idx);
                }

                // Add any remaining text after last span, and render cursor if not already rendered
                if !cursor_rendered && before_col < line.chars().count() {
                    // Need to render cursor in the remaining text
                    if char_idx < before_col {
                        let before_cursor: String = line
                            .chars()
                            .skip(char_idx)
                            .take(before_col - char_idx)
                            .collect();
                        if !before_cursor.is_empty() {
                            spans.push(Span::raw(before_cursor));
                        }
                    }
                    let at_cursor = line
                        .chars()
                        .nth(before_col)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| " ".to_string());
                    spans.push(Span::styled(
                        at_cursor,
                        Style::default()
                            .bg(Color::White)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    ));
                    if before_col + 1 < line.chars().count() {
                        let after_cursor: String = line.chars().skip(before_col + 1).collect();
                        if !after_cursor.is_empty() {
                            spans.push(Span::raw(after_cursor));
                        }
                    }
                } else if char_idx < line.chars().count() {
                    // No cursor to render, just remaining text
                    let remaining: String = line.chars().skip(char_idx).collect();
                    if !remaining.is_empty() {
                        spans.push(Span::raw(remaining));
                    }
                }

                // If no spans at all or cursor not rendered, use simple cursor rendering
                if spans.is_empty() || !cursor_rendered {
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
                }

                // Add inline diagnostic markers at end of cursor line
                let mut marker_spans = diagnostic_markers_for_line(app, line_idx);
                spans.append(&mut marker_spans);

                Line::from(spans)
            } else {
                // Non-cursor line: apply highlighting only
                if line_spans.is_empty() {
                    let mut spans = vec![Span::raw(line.clone())];
                    spans.extend(diagnostic_markers_for_line(app, line_idx));
                    Line::from(spans)
                } else {
                    let mut styled_spans = vec![];
                    let mut last_end = 0;

                    for span in line_spans {
                        // Add gap between spans
                        if span.start > last_end {
                            let gap: String = line
                                .chars()
                                .skip(last_end)
                                .take(span.start - last_end)
                                .collect();
                            if !gap.is_empty() {
                                styled_spans.push(Span::raw(gap));
                            }
                        }

                        // Add highlighted span
                        let text: String = line
                            .chars()
                            .skip(span.start)
                            .take(span.end - span.start)
                            .collect();
                        if !text.is_empty() {
                            styled_spans
                                .push(Span::styled(text, highlight_to_style(&span.highlight)));
                        }

                        last_end = span.end;
                    }

                    // Add remaining text
                    if last_end < line.chars().count() {
                        let remaining: String = line.chars().skip(last_end).collect();
                        if !remaining.is_empty() {
                            styled_spans.push(Span::raw(remaining));
                        }
                    }

                    // Add inline diagnostic markers at end of highlighted line
                    styled_spans.extend(diagnostic_markers_for_line(app, line_idx));
                    Line::from(styled_spans)
                }
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
        let (errors, warnings) = app.diagnostic_counts();
        let diag_str = if errors > 0 || warnings > 0 {
            format!(" | {}E {}W", errors, warnings)
        } else {
            String::new()
        };
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
            Span::styled(
                diag_str,
                Style::default().fg(if errors > 0 {
                    Color::Red
                } else {
                    Color::Yellow
                }),
            ),
        ]
    };

    let status_bar = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, status_area);

    // Render diagnostic detail panel if visible
    if let Some(diag_area) = diagnostics_area {
        let mut diag_lines: Vec<Line> = vec![];
        if app.diagnostics.is_empty() {
            diag_lines.push(Line::from(Span::styled(
                "No diagnostics",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for diag in &app.diagnostics {
                let severity_text = match diag.severity {
                    Some(lsp_types::DiagnosticSeverity::ERROR) => "[ERROR]",
                    Some(lsp_types::DiagnosticSeverity::WARNING) => "[WARN]",
                    Some(lsp_types::DiagnosticSeverity::INFORMATION) => "[INFO]",
                    Some(lsp_types::DiagnosticSeverity::HINT) => "[HINT]",
                    None => "     ",
                    _ => "[?]  ",
                };
                let severity_color = match diag.severity {
                    Some(lsp_types::DiagnosticSeverity::ERROR) => Color::Red,
                    Some(lsp_types::DiagnosticSeverity::WARNING) => Color::Yellow,
                    Some(lsp_types::DiagnosticSeverity::INFORMATION) => Color::Cyan,
                    Some(lsp_types::DiagnosticSeverity::HINT) => Color::Green,
                    None => Color::Gray,
                    _ => Color::Gray,
                };
                let line_num = diag.range.start.line + 1;
                let message = diag.message.split('\n').next().unwrap_or("").to_string();
                let text = format!("{:6} L{:4} {}", severity_text, line_num, message);
                diag_lines.push(Line::from(Span::styled(
                    text,
                    Style::default().fg(severity_color),
                )));
            }
        }

        let diag_panel = Paragraph::new(diag_lines)
            .block(Block::default().title("Diagnostics").borders(Borders::ALL))
            .style(Style::default().bg(Color::Black).fg(Color::White));
        frame.render_widget(diag_panel, diag_area);
    }
}
