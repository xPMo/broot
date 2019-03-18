use std::borrow::Cow;
use std::io::{self, Write};
use std::sync::Mutex;
use termion::style;
use users::{Groups, Users, UsersCache};

use crate::file_sizes::Size;
use crate::flat_tree::{LineType, Tree, TreeLine};
use crate::patterns::Pattern;
use crate::screens::{Screen, ScreenArea};

pub trait TreeView {
    fn write_tree(&mut self, tree: &Tree) -> io::Result<()>;
    fn write_line_size(&mut self, line: &TreeLine, total_size: Size) -> io::Result<()>;
    fn write_mode(&mut self, mode: u32) -> io::Result<()>;
    fn write_line_name(&mut self, line: &TreeLine, idx: usize, pattern: &Pattern)
        -> io::Result<()>;
}

impl TreeView for Screen {
    fn write_tree(&mut self, tree: &Tree) -> io::Result<()> {
        lazy_static! {
            static ref USERS_CACHE_MUTEX: Mutex<UsersCache> = Mutex::new(UsersCache::new());
        }
        let users_cache = USERS_CACHE_MUTEX.lock().unwrap();
        let mut max_user_name_len = 0;
        let mut max_group_name_len = 0;
        if tree.options.show_permissions {
            // we compute the max size of user/group names to reserve width for the columns
            for i in 1..tree.lines.len() {
                let line = &tree.lines[i];
                if let Some(user) = users_cache.get_user_by_uid(line.uid) {
                    max_user_name_len = max_user_name_len.max(user.name().to_string_lossy().len());
                }
                if let Some(group) = users_cache.get_group_by_gid(line.uid) {
                    max_group_name_len =
                        max_group_name_len.max(group.name().to_string_lossy().len());
                }
            }
        }
        let total_size = tree.total_size();
        let area = ScreenArea {
            top: 1,
            bottom: self.h - 1,
            scroll: tree.scroll,
            content_length: tree.lines.len() as i32,
            width: self.w,
        };
        let scrollbar = area.scrollbar();
        for y in 1..self.h - 1 {
            write!(self.stderr, "{}", termion::cursor::Goto(1, y),)?;
            let mut line_index = (y - 1) as usize;
            if line_index > 0 {
                line_index += tree.scroll as usize;
            }
            if line_index < tree.lines.len() {
                let line = &tree.lines[line_index];
                //self.apply_skin_entry(&self.skin.tree)?;
                write!(self.stderr, "{}", self.skin.tree.fgbg())?;
                for depth in 0..line.depth {
                    write!(
                        self.stderr,
                        "{}",
                        if line.left_branchs[depth as usize] {
                            if tree.has_branch(line_index + 1, depth as usize) {
                                if depth == line.depth - 1 {
                                    "├──"
                                } else {
                                    "│  "
                                }
                            } else {
                                "└──"
                            }
                        } else {
                            "   "
                        },
                    )?;
                }
                if tree.options.show_sizes && line_index > 0 {
                    self.write_line_size(line, total_size)?;
                }
                if tree.options.show_permissions && line_index > 0 {
                    if line.is_selectable() {
                        self.write_mode(line.mode)?;
                        if let Some(user) = users_cache.get_user_by_uid(line.uid) {
                            write!(
                                self.stderr,
                                " {:w$}",
                                user.name().to_string_lossy(),
                                w = max_user_name_len,
                            )?;
                        }
                        if let Some(group) = users_cache.get_group_by_gid(line.uid) {
                            write!(
                                self.stderr,
                                " {:w$} ",
                                group.name().to_string_lossy(),
                                w = max_group_name_len,
                            )?;
                        }
                    } else {
                        write!(
                            self.stderr,
                            "{}──────────────{}",
                            self.skin.tree.fg, self.skin.reset.fg,
                        )?;
                    }
                }
                let selected = line_index == tree.selection;
                if selected {
                    write!(self.stderr, "{}", self.skin.selected_line.bg)?;
                }
                self.write_line_name(line, line_index, &tree.options.pattern)?;
            }
            write!(
                self.stderr,
                "{}{}",
                termion::clear::UntilNewline,
                style::Reset,
            )?;
            if let Some((sctop, scbottom)) = scrollbar {
                if sctop <= y && y <= scbottom {
                    write!(self.stderr, "{}▐", termion::cursor::Goto(self.w, y),)?;
                }
            }
        }
        self.stderr.flush()?;
        Ok(())
    }

