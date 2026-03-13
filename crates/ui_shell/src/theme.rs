//! Theme token source for the fixed-layout PhotoTux shell.

use slint::Color;
use thiserror::Error;

/// Core theme tokens that can later be mapped into Slint properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeTokens {
    /// Application background color.
    pub bg_app: &'static str,
    /// Panel background color.
    pub bg_panel: &'static str,
    /// Panel header background color.
    pub bg_panel_header: &'static str,
    /// Primary text color.
    pub text_primary: &'static str,
    /// Secondary text color.
    pub text_secondary: &'static str,
    /// Primary accent color.
    pub accent_primary: &'static str,
    /// Subtle border color.
    pub border_subtle: &'static str,
    /// Default panel padding.
    pub panel_padding: f32,
    /// Medium control height.
    pub control_height_md: f32,
    /// Toolbar width.
    pub toolbar_width: f32,
    /// Panel header height.
    pub panel_header_height: f32,
    /// Document tab height.
    pub tab_height: f32,
    /// Chrome surface color.
    pub bg_chrome: &'static str,
    /// Canvas surround color.
    pub bg_canvas_surround: &'static str,
    /// Menu surface color.
    pub bg_menu: &'static str,
    /// Muted text color.
    pub text_muted: &'static str,
    /// Default border color.
    pub border_default: &'static str,
    /// Strong border color.
    pub border_strong: &'static str,
    /// Hover button background color.
    pub button_bg_hover: &'static str,
    /// Active button background color.
    pub button_bg_active: &'static str,
    /// Compact panel row height.
    pub layer_row_height_compact: f32,
    /// Right dock target width.
    pub right_dock_width: f32,
    /// Tool options strip height.
    pub tool_options_height: f32,
    /// Status bar height.
    pub status_bar_height: f32,
    /// Titlebar height.
    pub titlebar_height: f32,
    /// Menu bar height.
    pub menu_bar_height: f32,
}

/// Theme values converted into Slint-friendly types.
#[derive(Clone, Debug, PartialEq)]
pub struct SlintTheme {
    /// Application background color.
    pub bg_app: Color,
    /// Chrome background color.
    pub bg_chrome: Color,
    /// Panel background color.
    pub bg_panel: Color,
    /// Panel header background color.
    pub bg_panel_header: Color,
    /// Canvas surround background color.
    pub bg_canvas_surround: Color,
    /// Menu background color.
    pub bg_menu: Color,
    /// Primary text color.
    pub text_primary: Color,
    /// Secondary text color.
    pub text_secondary: Color,
    /// Muted text color.
    pub text_muted: Color,
    /// Primary accent color.
    pub accent_primary: Color,
    /// Subtle border color.
    pub border_subtle: Color,
    /// Default border color.
    pub border_default: Color,
    /// Strong border color.
    pub border_strong: Color,
    /// Hover state background color.
    pub button_bg_hover: Color,
    /// Active state background color.
    pub button_bg_active: Color,
    /// Default panel padding.
    pub panel_padding: f32,
    /// Medium control height.
    pub control_height_md: f32,
    /// Toolbar width.
    pub toolbar_width: f32,
    /// Right dock width.
    pub right_dock_width: f32,
    /// Titlebar height.
    pub titlebar_height: f32,
    /// Menu bar height.
    pub menu_bar_height: f32,
    /// Tool options height.
    pub tool_options_height: f32,
    /// Panel header height.
    pub panel_header_height: f32,
    /// Document tab height.
    pub tab_height: f32,
    /// Compact row height.
    pub layer_row_height_compact: f32,
    /// Status bar height.
    pub status_bar_height: f32,
}

/// Errors returned while converting theme tokens.
#[derive(Debug, Error)]
pub enum ThemeMappingError {
    /// A color token used an unsupported format.
    #[error("unsupported color token format: {token}")]
    UnsupportedColorToken {
        /// Original token string.
        token: String,
    },
}

impl TryFrom<ThemeTokens> for SlintTheme {
    type Error = ThemeMappingError;

    fn try_from(tokens: ThemeTokens) -> Result<Self, Self::Error> {
        Ok(Self {
            bg_app: parse_hex_color(tokens.bg_app)?,
            bg_chrome: parse_hex_color(tokens.bg_chrome)?,
            bg_panel: parse_hex_color(tokens.bg_panel)?,
            bg_panel_header: parse_hex_color(tokens.bg_panel_header)?,
            bg_canvas_surround: parse_hex_color(tokens.bg_canvas_surround)?,
            bg_menu: parse_hex_color(tokens.bg_menu)?,
            text_primary: parse_hex_color(tokens.text_primary)?,
            text_secondary: parse_hex_color(tokens.text_secondary)?,
            text_muted: parse_hex_color(tokens.text_muted)?,
            accent_primary: parse_hex_color(tokens.accent_primary)?,
            border_subtle: parse_hex_color(tokens.border_subtle)?,
            border_default: parse_hex_color(tokens.border_default)?,
            border_strong: parse_hex_color(tokens.border_strong)?,
            button_bg_hover: parse_rgba_color(tokens.button_bg_hover)?,
            button_bg_active: parse_rgba_color(tokens.button_bg_active)?,
            panel_padding: tokens.panel_padding,
            control_height_md: tokens.control_height_md,
            toolbar_width: tokens.toolbar_width,
            right_dock_width: tokens.right_dock_width,
            titlebar_height: tokens.titlebar_height,
            menu_bar_height: tokens.menu_bar_height,
            tool_options_height: tokens.tool_options_height,
            panel_header_height: tokens.panel_header_height,
            tab_height: tokens.tab_height,
            layer_row_height_compact: tokens.layer_row_height_compact,
            status_bar_height: tokens.status_bar_height,
        })
    }
}

