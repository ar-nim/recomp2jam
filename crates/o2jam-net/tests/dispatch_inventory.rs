//! Guards the documented opcode inventory against the real dispatch surface.
//!
//! Live caller chain: `GatewayConnection` frames (and relay TCP frames)
//! reach `OnlineSession::poll_wire`, which dispatches through
//! `Response::from_id`, `Event::from_id`, `Generic::from_id`,
//! `RelayCommand::from_id`, and the `WireMessage::COMMAND` checks used by
//! `WireMessage::decode_frame` and the session's typed arms. Those public
//! declarations, plus the opcode table in `docs/net-protocol.md`, are the
//! set of dispatched ids, together with named session constants matching
//! `*_COMMAND`, `*_OPCODE` or `*_ID`. A new arm without an inventory row
//! fails here.
//!
//! This is a documentation invariant, not a player-visible behavior test:
//! it pins coverage of the live seam above. No install data is read, so CI
//! can run it with archives absent.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_DIR: &str = "src";
const SESSION_SOURCE: &str = "../../crates/recomp2jam/src/session.rs";
const PROTOCOL_DOC: &str = "../../docs/net-protocol.md";
const INVENTORY_DOC: &str = "../../docs/dispatch-inventory.md";
const WIRE_ENUMS: [&str; 4] = ["Request", "Response", "Event", "RelayCommand"];

fn workspace_file(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn read_workspace_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn source_files() -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    collect_rust_files(&workspace_file(SOURCE_DIR), &mut paths);
    paths.push(workspace_file(SESSION_SOURCE));
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let source = read_workspace_file(&path);
            (path, source)
        })
        .collect()
}

