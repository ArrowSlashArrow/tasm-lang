use ratatui::{
    layout::{Constraint, Layout, Spacing},
    prelude::{Buffer, Rect},
    style::{Color, Stylize},
    symbols::{border, merge::MergeStrategy},
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use std::fmt::Write;

use crate::{
    core::structs::MemType,
    debugger::{Emulator, RunningRoutine, TICKS_PER_SECOND},
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
];

const KEY_AREA_HEIGHT: u16 = 15;

impl Widget for &Emulator {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // outline
        Block::bordered()
            .border_set(border::ROUNDED)
            .render(area, buf);

        let middle_h_temp = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(area);

        let workable_area = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(middle_h_temp[1])[1];

        /* setup areas */
        let h_layout = Layout::horizontal(vec![Constraint::Percentage(50), Constraint::Min(1)])
            .split(workable_area);

        let vleft_layout =
            Layout::vertical(vec![Constraint::Min(1), Constraint::Min(1)]).split(h_layout[0]);

        let htopleft_layout = Layout::horizontal(vec![Constraint::Min(1), Constraint::Length(32)])
            .split(vleft_layout[0]);

        let vbottomleft_layout = Layout::vertical(vec![Constraint::Length(7), Constraint::Min(1)])
            .split(vleft_layout[1]);

        let hbottomleft_layout =
            Layout::horizontal(vec![Constraint::Length(32), Constraint::Min(1)])
                .split(vbottomleft_layout[0]);

        let vright_layout = Layout::vertical(vec![
            Constraint::Min(1), // note to future self: we don't need all that space just for memory
            Constraint::Length(KEY_AREA_HEIGHT),
        ])
        .split(h_layout[1]);

        let logbox_area = htopleft_layout[0];
        let display_area = htopleft_layout[1];
        let ioblocks_area = hbottomleft_layout[0];
        let info_area = hbottomleft_layout[1];
        let routines_area = vbottomleft_layout[1];
        let memory_area = vright_layout[0];
        let keys_area = vright_layout[1];

        self.render_logbox(logbox_area, buf);
        self.render_displays(display_area, buf);
        self.render_keys(keys_area, buf);
        self.render_memory(memory_area, buf);
        self.render_ioblocks(ioblocks_area, buf);
        self.render_info(info_area, buf);

        if self.peeking_ioblock {
            let bottom_left_layout =
                Layout::horizontal(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(routines_area);
            self.render_routines(bottom_left_layout[0], buf);
            self.render_ioblock_peek(bottom_left_layout[1], buf);
        } else {
            self.render_routines(routines_area, buf);
        }
    }
}

impl Emulator {
    fn render_logbox(&self, logbox_area: Rect, buf: &mut Buffer) {
        let logbox_height = (logbox_area.height - 2) as usize;
        let logs = if self.logbox.len() > logbox_height {
            &self.logbox[(&self.logbox.len() - logbox_height)..]
        } else {
            &self.logbox[..]
        };

        // -2: 1 char padding on each side
        let max_log_length = (logbox_area.width - 2) as usize;

        Paragraph::new(Text::from(
            logs.iter()
                .map(|reflog| {
                    let mut log = reflog.clone();
                    let truncated;
                    if log.len() > max_log_length {
                        log.truncate(max_log_length - 5);
                        truncated = true;
                    } else {
                        truncated = false;
                    }
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
        .render(logbox_area, buf);
    }

    fn render_displays(&self, display_area: Rect, buf: &mut Buffer) {
        let displays_height = display_area.height as usize;
        let displays = if self.displays.len() > displays_height {
            &self.displays[..displays_height]
        } else {
            &self.displays[..]
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
        .render(display_area, buf);
    }

    fn render_keys(&self, keys_area: Rect, buf: &mut Buffer) {
        Block::bordered()
            .border_set(border::EMPTY)
            .title(" Keys".bold())
            .render(keys_area, buf);

        let middle_h_temp = Layout::horizontal(vec![
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(keys_area);

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

        if curr_column.len() > 0 {
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

    fn render_memory(&self, memory_area: Rect, buf: &mut Buffer) {
        if let None = self.tasm.mem_info {
            display_centered_message(
                "<No memory>",
                Block::bordered()
                    .border_set(border::PLAIN)
                    .title(" Memory cells ".blue().into_centered_line()),
                memory_area,
                buf,
            );
            return;
        }

        // sections: title, body, metadata/info
        let split = Layout::vertical(vec![
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(7),
        ])
        .split(memory_area);

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
        let col_width = 18.0 + f64::log10(mem.size as f64);
        let max_cols = ((cols_area.width - 1) as f64 / col_width) as u16;
        let col_height = u16::min(
            cols_area.height - 2,
            (mem.size as u16 + max_cols) / max_cols,
        );

        cols_area.height = col_height;

        let cols_division = Layout::horizontal(
            std::iter::repeat(Constraint::Fill(col_width as u16 + 2)).take(max_cols as usize),
        )
        .spacing(Spacing::Overlap(1))
        .split(cols_area);

        let highlight = Color::Rgb(255, 115, 15);
        let highlight_bg = Color::Rgb(60, 60, 60);
        let accent = Color::Rgb(255, 230, 115);

        // now, build all of the lines for each column
        let number_width = 11usize;
        let mut cols = vec![];
        let mut idx = 0;

        while cols.len() < max_cols as usize && idx < mem.size {
            let mut curr_col = vec![];
            while curr_col.len() < col_height as usize && idx < mem.size {
                let num = self.read_mem(idx);
                let neg = num.is_sign_negative();

                let is_selected = self.get_ptrpos_value().0 == idx as i32;

                let mut line_segments = vec![
                    " ".into(),
                    if is_selected {
                        "  >".fg(highlight)
                    } else {
                        format!("{idx:0>3}").fg(accent)
                    },
                    if is_selected {
                        "  ".into()
                    } else {
                        ": ".into()
                    },
                    if neg { "-" } else { " " }.into(),
                ];

                // fill in the leading zeros with gray zeros
                let mut num_str = if mem._type == MemType::Int || mem._type == MemType::LegacyInt {
                    (num as i32).abs().to_string()
                } else {
                    format!("{:.2}", num.abs())
                };

                if num_str.len() > number_width {
                    num_str.truncate(number_width);
                } else if num_str.len() < number_width {
                    line_segments.push("0".repeat(number_width - num_str.len()).fg(highlight_bg));
                }

                line_segments.push(num_str.fg(match is_selected {
                    false => accent,
                    true => highlight,
                }));

                line_segments.push(" ".into());

                let mut line = Line::from(line_segments).centered();
                if is_selected {
                    line = line.bg(highlight_bg)
                }

                curr_col.push(line);
                idx += 1;
            }

            cols.push(curr_col);
        }

        for (i, col) in cols.into_iter().enumerate() {
            Paragraph::new(Text::from(col))
                .block(Block::bordered().merge_borders(MergeStrategy::Exact))
                .render(cols_division[i], buf);
        }

        // todo: render memory metadata
    }

    fn render_ioblocks(&self, ioblocks_area: Rect, buf: &mut Buffer) {
        let lines = self
            .ioblocks
            .iter()
            .enumerate()
            .map(|(ioblock_idx, &routine_idx)| {
                Line::from(
                    vec![
                        " ".bold(),
                        if ioblock_idx == self.ioblock_idx {
                            "> ".green() // highlight
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
                        if ioblock_idx == self.ioblock_idx {
                            line.bold()
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<Line<'_>>>();

        let displayed_lines = if self.ioblock_idx < 5 {
            &lines[..]
        } else {
            &lines[(self.ioblock_idx - 4)..]
        }
        .to_vec();

        let more_lines = match lines.len() > 5 {
            true => {
                let lines_left = lines.len() - self.ioblock_idx - 1;
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
            .render(ioblocks_area, buf);
    }

    fn render_routines(&self, routines_area: Rect, buf: &mut Buffer) {
        Paragraph::new(Text::from(
            self.running_routines
                .iter()
                .map(|rtn| Line::from(vec![" ".bold(), self.get_state(rtn).into()]))
                .collect::<Vec<Line<'_>>>(),
        ))
        .block(
            Block::bordered()
                .border_set(border::PLAIN)
                .title(" Active routines ".magenta().into_centered_line()),
        )
        .render(routines_area, buf);
    }

    fn get_instr_line(&self, line: usize) -> String {
        self.tasm.lines[line].trim().to_owned()
    }

    fn get_state(&self, rtn: &RunningRoutine) -> String {
        if rtn.done {
            return format!("{}: done", rtn.routine.ident);
        }
        if rtn.waiting > 0 {
            return format!(
                "{}: [{}] {} (waiting {} ticks)",
                rtn.routine.ident,
                rtn.instr_ptr,
                self.get_instr_line(rtn.get_line()),
                rtn.waiting
            );
        } else {
            return format!(
                "{}: [{}] {}",
                rtn.routine.ident,
                rtn.instr_ptr,
                self.get_instr_line(rtn.get_line())
            );
        }
    }

    fn render_info(&self, info_area: Rect, buf: &mut Buffer) {
        Paragraph::new(Text::from(vec![
            self.tasm.fname.clone().into(),
            Line::from(vec![
                format!("Speed: {:.2}Hz // {:.2}x speed ", self.hz, self.hz / 240.0).into(),
                match self.lagging {
                    true => format!(
                        " Lag! Last tick: {:.3}ms",
                        self.last_tick_time.as_nanos() as f64 / 1_000_000.0 // scale to ms with dp precision
                    )
                    .red(),
                    false => "".into(),
                },
            ]),
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
            format!(
                "Memory: {}",
                match &self.tasm.mem_info {
                    Some(m) => format!(
                        "[{:?}] {} - {}",
                        m._type,
                        m.start_counter_id,
                        m.start_counter_id + m.size
                    ),
                    None => "No memory".into(),
                }
            )
            .into(),
        ]))
        .block(
            Block::bordered()
                .border_set(border::EMPTY)
                .title(" Info".bold()),
        )
        .render(info_area, buf);
    }

    fn render_ioblock_peek(&self, peek_area: Rect, buf: &mut Buffer) {
        let rtn_ident = &self.tasm.routines[self.ioblocks[self.ioblock_idx]]
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
