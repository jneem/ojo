use clap::ArgMatches;
use failure::Error;
use libojo::resolver::{CandidateChain, CycleResolver, OrderResolver};
use libojo::{Changes, Graggle, NodeId, Repo};
use std::io::Write;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::{clear, cursor, style};

pub fn run(m: &ArgMatches<'_>) -> Result<(), Error> {
    // The unwrap is ok because this is a required argument.
    let author = m.value_of("author").unwrap();

    let mut repo = super::open_repo()?;
    let branch = super::branch(&repo, m);
    let graggle = repo.graggle(&branch)?;

    let changes = {
        // Here we use the alternate screen, so nothing we print in this scope will be visible
        // after the scope ends.
        let stdout = std::io::stdout();
        let screen = AlternateScreen::from(stdout).into_raw_mode()?;
        let stdin = std::io::stdin();

        // TODO: check if the terminal is big enough.
        write!(std::io::stdout(), "{}", cursor::Hide)?;
        let cycle = CycleResolverState::new(&repo, screen, stdin.keys(), graggle)?;
        if let Some(order) = cycle.run()? {
            order.run()?
        } else {
            None
        }
    };
    write!(std::io::stdout(), "{}", cursor::Show)?;
    // TODO: the flush is currently necessary for the eprintln to work; see
    // https://gitlab.redox-os.org/redox-os/termion/issues/158
    std::io::stdout().flush()?;

    if let Some(changes) = changes {
        let id = repo.create_patch(author, "Resolve to a file", changes)?;
        repo.write()?;
        eprintln!("Created patch {}", id.to_base64());
    } else {
        eprintln!("No patch created");
    }

    Ok(())
}

const NUMBERS: &[u8] = b"1234567890";
const NUMBERS_UPPER: &[u8] = b"!@#$%^&*()";
const QWERTY: &[u8] = b"qwertyuiop";
const QWERTY_UPPER: &[u8] = b"QWERTYUIOP";

type Screen = termion::raw::RawTerminal<AlternateScreen<std::io::Stdout>>;
type Input = termion::input::Keys<std::io::Stdin>;

struct CycleResolverState<'a> {
    repo: &'a Repo,
    screen: Screen,
    input: Input,
    resolver: CycleResolver<'a>,

    // Dimensions of the screen.
    width: u16,
}

impl<'a> CycleResolverState<'a> {
    fn new(
        repo: &'a Repo,
        screen: Screen,
        input: Input,
        graggle: Graggle<'a>,
    ) -> Result<CycleResolverState<'a>, Error> {
        let (width, _) = termion::terminal_size()?;

        Ok(CycleResolverState {
            repo,
            screen,
            input,
            resolver: CycleResolver::new(graggle),
            width,
        })
    }

    fn run(mut self) -> Result<Option<OrderResolverState<'a>>, Error> {
        while let Some(component) = self.resolver.next_component() {
            let component = component.iter().cloned().collect::<Vec<_>>();

            // We show at most 10 lines on a page; this is the index of the first shown line.
            let mut offset = 0;

            // Loop until we resolve the current component.
            loop {
                let end = (offset + 10).max(component.len());
                self.redraw(&component[offset..end])?;
                let key = self
                    .input
                    .next()
                    .ok_or_else(|| failure::err_msg("Unexpected end of input"))??;
                match key {
                    Key::Char(c) => {
                        if let Some(x) = NUMBERS.iter().position(|&a| a == c as u8) {
                            if offset + x < end {
                                self.resolver.resolve_component(component[offset + x]);
                                break;
                            }
                        } else if c == 'j' && offset + 10 < component.len() {
                            offset += 10;
                        } else if c == 'k' && offset > 0 {
                            offset -= 10;
                        }
                    }
                    Key::Esc => {
                        return Ok(None);
                    }
                    _ => {
                        debug!("unknown key");
                    }
                }
            }
        }
        let resolver = self.resolver.into_order_resolver();
        OrderResolverState::new(self.repo, self.screen, self.input, resolver).map(Some)
    }

    fn redraw(&mut self, lines: &[NodeId]) -> Result<(), Error> {
        for (i, u) in lines.iter().enumerate() {
            write!(
                self.screen,
                "{goto}{key} {line}",
                key = NUMBERS[i],
                goto = cursor::Goto(1, 1 + (i as u16)),
                line = String::from_utf8_lossy(self.repo.contents(u)),
            )?;
        }

        let keys = format!("1-{}", NUMBERS[lines.len() - 1] as char);
        draw_keybindings(
            &mut self.screen,
            vec![
                (&keys[..], "choose line"),
                ("k", "show previous"),
                ("j", "show next"),
                ("ESC", "quit"),
            ],
            self.width,
        )?;

        self.screen.flush()?;
        Ok(())
    }
}

