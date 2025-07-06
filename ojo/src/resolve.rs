use {
    anyhow::Result,
    clap::Parser,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    libojo::{
        Changes, Graggle, NodeId, Repo,
        resolver::{CandidateChain, CycleResolver, OrderResolver},
    },
    ratatui::{
        Frame, Terminal,
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout, Rect},
        style::{Color, Modifier, Style},
        text::{Line, Span, Text},
        widgets::{Block, Borders, List, ListItem, Paragraph},
    },
    std::{
        io::{self, Stdout},
        time::Duration,
    },
};

#[derive(Parser, Debug)]
pub struct Opts {
    /// branch to work on
    #[arg(short, long)]
    branch: Option<String>,
    /// the person doing the resolving
    #[arg(short, long)]
    author: String,
    /// disables the display, which is useful when writing tests
    #[arg(long)]
    testing: bool,
}

pub fn run(opts: Opts) -> Result<()> {
    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, opts.branch);
    let graggle = repo.graggle(&branch)?;

    let changes = if opts.testing {
        // In testing mode, we bypass the UI and just use stdin directly
        let mut cycle_resolver = CycleResolver::new(graggle);
        while let Some(component) = cycle_resolver.next_component() {
            let component = component.iter().cloned().collect::<Vec<_>>();
            if let Some(first) = component.first() {
                cycle_resolver.resolve_component(*first);
            }
        }
        let order_resolver = cycle_resolver.into_order_resolver();
        Some(order_resolver.changes())
    } else {
        run_ui(&repo, graggle)?
    };

    if let Some(changes) = changes {
        let id = repo.create_patch(&opts.author, "Resolve to a file", changes)?;
        repo.write()?;
        println!("Created patch {}", id.to_base64());
    } else {
        eprintln!("No patch created");
    }

    Ok(())
}

fn run_ui(repo: &Repo, graggle: Graggle) -> Result<Option<Changes>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, repo, graggle);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    repo: &Repo,
    graggle: Graggle,
) -> Result<Option<Changes>> {
    let mut cycle_state = CycleResolverState::new(repo, CycleResolver::new(graggle));

    // First, resolve all cycles
    while let Some(component) = cycle_state.resolver.next_component() {
        let component = component.iter().cloned().collect::<Vec<_>>();
        let selected = run_cycle_ui(terminal, &mut cycle_state, component)?;
        if let Some(selected) = selected {
            cycle_state.resolver.resolve_component(selected);
        } else {
            return Ok(None); // User cancelled
        }
    }

    // Then resolve the ordering
    let order_resolver = cycle_state.resolver.into_order_resolver();
    let mut order_state = OrderResolverState::new(repo, order_resolver);
    run_order_ui(terminal, &mut order_state)
}

struct CycleResolverState<'a> {
    repo: &'a Repo,
    resolver: CycleResolver<'a>,
    offset: usize,
}

impl<'a> CycleResolverState<'a> {
    fn new(repo: &'a Repo, resolver: CycleResolver<'a>) -> Self {
        Self {
            repo,
            resolver,
            offset: 0,
        }
    }
}

struct OrderResolverState<'a> {
    repo: &'a Repo,
    resolver: OrderResolver<'a>,
    shown_first: usize,
}

impl<'a> OrderResolverState<'a> {
    fn new(repo: &'a Repo, resolver: OrderResolver<'a>) -> Self {
        Self {
            repo,
            resolver,
            shown_first: 0,
        }
    }
}

fn run_cycle_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut CycleResolverState,
    component: Vec<NodeId>,
) -> Result<Option<NodeId>> {
    loop {
        terminal.draw(|f| draw_cycle_ui(f, state, &component))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match handle_cycle_input(key, state, &component) {
                    InputResult::Select(node_id) => return Ok(Some(node_id)),
                    InputResult::Cancel => return Ok(None),
                    InputResult::Continue => {}
                }
            }
        }
    }
}

fn run_order_ui(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut OrderResolverState,
) -> Result<Option<Changes>> {
    loop {
        let candidates = state.resolver.candidates().collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(Some(state.resolver.changes()));
        }

        terminal.draw(|f| draw_order_ui(f, state, &candidates))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match handle_order_input(key, state, &candidates) {
                    InputResult::Continue => {}
                    InputResult::Cancel => return Ok(None),
                    InputResult::Select(_) => {} // Not used in order phase
                }
            }
        }
    }
}

enum InputResult {
    Continue,
    Cancel,
    Select(NodeId),
}