    fn write_mode(&mut self, mode: u32) -> io::Result<()> {
        write!(
            self.stderr,
            "{} {}{}{}{}{}{}{}{}{}",
            self.skin.permissions.fg,
            if (mode & (1 << 8)) != 0 { 'r' } else { '-' },
            if (mode & (1 << 7)) != 0 { 'w' } else { '-' },
            if (mode & (1 << 6)) != 0 { 'x' } else { '-' },
            if (mode & (1 << 5)) != 0 { 'r' } else { '-' },
            if (mode & (1 << 4)) != 0 { 'w' } else { '-' },
            if (mode & (1 << 3)) != 0 { 'x' } else { '-' },
            if (mode & (1 << 2)) != 0 { 'r' } else { '-' },
            if (mode & (1 << 1)) != 0 { 'w' } else { '-' },
            if (mode & 1) != 0 { 'x' } else { '-' },
        )
    }

    fn write_line_size(&mut self, line: &TreeLine, total_size: Size) -> io::Result<()> {
        if let Some(s) = line.size {
            let dr: usize = s.discrete_ratio(total_size, 8) as usize;
            let s: Vec<char> = s.to_string().chars().collect();
            write!(
                self.stderr,
                "{}{}",
                self.skin.size_text.fg, self.skin.size_bar_full.bg,
            )?;
            for i in 0..dr {
                write!(self.stderr, "{}", if i < s.len() { s[i] } else { ' ' })?;
            }
            write!(self.stderr, "{}", self.skin.size_bar_void.bg)?;
            for i in dr..8 {
                write!(self.stderr, "{}", if i < s.len() { s[i] } else { ' ' })?;
            }
            write!(self.stderr, "{}{} ", self.skin.reset.fg, self.skin.reset.bg,)
        } else {
            write!(
                self.stderr,
                "{}────────{} ",
                self.skin.tree.fg, self.skin.reset.fg,
            )
        }
    }

    fn write_line_name(
        &mut self,
        line: &TreeLine,
        idx: usize,
        pattern: &Pattern,
    ) -> io::Result<()> {
        // TODO draw in red lines with has_error
        match &line.line_type {
            LineType::Dir => {
                if idx == 0 {
                    write!(
                        self.stderr,
                        "{}{}{}",
                        style::Bold,
                        &self.skin.directory.fg,
                        &line.path.to_string_lossy(),
                    )?;
                } else {
                    write!(
                        self.stderr,
                        "{}{}{}",
                        style::Bold,
                        &self.skin.directory.fg,
                        decorated_name(
                            &line.name,
                            pattern,
                            &self.skin.char_match.fg,
                            &self.skin.directory.fg
                        ),
                    )?;
                    if line.unlisted > 0 {
                        write!(self.stderr, " …",)?;
                    }
                }
            }
            LineType::File => {
                if line.is_exe() {
                    write!(
                        self.stderr,
                        "{}{}",
                        &self.skin.exe.fg,
                        decorated_name(
                            &line.name,
                            pattern,
                            &self.skin.char_match.fg,
                            &self.skin.exe.fg
                        ),
                    )?;
                } else {
                    write!(
                        self.stderr,
                        "{}{}",
                        &self.skin.file.fg,
                        decorated_name(
                            &line.name,
                            pattern,
                            &self.skin.char_match.fg,
                            &self.skin.file.fg
                        ),
                    )?;
                }
            }
            LineType::SymLinkToFile(target) => {
                write!(
                    self.stderr,
                    "{}{} {}->{} {}",
                    &self.skin.link.fg,
                    decorated_name(
                        &line.name,
                        pattern,
                        &self.skin.char_match.fg,
                        &self.skin.link.fg
                    ),
                    if line.has_error {
                        &self.skin.file_error.fg
                    } else {
                        &self.skin.link.fg
                    },
                    &self.skin.file.fg,
                    &target,
                )?;
            }
            LineType::SymLinkToDir(target) => {
                write!(
                    self.stderr,
                    "{}{} {}->{}{} {}",
                    &self.skin.link.fg,
                    decorated_name(
                        &line.name,
                        pattern,
                        &self.skin.char_match.fg,
                        &self.skin.link.fg
                    ),
                    if line.has_error {
                        &self.skin.file_error.fg
                    } else {
                        &self.skin.link.fg
                    },
                    style::Bold,
                    &self.skin.directory.fg,
                    &target,
                )?;
            }
            LineType::Pruning => {
                write!(
                    self.stderr,
                    //"{}{}… {} unlisted", still not sure whether I want this '…'
                    "{}{}{} unlisted",
                    self.skin.unlisted.fg,
                    style::Italic,
                    &line.unlisted,
                )?;
            }
        }
        Ok(())
    }
}

fn decorated_name<'a>(
    name: &'a str,
    pattern: &Pattern,
    prefix: &str,
    postfix: &str,
) -> Cow<'a, str> {
    if pattern.is_some() {
        if let Some(m) = pattern.find(name) {
            return Cow::Owned(m.wrap_matching_chars(name, prefix, postfix));
        }
    }
    Cow::Borrowed(name)
}