struct OrderResolverState<'a> {
    repo: &'a Repo,
    screen: Screen,
    input: Input,
    resolver: OrderResolver<'a>,

    // Dimensions of the screen.
    width: u16,
    height: u16,

    // If there are many candidates available, we only show a few (up to 5) at a time. What's the
    // index of the first visible one?
    shown_first: usize,
}

impl<'a> OrderResolverState<'a> {
    fn new(
        repo: &'a Repo,
        screen: Screen,
        input: Input,
        resolver: OrderResolver<'a>,
    ) -> Result<OrderResolverState<'a>, Error> {
        let (width, height) = termion::terminal_size()?;

        Ok(OrderResolverState {
            repo,
            screen,
            input,
            resolver,
            width,
            height,
            shown_first: 0,
        })
    }

    fn run(mut self) -> Result<Option<Changes>, Error> {
        loop {
            let candidates = self.resolver.candidates().collect::<Vec<_>>();
            if candidates.is_empty() {
                return Ok(Some(self.resolver.changes()));
            }

            self.shown_first = 0;

            self.redraw()?;

            let key = self
                .input
                .next()
                .ok_or_else(|| failure::err_msg("Unexpected end of input"))??;
            match key {
                Key::Char(c) => {
                    let chosen = |x: usize| {
                        if x < 5 && self.shown_first + x < candidates.len() {
                            Some(&candidates[self.shown_first + x])
                        } else {
                            None
                        }
                    };

                    if let Some(x) = NUMBERS.iter().position(|&a| a == c as u8) {
                        if let Some(cand) = chosen(x) {
                            self.resolver.choose(&cand.first());
                        }
                    } else if let Some(x) = QWERTY.iter().position(|&a| a == c as u8) {
                        if let Some(cand) = chosen(x) {
                            self.resolver.delete(&cand.first());
                        }
                    } else if let Some(x) = NUMBERS_UPPER.iter().position(|&a| a == c as u8) {
                        if let Some(cand) = chosen(x) {
                            for u in cand.iter() {
                                self.resolver.choose(&u);
                            }
                        }
                    } else if let Some(x) = QWERTY_UPPER.iter().position(|&a| a == c as u8) {
                        if let Some(cand) = chosen(x) {
                            for u in cand.iter() {
                                self.resolver.delete(&u);
                            }
                        }
                    } else if c == 'j' {
                        if self.shown_first + 5 < candidates.len() {
                            self.shown_first += 5;
                        }
                    } else if c == 'k' {
                        if self.shown_first > 0 {
                            assert!(self.shown_first >= 5);
                            self.shown_first -= 5;
                        }
                    }
                }
                Key::Esc => {
                    return Ok(None);
                }
                _ => {
                    debug!("unknown key");
                }
            }
        }
    }

    fn redraw(&mut self) -> Result<(), Error> {
        let divider_row = self.height - 5;
        write!(
            self.screen,
            "{clear}{goto}{line}",
            clear = clear::All,
            goto = cursor::Goto(1, divider_row),
            line = std::iter::repeat('═')
                .take(self.width as usize)
                .collect::<String>()
        )?;

        // Draw all the lines that are finished.
        // TODO: add line numbers
        let done = self.resolver.ordered_nodes().to_owned();
        let mut row = divider_row;
        for u in done.iter().rev().take(divider_row as usize - 1) {
            row -= 1;
            write_truncated(&mut self.screen, self.repo.contents(u), 1, row, self.width)?;
        }

        let candidates = self.resolver.candidates().collect::<Vec<_>>();
        // If there are no candidates, we are already done.
        assert!(!candidates.is_empty());

        if candidates.len() == 1 {
            self.redraw_one_choice(&candidates[0])?;
        } else if candidates.len() == 2 {
            self.redraw_two_choices(candidates)?;
        } else {
            self.redraw_many_choices(candidates)?;
        }

        self.screen.flush()?;
        Ok(())
    }

    fn redraw_one_choice(&mut self, candidate: &CandidateChain) -> Result<(), Error> {
        self.write_candidate_chain(candidate, 1, self.width)?;
        self.draw_keybindings(vec![
            ("1", "take one"),
            ("q", "delete one"),
            ("!", "take all"),
            ("Q", "delete all"),
        ])?;
        Ok(())
    }

    fn redraw_two_choices(&mut self, candidates: Vec<CandidateChain>) -> Result<(), Error> {
        let divider_col = (self.width + 1) / 2;
        let divider_row = self.height - 5;

        for i in 1..=5 {
            write!(
                self.screen,
                "{goto}│",
                goto = cursor::Goto(divider_col, divider_row + i)
            )?;
        }

        // Draw the two columns of options.
        self.write_candidate_chain(&candidates[0], 1, divider_col - 1)?;
        let col2_start = divider_col + 1;
        let col2_width = self.width - divider_col - 1;
        self.write_candidate_chain(&candidates[1], col2_start, col2_width)?;

        // Draw little boxes with the numbers 1 and 2 in them.
        write!(
            self.screen,
            "{goto1}│1│{goto2}│2{goto3}└─┤{goto4}└─",
            goto1 = cursor::Goto(divider_col - 2, divider_row + 1),
            goto2 = cursor::Goto(self.width - 1, divider_row + 1),
            goto3 = cursor::Goto(divider_col - 2, divider_row + 2),
            goto4 = cursor::Goto(self.width - 1, divider_row + 2),
        )?;

        self.draw_keybindings(vec![
            ("1", "take left"),
            ("2", "take right"),
            ("q", "delete left"),
            ("w", "delete right"),
            ("ESC", "quit"),
        ])
    }

    fn redraw_many_choices(&mut self, candidates: Vec<CandidateChain>) -> Result<(), Error> {
        let divider_row = self.height - 5;
        let num_candidates = 5.min(candidates.len() - self.shown_first);
        let mut row = divider_row;

        for i in 0..num_candidates {
            row += 1;

            let cand_idx = self.shown_first + i;
            let key = (b'1' + i as u8) as char;
            write!(
                self.screen,
                "{goto}{bold}{key}{unbold}",
                goto = cursor::Goto(1, row),
                bold = style::Bold,
                key = key,
                unbold = style::NoBold,
            )?;
            let u = candidates[cand_idx].first();
            write_truncated(
                &mut self.screen,
                self.repo.contents(&u),
                3,
                row,
                self.width - 2,
            )?;
        }

        let mut choose_range = b"1-5".to_owned();
        let mut choose_all_range = b"!-%".to_owned();
        let mut delete_range = b"q-t".to_owned();
        let mut delete_all_range = b"Q-T".to_owned();
        choose_range[2] = NUMBERS[num_candidates - 1];
        choose_all_range[2] = NUMBERS_UPPER[num_candidates - 1];
        delete_range[2] = QWERTY[num_candidates - 1];
        delete_all_range[2] = QWERTY_UPPER[num_candidates - 1];

        let mut keybindings = vec![
            (std::str::from_utf8(&choose_range[..]).unwrap(), "take line"),
            (
                std::str::from_utf8(&delete_range[..]).unwrap(),
                "delete line",
            ),
            (
                std::str::from_utf8(&choose_all_range[..]).unwrap(),
                "take lines",
            ),
            (
                std::str::from_utf8(&delete_all_range[..]).unwrap(),
                "delete lines",
            ),
        ];

        if self.shown_first > 0 {
            keybindings.push(("k", "show previous"));
        }
        if self.shown_first + 5 < candidates.len() {
            keybindings.push(("j", "show next"));
        }
        keybindings.push(("ESC", "quit"));
        self.draw_keybindings(keybindings)?;
        Ok(())
    }

    fn draw_keybindings(&mut self, bindings: Vec<(&str, &str)>) -> Result<(), Error> {
        draw_keybindings(&mut self.screen, bindings, self.width)
    }

    fn write_candidate_chain(
        &mut self,
        chain: &CandidateChain,
        col: u16,
        max_width: u16,
    ) -> Result<(), Error> {
        let mut row = self.height - 5;
        for u in chain.iter().take(5) {
            row += 1;
            let data = self.repo.contents(&u);
            write_truncated(&mut self.screen, data, col, row, max_width)?;
        }
        Ok(())
    }
}