fn handle_cycle_input(
    key: KeyEvent,
    state: &mut CycleResolverState,
    component: &[NodeId],
) -> InputResult {
    match key.code {
        KeyCode::Esc => InputResult::Cancel,
        KeyCode::Char(c) => {
            if let Some(digit) = c.to_digit(10) {
                let index = digit as usize - 1;
                if index < 10 && state.offset + index < component.len() {
                    return InputResult::Select(component[state.offset + index]);
                }
            }
            match c {
                'j' => {
                    if state.offset + 10 < component.len() {
                        state.offset += 10;
                    }
                }
                'k' => {
                    if state.offset >= 10 {
                        state.offset -= 10;
                    } else {
                        state.offset = 0;
                    }
                }
                _ => {}
            }
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn handle_order_input(
    key: KeyEvent,
    state: &mut OrderResolverState,
    candidates: &[CandidateChain],
) -> InputResult {
    match key.code {
        KeyCode::Esc => InputResult::Cancel,
        KeyCode::Char(c) => {
            let chosen = |x: usize| {
                if x < 5 && state.shown_first + x < candidates.len() {
                    Some(&candidates[state.shown_first + x])
                } else {
                    None
                }
            };

            match c {
                '1'..='5' => {
                    let index = c as usize - '1' as usize;
                    if let Some(cand) = chosen(index) {
                        state.resolver.choose(&cand.first());
                    }
                }
                'q' | 'w' | 'e' | 'r' | 't' => {
                    let index = match c {
                        'q' => 0,
                        'w' => 1,
                        'e' => 2,
                        'r' => 3,
                        't' => 4,
                        _ => return InputResult::Continue,
                    };
                    if let Some(cand) = chosen(index) {
                        state.resolver.delete(&cand.first());
                    }
                }
                '!' | '@' | '#' | '$' | '%' => {
                    let index = match c {
                        '!' => 0,
                        '@' => 1,
                        '#' => 2,
                        '$' => 3,
                        '%' => 4,
                        _ => return InputResult::Continue,
                    };
                    if let Some(cand) = chosen(index) {
                        for u in cand.iter() {
                            state.resolver.choose(&u);
                        }
                    }
                }
                'Q' | 'W' | 'E' | 'R' | 'T' => {
                    let index = match c {
                        'Q' => 0,
                        'W' => 1,
                        'E' => 2,
                        'R' => 3,
                        'T' => 4,
                        _ => return InputResult::Continue,
                    };
                    if let Some(cand) = chosen(index) {
                        for u in cand.iter() {
                            state.resolver.delete(&u);
                        }
                    }
                }
                'j' => {
                    if state.shown_first + 5 < candidates.len() {
                        state.shown_first += 5;
                    }
                }
                'k' => {
                    if state.shown_first >= 5 {
                        state.shown_first -= 5;
                    } else {
                        state.shown_first = 0;
                    }
                }
                _ => {}
            }
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}

fn draw_cycle_ui(f: &mut Frame, state: &CycleResolverState, component: &[NodeId]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(8)].as_ref())
        .split(f.area());

    // Draw the component lines
    let end = (state.offset + 10).min(component.len());
    let visible_items = &component[state.offset..end];

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .map(|(i, node_id)| {
            let content = String::from_utf8_lossy(state.repo.contents(node_id));
            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", i + 1),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(content.trim_end().to_string()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Choose a line to resolve cycle"),
    );
    f.render_widget(list, chunks[0]);

    // Draw keybindings
    let keybindings = create_cycle_keybindings(visible_items.len(), state.offset, component.len());
    let help_text = Text::from(keybindings);
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Keybindings"));
    f.render_widget(help, chunks[1]);
}

fn draw_order_ui(f: &mut Frame, state: &OrderResolverState, candidates: &[CandidateChain]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(8),
            ]
            .as_ref(),
        )
        .split(f.area());

    // Draw ordered nodes (completed part)
    let done = state.resolver.ordered_nodes().to_owned();
    let done_items: Vec<ListItem> = done
        .iter()
        .rev()
        .take(chunks[0].height as usize)
        .map(|node_id| {
            let content = String::from_utf8_lossy(state.repo.contents(node_id));
            ListItem::new(Line::from(content.trim_end().to_string()))
        })
        .collect();

    let done_list =
        List::new(done_items).block(Block::default().borders(Borders::ALL).title("Completed"));
    f.render_widget(done_list, chunks[0]);

    // Draw separator
    let separator = Paragraph::new("═".repeat(chunks[1].width as usize))
        .style(Style::default().fg(Color::Gray));
    f.render_widget(separator, chunks[1]);

    // Draw candidates
    if candidates.len() == 1 {
        draw_single_candidate(f, chunks[2], state, &candidates[0]);
    } else if candidates.len() == 2 {
        draw_two_candidates(f, chunks[2], state, candidates);
    } else {
        draw_many_candidates(f, chunks[2], state, candidates);
    }
}

fn draw_single_candidate(
    f: &mut Frame,
    area: Rect,
    state: &OrderResolverState,
    candidate: &CandidateChain,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(25)].as_ref())
        .split(area);

    let items: Vec<ListItem> = candidate
        .iter()
        .take(5)
        .map(|node_id| {
            let content = String::from_utf8_lossy(state.repo.contents(&node_id));
            ListItem::new(Line::from(content.trim_end().to_string()))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Candidate"));
    f.render_widget(list, chunks[0]);

    let help_lines = vec![
        Line::from("1 - take one"),
        Line::from("q - delete one"),
        Line::from("! - take all"),
        Line::from("Q - delete all"),
        Line::from("ESC - quit"),
    ];
    let help =
        Paragraph::new(help_lines).block(Block::default().borders(Borders::ALL).title("Keys"));
    f.render_widget(help, chunks[1]);
}

fn draw_two_candidates(
    f: &mut Frame,
    area: Rect,
    state: &OrderResolverState,
    candidates: &[CandidateChain],
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ]
            .as_ref(),
        )
        .split(area);

    // Left candidate
    let left_items: Vec<ListItem> = candidates[0]
        .iter()
        .take(5)
        .map(|node_id| {
            let content = String::from_utf8_lossy(state.repo.contents(&node_id));
            ListItem::new(Line::from(content.trim_end().to_string()))
        })
        .collect();

    let left_list = List::new(left_items).block(Block::default().borders(Borders::ALL).title("1"));
    f.render_widget(left_list, chunks[0]);

    // Right candidate
    let right_items: Vec<ListItem> = candidates[1]
        .iter()
        .take(5)
        .map(|node_id| {
            let content = String::from_utf8_lossy(state.repo.contents(&node_id));
            ListItem::new(Line::from(content.trim_end().to_string()))
        })
        .collect();

    let right_list =
        List::new(right_items).block(Block::default().borders(Borders::ALL).title("2"));
    f.render_widget(right_list, chunks[1]);

    // Keybindings
    let help_lines = vec![
        Line::from("1 - take left"),
        Line::from("2 - take right"),
        Line::from("q - delete left"),
        Line::from("w - delete right"),
        Line::from("ESC - quit"),
    ];
    let help =
        Paragraph::new(help_lines).block(Block::default().borders(Borders::ALL).title("Keys"));
    f.render_widget(help, chunks[2]);
}

