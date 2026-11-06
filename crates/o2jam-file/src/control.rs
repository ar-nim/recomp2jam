//! Interface control lists: `ControlList_*.txt` blocks, entries,
//! and frame-rect consumption.

use super::{unknown_format, FileError};

/// One bound control entry: `IDC_NAME 0XCODE ["sprite"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlEntry {
    /// Control id (`IDC_...`).
    pub id: String,
    /// Control code (`0X...`, e.g. `0X84200003`).
    pub code: u32,
    /// Linked sprite file, if the entry names one.
    pub sprite: Option<String>,
}

/// One `SET ... { ... }` subgroup inside a block (frame lists,
/// number sprites, …). Its entries consume `.bnd` rects in file
/// order exactly like top-level entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSet {
    /// Numbers on the `SET` line (e.g. entry count, flags).
    pub params: Vec<u32>,
    /// Member entries in file order.
    pub entries: Vec<ControlEntry>,
}

/// One `STATE_`/`DIALOG_` block of a `ControlList_*.txt` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlBlock {
    /// Block name (`STATE_ROOM`, `DIALOG_EVENT`, …).
    pub name: String,
    /// Declared entry count (first number on the header line).
    pub number: u32,
    /// Extra numbers on the header line, if any.
    pub extra: Vec<u32>,
    /// Sprite-bind file from the `BOUND "..."` line, if present.
    pub bound_file: Option<String>,
    /// Entries before the first `SET` group, in file order
    /// (entry 0 is the block itself).
    pub bound: Vec<ControlEntry>,
    /// `SET` subgroups in file order.
    pub sets: Vec<ControlSet>,
}

impl ControlBlock {
    /// All entries in file order (`bound`, then each set's
    /// entries): `.bnd` record `i` lays out entry `i + 1`.
    pub fn all_entries(&self) -> Vec<&ControlEntry> {
        let mut out = Vec::with_capacity(
            self.bound.len() + self.sets.iter().map(|s| s.entries.len()).sum::<usize>(),
        );
        out.extend(self.bound.iter());
        for set in &self.sets {
            out.extend(set.entries.iter());
        }
        out
    }
}

/// A parsed `ControlList_*.txt` UI definition: state/dialog blocks
/// binding control ids to codes, sprites, and (via the `.bnd` file
/// each block names) screen rects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlList {
    /// Declared `NUMBER_OF_STATE` value, if present.
    pub num_states: Option<u32>,
    /// Declared `NUMBER_OF_DIALOG` value, if present.
    pub num_dialogs: Option<u32>,
    /// `STATE_`/`DIALOG_` blocks in file order (stub blocks with
    /// no body parse to empty entry lists).
    pub blocks: Vec<ControlBlock>,
}

fn parse_hex_word(word: &str) -> Result<u32, FileError> {
    let digits = word
        .strip_prefix("0x")
        .or_else(|| word.strip_prefix("0X"))
        .ok_or_else(|| unknown_format(word.as_bytes()))?;
    u32::from_str_radix(digits, 16).map_err(|_| unknown_format(word.as_bytes()))
}

/// First `"quoted"` value on a line, if any.
fn quoted_value(line: &str) -> Option<String> {
    let open = line.find('"')?;
    let close = line[open + 1..].find('"')?;
    Some(line[open + 1..open + 1 + close].to_string())
}

fn parse_control_entry(line: &str) -> Result<ControlEntry, FileError> {
    let mut parts = line.split_whitespace();
    let id = parts
        .next()
        .ok_or_else(|| unknown_format(line.as_bytes()))?;
    if !id.starts_with("IDC_") {
        return Err(unknown_format(id.as_bytes()));
    }
    let code = parts.next().ok_or_else(|| unknown_format(id.as_bytes()))?;
    Ok(ControlEntry {
        id: id.to_string(),
        code: parse_hex_word(code)?,
        sprite: quoted_value(line),
    })
}

/// A top-level `NAME 0xN [0xM]` block header (depth 0 only). Matches
/// by prefix, so the real-world `DAILOG_SELECTMUSIC` typo parses
/// like any other block. Returns `None` for non-header lines.
fn parse_block_header(line: &str) -> Result<Option<ControlBlock>, FileError> {
    let mut words = line.split_whitespace();
    let (Some(name), Some(number)) = (words.next(), words.next()) else {
        return Ok(None);
    };
    let is_block = ["STATE_", "DIALOG_", "DAILOG_"]
        .iter()
        .any(|prefix| name.starts_with(prefix));
    if !is_block || !(number.starts_with("0x") || number.starts_with("0X")) {
        return Ok(None);
    }
    let mut extra = Vec::new();
    for word in words {
        if word.starts_with("//") {
            break;
        }
        extra.push(parse_hex_word(word)?);
    }
    Ok(Some(ControlBlock {
        name: name.to_string(),
        number: parse_hex_word(number)?,
        extra,
        bound_file: None,
        bound: Vec::new(),
        sets: Vec::new(),
    }))
}

