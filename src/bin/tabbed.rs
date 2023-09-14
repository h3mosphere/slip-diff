use std::{
    error::Error,
    fs, io,
    path::PathBuf,
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use ratatui::{prelude::*, widgets::*};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(short, long, required = true)]
    pub file: PathBuf,

    #[clap(short, long)]
    pub clear: bool,
}

#[derive(Clone)]
struct FileVersion {
    pub contents: String,
    pub at: Instant,
}

impl FileVersion {
    pub fn new_at_now(contents: String) -> Self {
        let at = Instant::now();
        Self { contents, at }
    }
}

struct App {
    pub versions: Vec<FileVersion>,
    pub index: usize,
}

impl App {
    fn new() -> App {
        App {
            versions: Vec::new(),
            index: 0,
        }
    }

    pub fn next(&mut self) {
        let len = self.versions.len();
        self.index = (self.index + 1) % (len - 1);
    }

    pub fn previous(&mut self) {
        let len = self.versions.len();
        if self.index > 0 {
            self.index -= 1;
        } else {
            self.index = len - 2;
        }
    }

    pub fn current_contents(&self) -> String {
        self.versions[self.index].contents.clone()
    }

    pub fn next_contents(&self) -> Option<String> {
        self.versions
            .get(self.index + 1)
            .and_then(|f| Some(f.contents.clone()))
    }

    pub fn push_version(&mut self, version: FileVersion) {
        self.versions.push(version);
    }

    pub fn push_contents(&mut self, contents: String) -> Result<(), Box<dyn Error>> {
        let fv = FileVersion::new_at_now(contents);
        self.versions.push(fv);
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app, &args);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    args: &Args,
) -> Result<(), Box<dyn Error>> {
    let path = &args.file;
    let zero = fs::read_to_string(&path)?;
    app.push_contents(zero)?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
    loop {
        while let Ok(res) = rx.try_recv() {
            match res {
                Ok(event) => match event.kind {
                    notify::EventKind::Modify(event) => match event {
                        notify::event::ModifyKind::Data(_) => {
                            let prev = app.versions.last().unwrap();
                            let contents = fs::read_to_string(&path)?;
                            let new = FileVersion::new_at_now(contents);
                            if prev.contents != new.contents {
                                app.push_version(new);
                            }
                        }
                        _ => {}
                    },
                    notify::EventKind::Any => {}
                    notify::EventKind::Access(_) => {}
                    notify::EventKind::Create(_) => {}
                    notify::EventKind::Remove(_) => {}
                    notify::EventKind::Other => {}
                },
                Err(error) => println!("Error: {error:?}"),
            }
        }
        terminal.draw(|f| ui(f, &app))?;

        if let Ok(true) = event::poll(Duration::from_micros(1)) {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Esc => return Ok(()),
                        KeyCode::Right => app.next(),
                        KeyCode::Left => app.previous(),
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(size);

    let block = Block::default().on_black().white();
    f.render_widget(block, size);

    let titles = app
        .versions
        .iter()
        .enumerate()
        .map(|(i, _v)| Line::from(format!("{}", i)))
        .collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .select(app.index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .white()
                .add_modifier(Modifier::BOLD)
                .bg(Color::Black),
        );
    f.render_widget(tabs, chunks[0]);

    let contents = app.current_contents();
    let split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);
    let original = Paragraph::new(contents);
    f.render_widget(original, split[0]);

    let changed = Paragraph::new(app.next_contents().unwrap_or("Nothing".into()));

    f.render_widget(changed, split[1]);
}
