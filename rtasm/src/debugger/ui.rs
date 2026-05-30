use ratatui::{
    layout::{Constraint, Layout, Spacing},
    prelude::{Buffer, Rect},
    style::{Color, Stylize},
    symbols::{border, merge::MergeStrategy},
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};

use core::fmt::Write;

use crate::debugger::{
    Emulator, LegacyMemstate, RunningRoutine, TICKS_PER_SECOND, layout::KEY_AREA_HEIGHT,
};

const KEYBINDS: &[(&str, &str)] = &[
    ("Esc", "Exit the emulator"),
    ("Space", "Un/pause the emulator"),
    ("Enter", "Spawn this IOBlock"),
    ("Up", "Previous IOBlock"),
    ("Down", "Next IOBlock"),
    ("PgUp", "Go to top of IOBlocks"),
    ("PgDn", "Go to end of IOBlocks"),
    (".", "Step forward when paused"),
    ("c", "Clear emulator logs"),
    ("r", "Reset VM state"),
    ("Tab", "Peek IOBlock"),
    ("-", "Slow down"),
    ("+", "Speed up"),
    ("0", "Reset speed"),
    ("u", "Toggle unlimited speed"),
    /* todos */
    /* active routine controls */
    // left: select next process
    // right: select prev. process (if at idx 0, set to None; none selected)
    // t: toggle off/on
    // p: pause/unpause
    // k: kill process
    // /: peek details of active process
    //  - next instruction to process (+ index) + instr time
    //  - previous instruction processed
    //  - waiting time (if waiting) // is paused
    //  - is toggled
    /* snapshot controls */
    // s: save snapshot
    // d: discard snapshot
    // a: revert to previous snapshot
    /* state query controls */
    // q: query a counter
    //  - this doesn't impede searching ability since `q` is irrelevant to item ids
    // e: exit query
    // w: watch counter
    /* memory scrolling */
    // [: scroll left
    // ]: scroll right
    // g: goto pointer position
];

const MEM_HIGHLIGHT: Color = Color::Rgb(255, 115, 15);
const MEM_HIGHLIGHT_BG: Color = Color::Rgb(60, 60, 60);
const MEM_ACCENT: Color = Color::Rgb(255, 230, 115);

impl Widget for &Emulator {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // outline
        Block::bordered()
            .border_set(border::ROUNDED)
            .render(area, buf);

        self.render_logbox(buf);
        self.render_displays(buf);
        self.render_keys(buf);
        self.render_memory(buf);
        self.render_memory_info(buf);
        self.render_ioblocks(buf);
        self.render_info(buf);

        if self.ui_state.peeking_ioblock {
            let bottom_left_layout =
                Layout::horizontal(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(self.ui_state.layout.routines_area);
            self.render_routines(bottom_left_layout[0], buf);
            self.render_ioblock_peek(bottom_left_layout[1], buf);
        } else {
            self.render_routines(self.ui_state.layout.routines_area, buf);
        }
    }
}

impl Emulator {
    fn render_logbox(&self, buf: &mut Buffer) {
        let logbox_height = (self.ui_state.layout.logbox_area.height - 2) as usize;
        let logs = if self.ui_state.logbox.len() > logbox_height {
            &self.ui_state.logbox[(self.ui_state.logbox.len() - logbox_height)..]
        } else {
            &self.ui_state.logbox[..]
        };

        // -2: 1 char padding on each side
        let max_log_length = (self.ui_state.layout.logbox_area.width - 2) as usize;

        Paragraph::new(Text::from(
            logs.iter()
                .map(|reflog| {
                    let mut log = reflog.clone();
                    let truncated = if log.len() > max_log_length {
                        log.truncate(max_log_length - 5);
                        true
                    } else {
                        false
                    };

                    Line::from(vec![
                        " ".into(),
                        log.into(),
                        match truncated {
                            true => "...".gray(),
                            false => "".into(),
                        },
                    ])
                })
                .collect::<Vec<Line<'_>>>(),
        ))
        .block(
            Block::bordered()
                .border_set(border::PLAIN)
                .title(" Emulator logs ".yellow()),
        )
        .render(self.ui_state.layout.logbox_area, buf);
    }