/// Incremental `ControlList` builder: one trimmed line at a time.
struct ControlListParser {
    list: ControlList,
    block: Option<ControlBlock>,
    open_set: Option<ControlSet>,
    depth: usize,
}

impl ControlListParser {
    fn new() -> Self {
        Self {
            list: ControlList {
                num_states: None,
                num_dialogs: None,
                blocks: Vec::new(),
            },
            block: None,
            open_set: None,
            depth: 0,
        }
    }

    fn finish(mut self) -> ControlList {
        self.close_block();
        self.list
    }

    fn close_set(&mut self) {
        if let Some(set) = self.open_set.take() {
            if let Some(block) = self.block.as_mut() {
                block.sets.push(set);
            }
        }
    }

    fn close_block(&mut self) {
        self.close_set();
        if let Some(block) = self.block.take() {
            self.list.blocks.push(block);
        }
    }

    fn push_entry(&mut self, line: &str) -> Result<(), FileError> {
        let entry = parse_control_entry(line)?;
        if let Some(set) = self.open_set.as_mut() {
            set.entries.push(entry);
            return Ok(());
        }
        let Some(block) = self.block.as_mut() else {
            return Err(unknown_format(line.as_bytes()));
        };
        if block.sets.is_empty() {
            block.bound.push(entry);
            return Ok(());
        }
        // Continuation run after a SET group (e.g. `STATE_ROOM`'s
        // `EXBUTTON` rows): fold into a trailing parameter-less
        // section so `all_entries` keeps file order.
        match block.sets.last_mut() {
            Some(last) if last.params.is_empty() => last.entries.push(entry),
            _ => block.sets.push(ControlSet {
                params: Vec::new(),
                entries: vec![entry],
            }),
        }
        Ok(())
    }

    fn parse_count(&mut self, line: &str, is_state: bool) -> Result<(), FileError> {
        let word = line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| unknown_format(line.as_bytes()))?;
        let value = parse_hex_word(word)?;
        if is_state {
            self.list.num_states = Some(value);
        } else {
            self.list.num_dialogs = Some(value);
        }
        Ok(())
    }

    fn parse_line(&mut self, line: &str) -> Result<(), FileError> {
        if line.is_empty() || line.starts_with("//") {
            return Ok(());
        }
        if line == "{" {
            self.depth += 1;
            return Ok(());
        }
        if line == "}" {
            match self.depth {
                2 => self.close_set(),
                1 => self.close_block(),
                _ => return Err(unknown_format(line.as_bytes())),
            }
            self.depth -= 1;
            return Ok(());
        }
        if line.starts_with("NUMBER_OF_STATE") {
            return self.parse_count(line, true);
        }
        if line.starts_with("NUMBER_OF_DIALOG") {
            return self.parse_count(line, false);
        }
        if let Some(rest) = line.strip_prefix("BOUND") {
            let Some(block) = self.block.as_mut() else {
                return Err(unknown_format(line.as_bytes()));
            };
            block.bound_file = quoted_value(rest);
            return Ok(());
        }
        if let Some(rest) = line.strip_prefix("SET") {
            self.close_set();
            let mut params = Vec::new();
            for word in rest.split_whitespace() {
                if word.starts_with("//") {
                    break;
                }
                params.push(parse_hex_word(word)?);
            }
            self.open_set = Some(ControlSet {
                params,
                entries: Vec::new(),
            });
            return Ok(());
        }
        if line.starts_with("IDC_") {
            return self.push_entry(line);
        }
        if self.depth == 0 {
            if let Some(block) = parse_block_header(line)? {
                self.close_block();
                self.block = Some(block);
            }
        }
        Ok(())
    }
}

/// Parse a decoded `ControlList_*.txt` file (see
/// [`decode_control_text`]): `NUMBER_OF_*` counts plus
/// `STATE_`/`DIALOG_` blocks with `BOUND`/`SET` sections.
/// Comment lines (`//…`) and blank lines are skipped; unknown
/// top-level lines are ignored so future file revisions stay
/// readable, but malformed entries and unbalanced braces are
/// errors.
pub fn parse_control_list(text: &str) -> Result<ControlList, FileError> {
    let mut parser = ControlListParser::new();
    for raw_line in text.lines() {
        parser.parse_line(raw_line.trim())?;
    }
    Ok(parser.finish())
}
