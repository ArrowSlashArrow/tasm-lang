use ratatui::{
    layout::{Constraint, Layout},
    prelude::{Buffer, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

use crate::debugger::{Emulator, RunningRoutine};

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
];

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

        let vright_layout =
            Layout::vertical(vec![Constraint::Min(1), Constraint::Length(12)]).split(h_layout[1]);

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
        self.render_routines(routines_area, buf);
        self.render_info(info_area, buf);
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
                        self.state.get_item_value(*item)
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
                    "<".blue(),
                    key.blue().bold(),
                    "> ".blue(),
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
            if curr_column.len() == 10 {
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
        Block::bordered()
            .border_set(border::PLAIN)
            .title(" Memory cells ".blue().into_centered_line())
            .render(memory_area, buf);
        // todeo
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
                        // todo: fix bolding not working
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
        // todo:
        // render ticks, puased, mem info, file, group usage
        Paragraph::new(Text::from(vec![
            self.tasm.fname.clone().into(),
            format!(
                "Tick {} [{}]",
                self.ticks,
                match self.paused {
                    true => "Paused",
                    false => "Running...",
                }
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
}
