use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum ThemeName {
    Default,
    Dracula,
    Nord,
    Solarized,
    Gruvbox,
}

impl Default for ThemeName {
    fn default() -> Self {
        ThemeName::Default
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Palette {
    pub header_fg: Color,
    pub header_bg: Color,
    pub accent: Color,
    pub muted: Color,
    pub focus_border: Color,
    pub blur_border: Color,
    pub highlight: Color,
    pub tab_active: Color,
    pub tab_inactive: Color,
}

impl ThemeName {
    pub(super) fn label(self) -> &'static str {
        match self {
            ThemeName::Default => "Default",
            ThemeName::Dracula => "Dracula",
            ThemeName::Nord => "Nord",
            ThemeName::Solarized => "Solarized",
            ThemeName::Gruvbox => "Gruvbox",
        }
    }

    pub(super) const ALL: [ThemeName; 5] = [
        ThemeName::Default,
        ThemeName::Dracula,
        ThemeName::Nord,
        ThemeName::Solarized,
        ThemeName::Gruvbox,
    ];

    pub(super) fn palette(self) -> Palette {
        match self {
            ThemeName::Default => Palette {
                header_fg: Color::White,
                header_bg: Color::Blue,
                accent: Color::Cyan,
                muted: Color::DarkGray,
                focus_border: Color::Cyan,
                blur_border: Color::DarkGray,
                highlight: Color::DarkGray,
                tab_active: Color::Yellow,
                tab_inactive: Color::DarkGray,
            },
            ThemeName::Dracula => Palette {
                header_fg: Color::Rgb(248, 248, 242),
                header_bg: Color::Rgb(98, 114, 164),
                accent: Color::Rgb(189, 147, 249),
                muted: Color::Rgb(98, 114, 164),
                focus_border: Color::Rgb(255, 121, 198),
                blur_border: Color::Rgb(68, 71, 90),
                highlight: Color::Rgb(68, 71, 90),
                tab_active: Color::Rgb(255, 184, 108),
                tab_inactive: Color::Rgb(98, 114, 164),
            },
            ThemeName::Nord => Palette {
                header_fg: Color::Rgb(236, 239, 244),
                header_bg: Color::Rgb(59, 66, 82),
                accent: Color::Rgb(136, 192, 208),
                muted: Color::Rgb(76, 86, 106),
                focus_border: Color::Rgb(143, 188, 187),
                blur_border: Color::Rgb(76, 86, 106),
                highlight: Color::Rgb(67, 76, 94),
                tab_active: Color::Rgb(235, 203, 139),
                tab_inactive: Color::Rgb(76, 86, 106),
            },
            ThemeName::Solarized => Palette {
                header_fg: Color::Rgb(253, 246, 227),
                header_bg: Color::Rgb(7, 54, 66),
                accent: Color::Rgb(38, 139, 210),
                muted: Color::Rgb(101, 123, 131),
                focus_border: Color::Rgb(42, 161, 152),
                blur_border: Color::Rgb(88, 110, 117),
                highlight: Color::Rgb(7, 54, 66),
                tab_active: Color::Rgb(181, 137, 0),
                tab_inactive: Color::Rgb(101, 123, 131),
            },
            ThemeName::Gruvbox => Palette {
                header_fg: Color::Rgb(235, 219, 178),
                header_bg: Color::Rgb(60, 56, 54),
                accent: Color::Rgb(131, 165, 152),
                muted: Color::Rgb(146, 131, 116),
                focus_border: Color::Rgb(184, 187, 38),
                blur_border: Color::Rgb(80, 73, 69),
                highlight: Color::Rgb(80, 73, 69),
                tab_active: Color::Rgb(250, 189, 47),
                tab_inactive: Color::Rgb(146, 131, 116),
            },
        }
    }
}
