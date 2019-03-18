/// displays the "input" at the bottom of the screen
/// (reading is managed in the app module)
use std::io::{self, Write};

use crate::commands::Command;
use crate::screens::Screen;

pub trait Input {
    fn write_input(&mut self, cmd: &Command) -> io::Result<()>;
}

impl Input for Screen {
    fn write_input(&mut self, cmd: &Command) -> io::Result<()> {
        write!(
            self.stderr,
            "{}{}{}{}{}{} {}",
            termion::cursor::Goto(1, self.h),
            self.skin.input.fg,
            self.skin.input.bg,
            termion::clear::CurrentLine,
            cmd.raw,
            termion::style::Invert,
            termion::style::NoInvert,
        )?;
        self.stderr.flush()?;
        Ok(())
    }
}