    fn render_displays(&self, buf: &mut Buffer) {
        let displays_height = self.ui_state.layout.display_area.height as usize;
        let displays = if self.ui_state.displays.len() > displays_height {
            &self.ui_state.displays[..displays_height]
        } else {
            &self.ui_state.displays[..]
        };

        Paragraph::new(Text::from(
            displays
                .iter()
                .map(|item| {
                    Line::from(format!(
                        " {:<13} : {:>12} ",
                        format!("{item:?}"),
                        self.state.get_item_value_str(*item)
                    ))
                })
                .collect::<Vec<Line<'_>>>(),
        ))
        .block(
            Block::bordered()
                .border_set(border::PLAIN)
                .title(" Displayed items ".green().into_centered_line()),
        )
        .render(self.ui_state.layout.display_area, buf);
    }

    fn render_keys(&self, buf: &mut Buffer) {
        Block::bordered()
            .border_set(border::EMPTY)
            .title(" Keys".bold())
            .render(self.ui_state.layout.keys_area, buf);

        let middle_h_temp = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(self.ui_state.layout.keys_area);

        let workable_area = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(middle_h_temp[1])[1];

        let keylines = KEYBINDS
            .iter()
            .map(|(key, bind)| {
                Line::from(vec![
                    "<".cyan(),
                    key.cyan().bold(),
                    "> ".cyan(),
                    (*bind).into(),
                ])
            })
            .collect::<Vec<Line<'_>>>();

        // 10 keys per column
        let mut curr_column = vec![];
        let mut columns = vec![];
        let mut widths = vec![];

        for line in keylines {
            curr_column.push(line);
            if curr_column.len() + 2 == KEY_AREA_HEIGHT as usize {
                // get max length for layout reasons
                let max_width = curr_column.iter().map(|l| l.width()).max().unwrap();
                columns.push(curr_column);
                widths.push(Constraint::Length((max_width + 1) as u16));
                curr_column = vec![];
            }
        }

        if !curr_column.is_empty() {
            columns.push(curr_column);
            widths.push(Constraint::Min(1));
        } else {
            let last = widths.len() - 1;
            widths[last] = Constraint::Min(1);
        }

        let cols = columns.iter().map(|lines| Text::from(lines.clone()));
        let h_layout = Layout::horizontal(widths).split(workable_area);

        for (idx, c) in cols.enumerate() {
            c.render(h_layout[idx], buf);
        }
    }

