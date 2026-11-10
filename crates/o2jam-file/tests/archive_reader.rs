//! Direct archive-reader tests over synthetic bytes (no game data).
//!
//! The reader seam is [`o2jam_file::parse_opi`] plus
//! [`o2jam_file::parse_opi_slots`] plus
//! [`o2jam_file::opi_entry_body`]; the bytes come from the
//! test-support builder [`o2jam_file::test_archive::build_archive`].

use o2jam_file::{opi_entry_body, parse_opi, parse_opi_slots, test_archive};

#[test]
fn valid_archive_lists_chosen_entries_with_bodies() {
    let archive = test_archive::build_archive(&[
        ("a.ojs", b"hello".as_slice()),
        ("b.txt", b"world!".as_slice()),
    ]);
    let entries = parse_opi(&archive).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "a.ojs");
    assert_eq!(entries[1].name, "b.txt");
    assert_eq!(opi_entry_body(&archive, &entries[0]).unwrap(), b"hello");
    assert_eq!(opi_entry_body(&archive, &entries[1]).unwrap(), b"world!");
}

#[test]
fn truncated_header_fails_as_too_short() {
    let archive = test_archive::build_archive(&[("a.ojs", b"hello".as_slice())]);
    let cut = &archive[..8];
    assert!(matches!(
        parse_opi(cut),
        Err(o2jam_file::FileError::TooShort(_))
    ));
    assert!(matches!(
        parse_opi_slots(cut),
        Err(o2jam_file::FileError::TooShort(_))
    ));
}

#[test]
fn slot_count_past_end_fails_as_truncated() {
    let mut archive = test_archive::build_archive(&[("a.ojs", b"hello".as_slice())]);
    test_archive::set_slot_count(&mut archive, u32::MAX);
    assert!(matches!(
        parse_opi(&archive),
        Err(o2jam_file::FileError::Truncated { .. })
    ));
}

#[test]
fn body_size_past_end_fails_as_truncated() {
    let mut archive = test_archive::build_archive(&[("a.ojs", b"hello".as_slice())]);
    test_archive::set_first_entry_size(&mut archive, u32::MAX);
    assert!(matches!(
        parse_opi(&archive),
        Err(o2jam_file::FileError::Truncated { .. })
    ));
}

#[test]
fn corrupt_variant_fails_to_parse() {
    let archive = test_archive::build_corrupt_archive();
    assert!(parse_opi(&archive).is_err());
}
