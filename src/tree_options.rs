use std::fs;
use regex::Regex;
use std::path::PathBuf;
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Debug, Clone)]
pub struct TreeOptions {
    pub show_hidden: bool,
    pub filename_regex: Option<Regex>,
    white_list: Option<HashSet<PathBuf>>,
}

impl TreeOptions {
    pub fn new() -> TreeOptions {
        TreeOptions {
            show_hidden: false,
            filename_regex: None,
            white_list: None,
        }
    }
    pub fn set_filename_pattern(&mut self, pattern: &str) {
        self.filename_regex = None;
        if pattern.len() > 0 {
            if let Ok(regex) = Regex::new(pattern) {
                self.filename_regex = Some(regex);
            }
        }
    }
    pub fn accepts(&self, path: &PathBuf) -> bool {
        match &self.white_list {
            Some(matches) => matches.contains(path),
            None => {
                // FIXME what's the proper way to check whether a file is hidden ?
                if let Some(filename) = path.file_name() {
                    let first_char = filename.to_string_lossy().chars().next();
                    if let Some('.') = first_char {
                        return false;
                    }
                }
                true
            }
        }
    }
    pub fn prepare_for_root(&mut self, root: &PathBuf) {
        self.white_list = match &self.filename_regex {
            None => None,
            Some(regex) => Some(HashSet::from_iter(self.all_matches(root)))
        }
    }
    // returns the number of matches (which is usually smaller than the size of the
    //  vector which also contains parents even if not directly matching)
    fn find_matches(
        &self,
        candidate: &PathBuf,
        matches: &mut Vec<PathBuf>,
    ) -> u32 {
        let filename = match candidate.file_name() {
            Some(filename) => filename.to_string_lossy(),
            None => { return 0; },
        };
        if !self.show_hidden {
            let first_char = filename.chars().next();
            if let Some('.') = first_char {
                return 0; // we don't look for matches of hidden dirs either
            }
        }
        let metadata = match fs::metadata(&candidate) {
            Ok(metadata) => metadata,
            _ => { return 0; },
        };
        let mut matches_count = 0;
        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(&candidate) {
                for e in entries {
                    if let Ok(e) = e {
                        let path = e.path();
                        matches_count += self.find_matches(
                            &path,
                            matches
                        );
                    }
                }
            }
        }
        match &self.filename_regex {
            Some(regex) => {
                if regex.is_match(&filename) {
                    matches_count += 1;
                }
            }
            None => { // we should probably not do a DFS search, to start with...
                matches_count += 1;
            }
        }
        if matches_count > 0 {
            matches.push(candidate.clone());
        }
        matches_count
    }
    pub fn all_matches(&self, root: &PathBuf) -> Vec<PathBuf> {
        let mut matches: Vec<PathBuf> = Vec::new();
        let n = self.find_matches(root, &mut matches);
        println!("{} matches found", n);
        matches
    }

}