    // renders only the memory cells, not the state or any metadata
    fn render_memory(&self, buf: &mut Buffer) {
        if self.tasm.mem_info.is_none() {
            display_centered_message(
                "<No memory>",
                Block::bordered()
                    .border_set(border::PLAIN)
                    .title(" Memory cells ".blue().into_centered_line()),
                self.ui_state.layout.memory_cells_area,
                buf,
            );
            return;
        }

        // sections: title, body, metadata/info
        let split = Layout::vertical(vec![Constraint::Length(1), Constraint::Min(1)])
            .split(self.ui_state.layout.memory_cells_area);

        // render title
        Block::new()
            .title(Line::from("Memory cells").centered())
            .render(split[0], buf);

        // for padding on the left side
        let split2 =
            Layout::horizontal(vec![Constraint::Length(1), Constraint::Min(1)]).split(split[1]);

        let mut cols_area = split2[1];
        let mem = self.tasm.mem_info.as_ref().unwrap();

        // memory cell
        // ---------------
        // 034: 0000001234
        // ---------------
        // length: 2 (padding) + 1 (margin) + log10(memsize) (address) + 2 (": ") + 12 (number)
        // = 17 + log10(memsize)

        // set up preliminary size vars
        let idx_width = f64::log10(mem.size as f64).ceil() as usize;
        let col_width = 18 + idx_width;
        let max_cols = ((cols_area.width - 1) as f64 / col_width as f64) as u16;
        let col_height = cols_area.height - 2;

        cols_area.height = col_height + 2;

        let cols_division = Layout::horizontal(core::iter::repeat_n(
            Constraint::Fill((col_width + 2) as u16),
            max_cols as usize,
        ))
        .spacing(Spacing::Overlap(1))
        .split(cols_area);

        // now, build all of the lines for each column
        let mut cols = vec![];
        let mut idx = 0;

        while cols.len() < max_cols as usize && idx < mem.size {
            let mut curr_col = vec![];
            let start_idx = idx;
            while curr_col.len() < col_height as usize && idx < mem.size {
                let num = self.read_mem(idx);

                let is_selected = self.get_ptrpos_value().0 == idx as i32;

                let mut line_segments = vec![
                    " ".into(),
                    if is_selected {
                        "  >".fg(MEM_HIGHLIGHT)
                    } else {
                        format!("{idx:0>idx_width$}").fg(MEM_ACCENT)
                    },
                    if is_selected {
                        "  ".into()
                    } else {
                        ": ".into()
                    },
                ];

                render_number(&mut line_segments, num, mem.is_int(), is_selected);

                line_segments.push(" ".into());

                let mut line = Line::from(line_segments).centered();
                if is_selected {
                    line = line.bg(MEM_HIGHLIGHT_BG)
                }

                curr_col.push(line);
                idx += 1;
            }
            let end_idx = idx;

            cols.push((curr_col, start_idx, end_idx));
        }

        let ptrpos = self.get_ptrpos_value().0 as i16;

        for (i, (col, start, end)) in cols.into_iter().enumerate() {
            let is_ptr_here = start <= ptrpos && ptrpos <= end;
            Paragraph::new(Text::from(col))
                .block(
                    Block::bordered().merge_borders(MergeStrategy::Exact).title(
                        format!("[{start:0>idx_width$} - {end:0>idx_width$}]")
                            .fg(match is_ptr_here {
                                true => MEM_HIGHLIGHT,
                                false => MEM_ACCENT,
                            })
                            .into_centered_line(),
                    ),
                )
                .render(cols_division[i], buf);
        }
    }

    fn render_memory_info(&self, buf: &mut Buffer) {
        let mut lines = match self.tasm.mem_info {
            Some(ref mem) => {
                let mut memreg_line_segments = vec![Span::from("Register: ")];
                render_number(
                    &mut memreg_line_segments,
                    self.state.get_num(mem.memreg.to_item().unwrap()),
                    mem.is_int(),
                    false,
                );
                let mut ptrpos_line_segments = vec![Span::from("Position: ")];
                render_number(
                    &mut ptrpos_line_segments,
                    self.get_ptrpos_value().0 as f64,
                    true,
                    false,
                );
                vec![
                    Line::from(memreg_line_segments),
                    Line::from(ptrpos_line_segments),
                    format!(
                        "Memory: [{:?}] {} - {}",
                        mem.get_type(),
                        mem.start_counter_id,
                        mem.start_counter_id + mem.size - 1,
                    )
                    .into(),
                    format!("Size: {}", mem.size).into(),
                ]
            }
            None => vec![Line::from(
                "This program does not use memory.".fg(MEM_ACCENT),
            )],
        };

        match self.legacy_memstate {
            LegacyMemstate::None => {
                // check that legacy memory is being used here
                if let Some(ref mem) = self.tasm.mem_info
                    && (mem.is_legacy())
                {
                    lines.push(Line::from(vec![
                        "Memory mode: ".into(),
                        "uninitialised".gray().italic(),
                    ]))
                }
            }
            LegacyMemstate::Read => {
                lines.push(Line::from(vec!["Memory mode: ".into(), "READ".green()]))
            }
            LegacyMemstate::Write => {
                lines.push(Line::from(vec!["Memory mode: ".into(), "WRITE".red()]))
            }
        }

        Paragraph::new(Text::from(lines))
            .block(
                Block::bordered()
                    .border_set(border::EMPTY)
                    .title(" Memory State".bold().fg(MEM_ACCENT)),
            )
            .render(self.ui_state.layout.memory_info_area, buf);
    }

