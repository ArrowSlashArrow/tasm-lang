use ratatui::layout::{Constraint, Layout, Rect};

pub const KEY_AREA_HEIGHT: u16 = 15;

#[derive(Debug, Default)]
pub struct PrecomputedLayout {
    pub logbox_area: Rect,
    pub display_area: Rect,
    pub ioblocks_area: Rect,
    pub info_area: Rect,
    pub routines_area: Rect,
    pub memory_cells_area: Rect,
    pub memory_info_area: Rect,
    pub keys_area: Rect,
    pub is_dirty: bool,
}

impl PrecomputedLayout {
    pub fn new() -> Self {
        Self {
            is_dirty: true,
            ..Default::default()
        }
    }

    pub fn compute(&mut self, area: Rect) {
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

        let htopleft_layout = Layout::horizontal(vec![
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(32),
        ])
        .split(vleft_layout[0]);

        let vbottomleft_layout = Layout::vertical(vec![Constraint::Length(7), Constraint::Min(1)])
            .split(vleft_layout[1]);

        let hbottomleft_layout =
            Layout::horizontal(vec![Constraint::Length(32), Constraint::Min(1)])
                .split(vbottomleft_layout[0]);

        let vright_layout = Layout::vertical(vec![
            Constraint::Percentage(50), // aligned to be in the same vertical area as logs and items; looks better
            Constraint::Min(1),
            Constraint::Length(KEY_AREA_HEIGHT),
        ])
        .split(h_layout[1]);

        self.logbox_area = htopleft_layout[0];
        self.display_area = htopleft_layout[2];
        self.ioblocks_area = hbottomleft_layout[0];
        self.info_area = hbottomleft_layout[1];
        self.routines_area = vbottomleft_layout[1];
        self.memory_cells_area = vright_layout[0];
        self.memory_info_area = vright_layout[1];
        self.keys_area = vright_layout[2];
        self.is_dirty = false;
    }
}