fn collect_rust_files(directory: &Path, paths: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot list {}: {error}", directory.display()))
    {
        let path = entry.expect("directory entry is readable").path();
        if path.is_dir() {
            collect_rust_files(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
}

fn declaration_lines(source: &str, marker: &str) -> Vec<String> {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("dispatch declaration {marker} missing from source"));
    let open = source[start..]
        .find('{')
        .unwrap_or_else(|| panic!("dispatch declaration {marker} has no body"));
    let open = start + open;
    let mut depth = 0u32;
    let mut close = None;
    for (offset, character) in source[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close.unwrap_or_else(|| panic!("dispatch declaration {marker} is unterminated"));
    source[open + 1..close].lines().map(str::to_owned).collect()
}

fn strip_comment(line: &str) -> &str {
    line.split("//").next().unwrap_or("").trim()
}

fn parse_wire_value(raw_value: &str, enum_values: &BTreeMap<String, u16>) -> u16 {
    let trimmed = raw_value
        .trim()
        .trim_end_matches(';')
        .trim_end_matches(',')
        .trim();
    let literal = trimmed
        .strip_suffix("as u16")
        .unwrap_or(trimmed)
        .trim()
        .trim_end_matches(',')
        .trim_end_matches(';')
        .trim();
    if let Some(hex) = literal
        .strip_prefix("0x")
        .or_else(|| literal.strip_prefix("0X"))
    {
        return u16::from_str_radix(hex, 16)
            .unwrap_or_else(|error| panic!("bad hex opcode `{raw_value}`: {error}"));
    }
    if let Ok(value) = literal.parse::<u16>() {
        return value;
    }
    let variant = literal.rsplit("::").next().unwrap_or(literal);
    enum_values
        .get(variant)
        .copied()
        .unwrap_or_else(|| panic!("unparsable wire opcode value `{raw_value}`"))
}

fn enum_values(
    source: &str,
    enum_name: &str,
    known: &BTreeMap<String, u16>,
) -> BTreeMap<String, u16> {
    let marker = format!("pub enum {enum_name}:");
    declaration_lines(source, &marker)
        .iter()
        .filter_map(|line| {
            let declaration = strip_comment(line);
            if declaration.is_empty()
                || declaration.starts_with('#')
                || declaration.starts_with('$')
            {
                return None;
            }
            let (variant, raw_value) = declaration.split_once('=')?;
            let value = parse_wire_value(raw_value, known);
            Some((variant.trim().to_owned(), value))
        })
        .collect()
}

fn generic_match_ids(source: &str, known: &BTreeMap<String, u16>) -> BTreeSet<u16> {
    declaration_lines(source, "impl Generic")
        .iter()
        .filter_map(|line| {
            let arm = strip_comment(line);
            if arm.is_empty() || arm.starts_with('#') || arm.starts_with('_') {
                return None;
            }
            let (pattern, _) = arm.split_once("=>")?;
            Some(parse_wire_value(pattern, known))
        })
        .collect()
}

fn macro_commands(source: &str, known: &BTreeMap<String, u16>) -> BTreeSet<u16> {
    source
        .lines()
        .filter_map(|line| {
            let declaration = strip_comment(line);
            let is_empty_command = declaration.starts_with("empty_message!(")
                || declaration.starts_with("song_empty!(");
            if !is_empty_command || declaration.contains('$') {
                return None;
            }
            let (_, arguments) = declaration.split_once('(')?;
            let value = arguments.split(',').nth(1)?;
            let value = value.trim().trim_end_matches(';').trim_end_matches(')');
            Some(parse_wire_value(value, known))
        })
        .collect()
}

fn constant_commands(source: &str, known: &BTreeMap<String, u16>) -> BTreeSet<u16> {
    source
        .lines()
        .filter_map(|line| {
            let declaration = strip_comment(line);
            let (name, rest) = declaration.split_once(": u16 =")?;
            let names_wire_id =
                name.contains("COMMAND") || name.contains("OPCODE") || name.ends_with("_ID");
            if !declaration.starts_with("const ") || !names_wire_id || declaration.contains('$') {
                return None;
            }
            Some(parse_wire_value(rest, known))
        })
        .collect()
}

fn wire_message_commands(source: &str, known: &BTreeMap<String, u16>) -> BTreeSet<u16> {
    source
        .lines()
        .filter_map(|line| {
            let declaration = strip_comment(line);
            let (_, raw_value) = declaration.split_once("const COMMAND: u16 =")?;
            if declaration.contains('$') {
                return None;
            }
            Some(parse_wire_value(raw_value, known))
        })
        .collect()
}

fn decimal_tokens(line: &str) -> Vec<u16> {
    let bytes = line.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        let preceded_by_word =
            start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        let followed_by_word =
            index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_');
        if !preceded_by_word && !followed_by_word {
            if let Ok(number) = line[start..index].parse() {
                numbers.push(number);
            }
        }
    }
    numbers
}

fn protocol_dispatched_ids() -> BTreeSet<u16> {
    let protocol = read_workspace_file(&workspace_file(PROTOCOL_DOC));
    let families = protocol
        .split("## 4. Packet families")
        .nth(1)
        .expect("protocol section 4 exists")
        .split("## 5.")
        .next()
        .expect("protocol section 4 has an end");
    let mut opcodes = BTreeSet::new();
    let mut in_wrapped_opcode_list = false;
    for line in families.lines() {
        let starts_opcode_list = ["Responses:", "Events:", "- Responses:", "- Events:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        if starts_opcode_list {
            in_wrapped_opcode_list = true;
        } else if in_wrapped_opcode_list && (line.trim().is_empty() || line.starts_with("###")) {
            in_wrapped_opcode_list = false;
        }
        if line.starts_with('|') || in_wrapped_opcode_list {
            opcodes.extend(decimal_tokens(line));
        }
    }
    opcodes
}

fn source_dispatched_ids() -> BTreeSet<u16> {
    let sources = source_files();
    let mut known = BTreeMap::new();
    for (_, source) in &sources {
        for enum_name in WIRE_ENUMS {
            if source.contains(&format!("pub enum {enum_name}:")) {
                known.extend(enum_values(source, enum_name, &known));
            }
        }
    }
    let mut ids = BTreeSet::new();
    for (_, source) in &sources {
        for enum_name in WIRE_ENUMS {
            if source.contains(&format!("pub enum {enum_name}:")) {
                ids.extend(enum_values(source, enum_name, &known).into_values());
            }
        }
        if source.contains("impl Generic") {
            ids.extend(generic_match_ids(source, &known));
        }
        ids.extend(wire_message_commands(source, &known));
        ids.extend(macro_commands(source, &known));
        ids.extend(constant_commands(source, &known));
    }
    ids
}

fn inventory_rows() -> Vec<(u16, String, String)> {
    read_workspace_file(&workspace_file(INVENTORY_DOC))
        .lines()
        .filter(|line| line.starts_with('|'))
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let id = cells.get(1)?.parse().ok()?;
            let codec = cells.get(5)?.to_string();
            let evidence = cells.get(6)?.to_string();
            Some((id, codec, evidence))
        })
        .collect()
}

#[test]
fn every_dispatched_id_has_an_inventory_entry() {
    let rows = inventory_rows();
    let inventoried: BTreeSet<u16> = rows.iter().map(|(id, _, _)| *id).collect();
    let required: BTreeSet<u16> = protocol_dispatched_ids()
        .union(&source_dispatched_ids())
        .copied()
        .collect();
    let missing: Vec<u16> = required.difference(&inventoried).copied().collect();
    assert!(
        missing.is_empty(),
        "dispatched ids missing from docs/dispatch-inventory.md: {missing:?}"
    );
}

#[test]
fn every_inventory_row_names_a_codec_or_an_open_status() {
    let rows = inventory_rows();
    let unmapped: Vec<u16> = rows
        .iter()
        .filter(|(_, codec, _)| {
            let codec = codec.trim_matches('`');
            !(codec.contains("OPEN") || codec.starts_with("o2jam-net::"))
        })
        .map(|(id, _, _)| *id)
        .collect();
    assert!(
        unmapped.is_empty(),
        "inventory rows without a codec or OPEN label: {unmapped:?}"
    );
}

#[test]
fn every_inventory_row_cites_evidence() {
    let uncited: Vec<u16> = inventory_rows()
        .iter()
        .filter(|(_, _, evidence)| evidence.trim().is_empty() || evidence == "-")
        .map(|(id, _, _)| *id)
        .collect();
    assert!(
        uncited.is_empty(),
        "inventory rows without evidence: {uncited:?}"
    );
}
