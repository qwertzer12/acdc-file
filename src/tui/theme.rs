use ratatui::style::Color;

pub struct Theme {
    pub active_border: Color,
    pub inactive_border: Color,
    pub header_fg: Color,
    pub header_bg: Color,
    pub footer_fg: Color,
    pub text_fg: Color,
}

pub const THEME: Theme = Theme {
    active_border: Color::Blue,
    inactive_border: Color::DarkGray,
    header_fg: Color::Black,
    header_bg: Color::Blue,
    footer_fg: Color::DarkGray,
    text_fg: Color::White,
};
