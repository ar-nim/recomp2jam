//! The archive builder reaches shell tests without game data.
//!
//! Synthetic bytes cross the same reader seam the shell uses
//! ([`o2jam_file::parse_opi`] plus [`o2jam_file::opi_entry_body`]);
//! no `install-root` read happens here.

use o2jam_file::{find_opi_entry, opi_entry_body, parse_opi, test_archive};

#[test]
fn synthetic_archive_entry_body_round_trips_without_game_data() {
    let archive = test_archive::build_archive(&[
        ("State_Room.bnd", b"rect-data".as_slice()),
        ("Back.ojs", b"sprite-data".as_slice()),
    ]);
    let entries = parse_opi(&archive).unwrap();
    let wanted = find_opi_entry(&entries, "back.ojs").unwrap();
    assert_eq!(wanted.name, "Back.ojs");
    assert_eq!(opi_entry_body(&archive, wanted).unwrap(), b"sprite-data");
}

#[test]
fn synthetic_archive_missing_entry_stays_absent() {
    let archive = test_archive::build_archive(&[("State_Room.bnd", b"rect-data".as_slice())]);
    let entries = parse_opi(&archive).unwrap();
    assert!(find_opi_entry(&entries, "Absent.ojs").is_none());
}