    fn render_ioblocks(&self, buf: &mut Buffer) {
        let lines = self
            .ioblocks
            .iter()
            .enumerate()
            .map(|(ioblock_idx, &routine_idx)| {
                Line::from(
                    vec![
                        " ".bold(),
                        if ioblock_idx == self.ui_state.ioblock_idx {
                            "> ".green() // MEM_HIGHLIGHT
                        } else {
                            "".into()
                        },
                        if routine_idx >= self.tasm.routines.len() {
                            // happens if the element is usize::MAX
                            // which happens if there are no ioblocks
                            "<No routine>".gray()
                        } else {
                            self.tasm.routines[routine_idx].ident.clone().into()
                        },
                    ]
                    .into_iter()
                    .map(|line| {
                        if ioblock_idx == self.ui_state.ioblock_idx {
                            line.bold()
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<Line<'_>>>();

        let displayed_lines = if self.ui_state.ioblock_idx < 5 {
            &lines[..]
        } else {
            &lines[(self.ui_state.ioblock_idx - 4)..]
        }
        .to_vec();

        let more_lines = match lines.len() > 5 {
            true => {
                let lines_left = lines.len() - self.ui_state.ioblock_idx - 1;
                match lines_left > 0 {
                    // use one of these: ↓⌄
                    true => Some(format!(" {lines_left} more ↓ ")),
                    false => None,
                }
            }
            false => None,
        };

        let mut pg_block = Block::bordered()
            .border_set(border::DOUBLE)
            .title(" IOBlocks ".green().into_centered_line());

        if let Some(line) = more_lines {
            pg_block = pg_block.title_bottom(line.italic());
        }

        Paragraph::new(Text::from(displayed_lines))
            .block(pg_block)
            .render(self.ui_state.layout.ioblocks_area, buf);
    }

    fn render_routines(&self, routines_area: Rect, buf: &mut Buffer) {
        let mut lines = vec![];

        for rtn in self.running_routines.iter() {
            let s = self.get_state(rtn);

            lines.push(Line::from(vec![
                " ".bold(),
                if !rtn.toggled { s.italic() } else { s.into() },
            ]))
        }
        Paragraph::new(Text::from(lines))
            .block(
                Block::bordered()
                    .border_set(border::PLAIN)
                    .title(" Active routines ".light_green().into_centered_line()),
            )
            .render(routines_area, buf);
    }

    fn get_instr_line(&self, line: usize) -> String {
        self.tasm.lines[line].trim().to_owned()
    }

    fn get_state(&self, rtn: &RunningRoutine) -> String {
        if rtn.done {
            format!("{}: done", rtn.routine.ident)
        } else {
            let waiting_ticks = rtn.waiting - 1;
            if waiting_ticks > 0 {
                format!(
                    "{}: [{}] {} (waiting {} ticks)",
                    rtn.routine.ident,
                    rtn.instr_ptr,
                    self.get_instr_line(rtn.get_line()),
                    waiting_ticks
                )
            } else {
                format!(
                    "{}: [{}] {}",
                    rtn.routine.ident,
                    rtn.instr_ptr,
                    self.get_instr_line(rtn.get_line())
                )
            }
        }
    }

    fn render_info(&self, buf: &mut Buffer) {
        Paragraph::new(Text::from(vec![
            self.tasm.fname.clone().into(),
            Line::from(if self.ui_state.unlimited_speed {
                vec![
                    "Speed: ".into(),
                    "Unlimited".bold().fg(Color::Rgb(0, 255, 255)),
                    " // ".into(),
                    format!(
                        "Running at {:.2}Hz",
                        // ticks in an interval: interval / ticks
                        self.ui_state.elapsed_ticks as f64
                            / (self.ui_state.struct_field_0xe.as_nanos() as f64 / 1_000_000_000.0)
                    )
                    .into(),
                ]
            } else {
                vec![
                    format!("Speed: {:.2}Hz // {:.2}x speed ", self.hz, self.hz / 240.0).into(),
                    match self.ui_state.lagging {
                        true => {
                            let tick_time =
                                self.ui_state.last_tick_time.as_nanos() as f64 / 1_000_000.0;
                            if tick_time > 0.01 {
                                format!(" Lag! Last tick: {tick_time:.3}ms",)
                            } else {
                                format!(" Lag! Last tick: {:.3}μs", tick_time * 1000.0)
                            }
                        }
                        .red(),
                        false => "".into(),
                    },
                ]
            }),
            format!(
                "Tick {} [{}] // {}",
                self.ticks,
                match self.paused {
                    true => "Paused",
                    false => "Running",
                },
                display_time(self.ticks as f64 / TICKS_PER_SECOND)
            )
            .into(),
        ]))
        .block(
            Block::bordered()
                .border_set(border::EMPTY)
                .title(" Info".bold()),
        )
        .render(self.ui_state.layout.info_area, buf);
    }

    fn render_ioblock_peek(&self, peek_area: Rect, buf: &mut Buffer) {
        let rtn_ident = &self.tasm.routines[self.ioblocks[self.ui_state.ioblock_idx]]
            .ident
            .as_str();
        let block = Block::bordered().border_set(border::EMPTY).title(
            format!(" {rtn_ident} instructions ")
                .red()
                .into_centered_line(),
        );
        if self.ioblocks[0] == usize::MAX {
            display_centered_message(" <No routine> ".italic(), block, peek_area, buf);
            return;
        }

        // assume that we are peeking from the start of the routine
        // maybe there can be some way to scroll in that window

        let raw_routine = &self
            .tasm
            .routine_data
            .iter()
            .find(|r| r.routine_ident.as_str() == *rtn_ident)
            .unwrap()
            .lines[..];

        Paragraph::new(Text::from(
            raw_routine
                .iter()
                .map(|(_, s)| s.as_str().into())
                .collect::<Vec<_>>(),
        ))
        .block(block)
        .render(peek_area, buf);
    }
}

fn display_centered_message<'a, L: Into<Line<'a>>>(
    message: L,
    block: Block,
    area: Rect,
    buf: &mut Buffer,
) {
    let area_split = Layout::vertical(vec![
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);

    block.render(area, buf);
    Paragraph::new(Text::from(message.into().centered())).render(area_split[1], buf);
}

fn display_time(secs: f64) -> String {
    let mut time_str = String::with_capacity(32);

    let ms = (secs * 1000.0 % 1000.0).floor() as i32;
    let seconds = secs as i32 % 60;
    let minutes = (secs / 60.0).floor() as i32 % 60;
    let hours = (secs / 3600.0).floor() as i32 % 24;
    let days = (secs / 86400.0).floor() as i32;

    if days > 0 {
        write!(time_str, "{days:0>2}:").unwrap();
    }

    if hours > 0 || days > 0 {
        write!(time_str, "{hours:0>2}:").unwrap();
    }

    write!(time_str, "{minutes:0>2}:{seconds:0>2}.{ms:0>3}").unwrap();

    time_str
}

fn render_number(line_segments: &mut Vec<Span<'_>>, num: f64, is_int: bool, colour_num_str: bool) {
    let number_width = 11usize;
    let neg = num.is_sign_negative();

    // push negative sign
    line_segments.push(if neg { "-" } else { " " }.into());

    // fill in the leading zeros with gray zeros
    let mut num_str = if is_int {
        (num as i32).abs().to_string()
    } else {
        format!("{:.2}", num.abs())
    };

    // add the number
    if num_str.len() > number_width {
        num_str.truncate(number_width);
    } else if num_str.len() < number_width {
        line_segments.push(
            "0".repeat(number_width - num_str.len())
                .fg(MEM_HIGHLIGHT_BG),
        );
    };

    line_segments.push(num_str.fg(match colour_num_str {
        false => MEM_ACCENT,
        true => MEM_HIGHLIGHT,
    }));
}
