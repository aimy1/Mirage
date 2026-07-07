use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub fg: Color,
    pub border: Color,
    pub accent1: Color,
    pub accent2: Color,
    pub accent3: Color,
    pub accent4: Color,
    pub accents: Vec<Color>,
}

impl Theme {
    pub fn new_tokyo_night() -> Self {
        Self {
            name: "tokyo_night",
            bg: Color::Rgb(26, 27, 38),
            fg: Color::Rgb(169, 177, 214),
            border: Color::Rgb(59, 66, 97),
            accent1: Color::Rgb(122, 162, 247), // Blue
            accent2: Color::Rgb(125, 207, 255), // Cyan
            accent3: Color::Rgb(187, 154, 247), // Purple
            accent4: Color::Rgb(247, 118, 142), // Pink
            accents: vec![
                Color::Rgb(122, 162, 247),
                Color::Rgb(125, 207, 255),
                Color::Rgb(187, 154, 247),
                Color::Rgb(247, 118, 142),
                Color::Rgb(255, 158, 100), // Orange
                Color::Rgb(158, 206, 106), // Green
            ],
        }
    }

    pub fn new_catppuccin() -> Self {
        Self {
            name: "catppuccin",
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            border: Color::Rgb(88, 91, 112),
            accent1: Color::Rgb(137, 180, 250), // Blue
            accent2: Color::Rgb(137, 220, 235), // Sky
            accent3: Color::Rgb(203, 166, 247), // Mauve
            accent4: Color::Rgb(243, 139, 168), // Red
            accents: vec![
                Color::Rgb(137, 180, 250),
                Color::Rgb(137, 220, 235),
                Color::Rgb(166, 227, 161), // Green
                Color::Rgb(249, 226, 175), // Yellow
                Color::Rgb(250, 179, 135), // Peach
                Color::Rgb(203, 166, 247),
            ],
        }
    }

    pub fn new_gruvbox() -> Self {
        Self {
            name: "gruvbox",
            bg: Color::Rgb(40, 40, 40),
            fg: Color::Rgb(235, 219, 178),
            border: Color::Rgb(102, 92, 84),
            accent1: Color::Rgb(69, 133, 136),  // Blue
            accent2: Color::Rgb(142, 192, 124), // Aqua
            accent3: Color::Rgb(215, 153, 33),  // Yellow
            accent4: Color::Rgb(204, 36, 29),   // Red
            accents: vec![
                Color::Rgb(69, 133, 136),
                Color::Rgb(142, 192, 124),
                Color::Rgb(152, 151, 26), // Green
                Color::Rgb(215, 153, 33),
                Color::Rgb(214, 93, 14),   // Orange
                Color::Rgb(177, 98, 134),  // Purple
            ],
        }
    }

    pub fn new_nord() -> Self {
        Self {
            name: "nord",
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(236, 239, 244),
            border: Color::Rgb(76, 86, 106),
            accent1: Color::Rgb(129, 161, 193), // Frost 3
            accent2: Color::Rgb(136, 192, 208), // Frost 2
            accent3: Color::Rgb(143, 188, 187), // Frost 1
            accent4: Color::Rgb(94, 129, 172),  // Frost 4
            accents: vec![
                Color::Rgb(136, 192, 208),
                Color::Rgb(129, 161, 193),
                Color::Rgb(143, 188, 187),
                Color::Rgb(94, 129, 172),
                Color::Rgb(191, 97, 106),  // Red
                Color::Rgb(208, 135, 112), // Orange
            ],
        }
    }

    pub fn new_dracula() -> Self {
        Self {
            name: "dracula",
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            border: Color::Rgb(98, 114, 164),
            accent1: Color::Rgb(139, 233, 253), // Cyan
            accent2: Color::Rgb(80, 250, 123),  // Green
            accent3: Color::Rgb(189, 147, 249), // Purple
            accent4: Color::Rgb(255, 121, 198), // Pink
            accents: vec![
                Color::Rgb(139, 233, 253),
                Color::Rgb(80, 250, 123),
                Color::Rgb(255, 184, 108), // Orange
                Color::Rgb(255, 121, 198),
                Color::Rgb(189, 147, 249),
                Color::Rgb(255, 85, 85),   // Red
            ],
        }
    }

    pub fn new_everforest() -> Self {
        Self {
            name: "everforest",
            bg: Color::Rgb(43, 51, 57),
            fg: Color::Rgb(211, 198, 170),
            border: Color::Rgb(79, 90, 97),
            accent1: Color::Rgb(167, 192, 128), // Green
            accent2: Color::Rgb(127, 187, 179), // Blue/Teal
            accent3: Color::Rgb(219, 188, 127), // Yellow
            accent4: Color::Rgb(230, 126, 128), // Red
            accents: vec![
                Color::Rgb(167, 192, 128),
                Color::Rgb(127, 187, 179),
                Color::Rgb(219, 188, 127),
                Color::Rgb(230, 152, 117), // Orange
                Color::Rgb(211, 155, 182), // Purple
                Color::Rgb(230, 126, 128),
            ],
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "catppuccin" => Self::new_catppuccin(),
            "gruvbox" => Self::new_gruvbox(),
            "nord" => Self::new_nord(),
            "dracula" => Self::new_dracula(),
            "everforest" => Self::new_everforest(),
            _ => Self::new_tokyo_night(), // default
        }
    }
}