fn draw_keybindings(
    screen: &mut Screen,
    bindings: Vec<(&str, &str)>,
    width: u16,
) -> Result<(), Error> {
    let mut row = 1;
    for (key, msg) in bindings {
        write!(
            screen,
            "{goto}│{bold}{key}{reset}{gotomsg}{msg}",
            goto = cursor::Goto(width - 20, row),
            bold = style::Bold,
            key = key,
            reset = style::Reset,
            gotomsg = cursor::Goto(width - 15, row),
            msg = msg,
        )?;
        row += 1;
    }
    write!(
        screen,
        "{goto}└{line}",
        goto = cursor::Goto(width - 20, row),
        line = std::iter::repeat("─").take(20).collect::<String>()
    )?;
    Ok(())
}

fn write_truncated(
    screen: &mut Screen,
    data: &[u8],
    col: u16,
    row: u16,
    max_width: u16,
) -> Result<(), Error> {
    let mut data = String::from_utf8_lossy(data);
    // TODO: Here, we're pretending that the number of chars is the same as the displayed
    // width. Is there some crate to help us figure out the actual width?
    if data.chars().count() > max_width as usize {
        let mut truncated = data
            .chars()
            .take(max_width as usize - 3)
            .collect::<String>();
        truncated += "...";
        data = std::borrow::Cow::from(truncated);
    }
    // Trim the string, because if it ends with a '\n' then it will mess up our formatting.
    write!(screen, "{}{}", cursor::Goto(col, row), data.trim_end())?;
    Ok(())
}