fn draw_many_candidates(
    f: &mut Frame,
    area: Rect,
    state: &OrderResolverState,
    candidates: &[CandidateChain],
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)].as_ref())
        .split(area);

    let num_candidates = 5.min(candidates.len() - state.shown_first);
    let items: Vec<ListItem> = (0..num_candidates)
        .map(|i| {
            let cand_idx = state.shown_first + i;
            let candidate = &candidates[cand_idx];
            let key = (i + 1).to_string();
            let first_node = candidate.first();
            let content = String::from_utf8_lossy(state.repo.contents(&first_node));

            let line = Line::from(vec![
                Span::styled(
                    format!("{key} "),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(content.trim_end().to_string()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Candidates"));
    f.render_widget(list, chunks[0]);

    let help_lines =
        create_many_candidates_help(num_candidates, state.shown_first, candidates.len());
    let help =
        Paragraph::new(help_lines).block(Block::default().borders(Borders::ALL).title("Keys"));
    f.render_widget(help, chunks[1]);
}

fn create_cycle_keybindings(
    visible_count: usize,
    offset: usize,
    total_count: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    if visible_count > 0 {
        lines.push(Line::from(format!("1-{visible_count} - choose line")));
    }

    if offset + 10 < total_count {
        lines.push(Line::from("j - show next"));
    }

    if offset > 0 {
        lines.push(Line::from("k - show previous"));
    }

    lines.push(Line::from("ESC - quit"));
    lines
}

fn create_many_candidates_help(
    num_candidates: usize,
    shown_first: usize,
    total_candidates: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.push(Line::from(format!("1-{num_candidates} - take line")));
    lines.push(Line::from(format!(
        "q-{} - delete line",
        match num_candidates {
            1 => "q",
            2 => "w",
            3 => "e",
            4 => "r",
            5 => "t",
            _ => "t",
        }
    )));
    lines.push(Line::from(format!(
        "!-{} - take all lines",
        match num_candidates {
            1 => "!",
            2 => "@",
            3 => "#",
            4 => "$",
            5 => "%",
            _ => "%",
        }
    )));
    lines.push(Line::from(format!(
        "Q-{} - delete all lines",
        match num_candidates {
            1 => "Q",
            2 => "W",
            3 => "E",
            4 => "R",
            5 => "T",
            _ => "T",
        }
    )));

    if shown_first > 0 {
        lines.push(Line::from("k - show previous"));
    }

    if shown_first + 5 < total_candidates {
        lines.push(Line::from("j - show next"));
    }

    lines.push(Line::from("ESC - quit"));
    lines
}