/// Dark Pro theme tokens from the design system.
pub const DARK_PRO: ThemeTokens = ThemeTokens {
    bg_app: "#1B1D21",
    bg_chrome: "#202329",
    bg_panel: "#252930",
    bg_panel_header: "#2C3139",
    bg_canvas_surround: "#14161A",
    bg_menu: "#252A31",
    text_primary: "#E8ECF3",
    text_secondary: "#B3BCC8",
    text_muted: "#8A94A3",
    accent_primary: "#4F8CFF",
    border_subtle: "#313741",
    border_default: "#3A414D",
    border_strong: "#4A5361",
    button_bg_hover: "rgba(255,255,255,0.06)",
    button_bg_active: "rgba(79,140,255,0.18)",
    panel_padding: 6.0,
    control_height_md: 28.0,
    toolbar_width: 44.0,
    right_dock_width: 312.0,
    titlebar_height: 28.0,
    menu_bar_height: 24.0,
    tool_options_height: 36.0,
    panel_header_height: 24.0,
    tab_height: 28.0,
    layer_row_height_compact: 28.0,
    status_bar_height: 20.0,
};

fn parse_hex_color(token: &str) -> Result<Color, ThemeMappingError> {
    let Some(raw) = token.strip_prefix('#') else {
        return Err(ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        });
    };

    match raw.len() {
        6 => {
            let red = u8::from_str_radix(&raw[0..2], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;
            let green = u8::from_str_radix(&raw[2..4], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;
            let blue = u8::from_str_radix(&raw[4..6], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;

            Ok(Color::from_rgb_u8(red, green, blue))
        }
        8 => {
            let red = u8::from_str_radix(&raw[0..2], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;
            let green = u8::from_str_radix(&raw[2..4], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;
            let blue = u8::from_str_radix(&raw[4..6], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;
            let alpha = u8::from_str_radix(&raw[6..8], 16).map_err(|_| {
                ThemeMappingError::UnsupportedColorToken {
                    token: token.to_string(),
                }
            })?;

            Ok(Color::from_argb_u8(alpha, red, green, blue))
        }
        _ => Err(ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        }),
    }
}

fn parse_rgba_color(token: &str) -> Result<Color, ThemeMappingError> {
    let Some(raw) = token
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    else {
        return parse_hex_color(token);
    };

    let mut parts = raw.split(',').map(str::trim);
    let red = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        })?;
    let green = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        })?;
    let blue = parts
        .next()
        .and_then(|value| value.parse::<u8>().ok())
        .ok_or_else(|| ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        })?;
    let alpha = parts
        .next()
        .and_then(|value| value.parse::<f32>().ok())
        .ok_or_else(|| ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        })?;

    if parts.next().is_some() {
        return Err(ThemeMappingError::UnsupportedColorToken {
            token: token.to_string(),
        });
    }

    let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    Ok(Color::from_argb_u8(alpha, red, green, blue))
}

#[cfg(test)]
mod tests {
    use super::{DARK_PRO, SlintTheme, ThemeMappingError, parse_hex_color, parse_rgba_color};

    #[test]
    fn maps_dark_pro_theme_tokens() {
        let theme = SlintTheme::try_from(DARK_PRO).expect("theme should map");

        assert_eq!(theme.toolbar_width, DARK_PRO.toolbar_width);
        assert_eq!(theme.right_dock_width, DARK_PRO.right_dock_width);
        assert_eq!(theme.status_bar_height, DARK_PRO.status_bar_height);
    }

    #[test]
    fn parses_hex_color_tokens() {
        let color = parse_hex_color("#4F8CFF").expect("hex color should parse");

        assert_eq!(color.red(), 0x4F);
        assert_eq!(color.green(), 0x8C);
        assert_eq!(color.blue(), 0xFF);
    }

    #[test]
    fn parses_rgba_color_tokens() {
        let color = parse_rgba_color("rgba(255,255,255,0.06)").expect("rgba color should parse");

        assert_eq!(color.red(), 255);
        assert_eq!(color.green(), 255);
        assert_eq!(color.blue(), 255);
    }

    #[test]
    fn rejects_unsupported_color_tokens() {
        let error = parse_rgba_color("not-a-color").expect_err("token should fail");

        assert!(matches!(
            error,
            ThemeMappingError::UnsupportedColorToken { .. }
        ));
    }
}
