use std::str::FromStr;

use ratatui::style::Color;

use crate::{BorderStyle, Style};

#[derive(Default, Clone)]
pub struct Theme {
    pub colors: Colors,
    pub text_styles: TextStyles,
    pub border_styles: BorderStyles,
}

impl Theme {
    pub fn material_oceanic() -> Theme {
        let colors = Colors::material_oceanic();
        Theme {
            text_styles: TextStyles::with_colors(colors.clone()),
            border_styles: BorderStyles::with_colors(colors.clone()),
            colors,
        }
    }
}

/// From https://material-theme.com/docs/reference/color-palette/
#[derive(Clone)]
pub struct Colors {
    pub background: Color,
    pub foreground: Color,
    pub text: Color,
    pub selection_background: Color,
    pub selection_foreground: Color,
    pub buttons: Color,
    pub second_background: Color,
    pub fdisabled: Color,
    pub contrast: Color,
    pub active: Color,
    pub border: Color,
    pub highlight: Color,
    pub tree: Color,
    pub notifications: Color,
    pub accent_color: Color,
    pub excluded_files_color: Color,
    pub green_color: Color,
    pub yellow_color: Color,
    pub blue_color: Color,
    pub red_color: Color,
    pub purple_color: Color,
    pub orange_color: Color,
    pub cyan_color: Color,
    pub gray_color: Color,
    pub white_black_color: Color,
    pub error_color: Color,
    pub comments_color: Color,
    pub variables_color: Color,
    pub links_color: Color,
    pub functions_color: Color,
    pub keywords_color: Color,
    pub tags_color: Color,
    pub strings_color: Color,
    pub operators_color: Color,
    pub attributes_color: Color,
    pub numbers_color: Color,
    pub parameters_color: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self::material_oceanic()
    }
}

impl Colors {
    pub fn material_oceanic() -> Colors {
        Self {
            background: Color::from_str("#263238").unwrap(),
            foreground: Color::from_str("#B0BEC5").unwrap(),
            text: Color::from_str("#607D8B").unwrap(),
            selection_background: Color::from_str("#546E7A").unwrap(),
            selection_foreground: Color::from_str("#FFFFFF").unwrap(),
            buttons: Color::from_str("#2E3C43").unwrap(),
            second_background: Color::from_str("#32424A").unwrap(),
            fdisabled: Color::from_str("#415967").unwrap(),
            contrast: Color::from_str("#1E272C").unwrap(),
            active: Color::from_str("#314549").unwrap(),
            border: Color::from_str("#2A373E").unwrap(),
            highlight: Color::from_str("#425B67").unwrap(),
            tree: Color::from_str("#6E7A70").unwrap(), // "#546E7A70" opacity is not supported
            notifications: Color::from_str("#1E272C").unwrap(),
            accent_color: Color::from_str("#009688").unwrap(),
            excluded_files_color: Color::from_str("#2E3C43").unwrap(),
            green_color: Color::from_str("#c3e88d").unwrap(),
            yellow_color: Color::from_str("#ffcb6b").unwrap(),
            blue_color: Color::from_str("#82aaff").unwrap(),
            red_color: Color::from_str("#f07178").unwrap(),
            purple_color: Color::from_str("#c792ea").unwrap(),
            orange_color: Color::from_str("#f78c6c").unwrap(),
            cyan_color: Color::from_str("#89ddff").unwrap(),
            gray_color: Color::from_str("#546e7a").unwrap(),
            white_black_color: Color::from_str("#eeffff").unwrap(),
            error_color: Color::from_str("#ff5370").unwrap(),
            comments_color: Color::from_str("#546e7a").unwrap(),
            variables_color: Color::from_str("#eeffff").unwrap(),
            links_color: Color::from_str("#80cbc4").unwrap(),
            functions_color: Color::from_str("#82aaff").unwrap(),
            keywords_color: Color::from_str("#c792ea").unwrap(),
            tags_color: Color::from_str("#f07178").unwrap(),
            strings_color: Color::from_str("#c3e88d").unwrap(),
            operators_color: Color::from_str("#89ddff").unwrap(),
            attributes_color: Color::from_str("#ffcb6b").unwrap(),
            numbers_color: Color::from_str("#f78c6c").unwrap(),
            parameters_color: Color::from_str("#f78c6c").unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct TextStyles {
    pub default: Style,
    pub hover: Style,
    pub selected: Style,
}

impl Default for TextStyles {
    fn default() -> Self {
        Self::with_colors(Colors::default())
    }
}

impl TextStyles {
    pub fn with_colors(colors: Colors) -> Self {
        let default = Style::default().bg(colors.background).fg(colors.text);
        TextStyles {
            selected: Style::default()
                .bg(colors.selection_background)
                .fg(colors.selection_foreground),
            hover: default,
            default,
        }
    }
}

#[derive(Clone)]
pub struct BorderStyles {
    pub default: BorderStyle,
    pub hover: BorderStyle,
    pub focus: BorderStyle,
}

impl BorderStyles {
    pub fn with_colors(colors: Colors) -> Self {
        let mut default = BorderStyle::default();
        default.style = default.style.fg(colors.border);

        let mut hover = BorderStyle::default();
        hover.style = hover.style.fg(colors.highlight);

        let mut focus = BorderStyle::default();
        focus.style = focus.style.fg(colors.active);

        Self {
            default,
            hover,
            focus,
        }
    }
}

impl Default for BorderStyles {
    fn default() -> Self {
        Self::with_colors(Colors::default())
    }
}
