//! Guards the documented opcode inventory against the live dispatch maps.
//!
//! The seam is sufficient: `OnlineSession` reads the frame id at buffer
//! offset 2 and dispatches through `Response::from_id`, `Event::from_id`,
//! `Generic::from_id`, and `RelayCommand::from_id`. Those public maps plus
//! the opcode table in `docs/net-protocol.md` are the complete set of
//! dispatched ids; an undocumented new arm fails this test before it can
//! reach a player.
//!
//! This is a documentation invariant, not a player-visible behavior, so
//! no install data is read and CI can run it with archives absent.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const COMMAND_SOURCE: &str = "src/command.rs";
const RELAY_SOURCE: &str = "src/relay/commands.rs";
const PROTOCOL_DOC: &str = "../../docs/net-protocol.md";
const INVENTORY_DOC: &str = "../../docs/dispatch-inventory.md";

fn workspace_file(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path)
}

fn read_workspace_file(relative_path: &str) -> String {
    let path = workspace_file(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn declaration_lines(source: &str, marker: &str) -> Vec<String> {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("dispatch declaration {marker} missing from source"));
    let open = source[start..]
        .find('{')
        .expect("dispatch declaration body opens");
    let close = source[start..]
        .find('}')
        .expect("dispatch declaration body closes");
    source[start + open + 1..start + close]
        .lines()
        .map(str::to_owned)
        .collect()
}

fn enum_lines(source: &str, enum_name: &str) -> Vec<String> {
    declaration_lines(source, &format!("pub enum {enum_name}:"))
}

fn ids_from_enum(source: &str, enum_name: &str) -> BTreeSet<u16> {
    enum_lines(source, enum_name)
        .iter()
        .filter_map(|line| {
            let declaration = line.split("//").next()?.trim();
            if declaration.starts_with('#') || declaration.starts_with("///") {
                return None;
            }
            let (_, value) = declaration.rsplit_once(" = ")?;
            value.trim_end_matches(',').trim().parse().ok()
        })
        .collect()
}

fn ids_from_generic_match(source: &str) -> BTreeSet<u16> {
    declaration_lines(source, "impl Generic")
        .into_iter()
        .filter_map(|line| {
            let (value, _) = line.split_once("=>")?;
            value.trim().parse().ok()
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
    let protocol = read_workspace_file(PROTOCOL_DOC);
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
    let commands = read_workspace_file(COMMAND_SOURCE);
    let relay = read_workspace_file(RELAY_SOURCE);
    let mut ids = BTreeSet::new();
    for enum_name in ["Request", "Response", "Event"] {
        ids.extend(ids_from_enum(&commands, enum_name));
    }
    ids.extend(ids_from_generic_match(&commands));
    ids.extend(ids_from_enum(&relay, "RelayCommand"));
    ids
}

fn inventory_rows() -> Vec<(u16, String, String)> {
    read_workspace_file(INVENTORY_DOC)
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
