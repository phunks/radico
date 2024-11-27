use anyhow::{Error, Result};
#[allow(unused_imports)]
use crossterm::terminal::{
    enable_raw_mode, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{cursor, execute};
use inquire::ui::{Attributes, Color, RenderConfig, StyleSheet, Styled};
use inquire::Select;
use std::io;

pub fn show(v: &[String]) -> Result<String> {
    inquire::set_global_render_config(render_config());

    let mut stdout = io::stdout();

    enable_raw_mode().unwrap();
    execute!(
        stdout,
        // EnterAlternateScreen,
        // Clear(ClearType::All),
        cursor::MoveTo(0, 1),
        cursor::Hide
    )?;

    let station = match Select::new("station?", v.to_vec()).prompt() {
        Ok(station) => station,
        Err(e) => return Err(Error::from(e)),
    };

    execute!(
        stdout,
        // LeaveAlternateScreen,
        cursor::Show
    )?;

    Ok(station)
}

pub fn render_config() -> RenderConfig<'static> {
    RenderConfig {
        help_message: StyleSheet::new() // help message
            .with_fg(Color::rgb(150, 150, 140)),
        prompt_prefix: Styled::new("?") // question prompt
            .with_fg(Color::rgb(150, 150, 140)),
        highlighted_option_prefix: Styled::new(">") // cursor
            .with_fg(Color::rgb(150, 250, 40)),
        selected_option: Some(
            StyleSheet::new() // focus
                .with_fg(Color::rgb(250, 180, 40)),
        ),
        answer: StyleSheet::new()
            .with_attr(Attributes::ITALIC)
            .with_attr(Attributes::BOLD)
            .with_fg(Color::rgb(220, 220, 240)),
        ..Default::default()
    }
}
