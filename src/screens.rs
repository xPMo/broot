use crate::skin::Skin;
use std::io::{self, stderr, Write};
use termion::color;
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;

pub struct Screen {
    pub w: u16,
    pub h: u16,
    pub stderr: AlternateScreen<RawTerminal<io::Stderr>>,
    pub skin: Skin,
}

#[derive(Debug)]
pub struct ScreenArea {
    pub top: u16,    // first line
    pub bottom: u16, // last line, included
    pub scroll: i32, // 0 for no scroll, positive if scrolled
    pub content_length: i32,
    pub width: u16,
}

impl Screen {
    pub fn new(skin: Skin) -> io::Result<Screen> {
        let stderr = AlternateScreen::from(stderr().into_raw_mode()?);
        let mut screen = Screen {
            w: 0,
            h: 0,
            stderr,
            skin,
        };
        screen.read_size()?;
        write!(screen.stderr, "{}", termion::cursor::Hide)?;
        Ok(screen)
    }
    pub fn read_size(&mut self) -> io::Result<()> {
        let (w, h) = termion::terminal_size()?;
        self.w = w;
        self.h = h;
        Ok(())
    }
    pub fn reset_colors(&mut self) -> io::Result<()> {
        write!(
            self.stderr,
            "{}{}",
            color::Fg(color::Reset),
            color::Bg(color::Reset),
        )
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        write!(self.stderr, "{}", termion::cursor::Show).unwrap();
        // if we don't flush now, the standard screen may receive some
        // unflushed data which was meant for the alternate screen.
        self.stderr.flush().unwrap();
    }
}

impl ScreenArea {
    pub fn new(top: u16, bottom: u16, width: u16) -> ScreenArea {
        ScreenArea {
            top,
            bottom,
            scroll: 0,
            content_length: 0,
            width,
        }
    }
    pub fn try_scroll(&mut self, dy: i32) {
        self.scroll = (self.scroll + dy)
            .max(0)
            .min(self.content_length - self.height() + 1);
    }
    pub fn scrollbar(&self) -> Option<(u16, u16)> {
        let h = self.height();
        if self.content_length <= h {
            return None;
        }
        let sbh = h * h / self.content_length;
        let sc = i32::from(self.top) + self.scroll * h / self.content_length;
        Some((sc as u16, (sc + sbh - 1).min(i32::from(self.bottom)) as u16))
    }
    pub fn height(&self) -> i32 {
        i32::from(self.bottom - self.top) + 1
    }
}
