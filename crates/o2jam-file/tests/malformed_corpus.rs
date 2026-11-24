//! Deterministic malformed-data corpus for the shipped file parsers (#905).
//!
//! Every case starts from a small synthetic but valid file, then applies
//! one named mutation: truncation, an over-long length/count, or a
//! bit-flipped structural marker. Each case must return a named
//! `FileError`; a panic or an `Ok` result fails the guard.
//!
//! No game data is read, so CI runs the corpus with `install-root/`
//! absent. Live caller chain: `cargo test --workspace` -> this test ->
//! the public parser entry points used by boot-time asset loading.
//! Exe failure behavior on corrupt data stays OPEN; see
//! `docs/file-formats/corpus.md` for the next evidence step.

use std::panic::{catch_unwind, AssertUnwindSafe};

use o2jam_file::{
    parse_avatar_item_table, parse_bnd, parse_control_list, parse_ojm, parse_ojn, parse_ojn_levels,
    parse_ojn_list, parse_opi, parse_set_info_table, FileError, OPI_ENTRY_LEN, OPI_HEADER_LEN,
    OPI_NAME_OFF, OPI_OFFSET_OFF, OPI_RESERVED_OFF, OPI_SIZE_OFF,
};

type ParseFn = fn(&[u8]) -> Result<(), FileError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mutation {
    Truncated,
    OverlongLength,
    BitFlippedMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamedError {
    TooShort,
    UnknownFormat,
    Truncated,
}

impl NamedError {
    fn matches(self, error: &FileError) -> bool {
        matches!(
            (self, error),
            (Self::TooShort, FileError::TooShort(_))
                | (Self::UnknownFormat, FileError::UnknownFormat { .. })
                | (Self::Truncated, FileError::Truncated { .. })
        )
    }
}

struct MalformedCase {
    parser: &'static str,
    mutation: Mutation,
    bytes: Vec<u8>,
    parse: ParseFn,
    expected: NamedError,
}

impl MalformedCase {
    fn new(
        parser: &'static str,
        mutation: Mutation,
        bytes: Vec<u8>,
        parse: ParseFn,
        expected: NamedError,
    ) -> Self {
        Self {
            parser,
            mutation,
            bytes,
            parse,
            expected,
        }
    }

    fn assert_rejected(self) {
        let outcome = catch_unwind(AssertUnwindSafe(|| (self.parse)(&self.bytes)));
        let result =
            outcome.unwrap_or_else(|_| panic!("{} panicked on {:?}", self.parser, self.mutation));
        let error = match result {
            Ok(()) => panic!("{} silently accepted {:?}", self.parser, self.mutation),
            Err(error) => error,
        };
        assert!(
            self.expected.matches(&error),
            "{} on {:?} expected {:?}, got {error}",
            self.parser,
            self.mutation,
            self.expected
        );
    }
}

fn assert_accepted(parser: &'static str, bytes: &[u8], parse: ParseFn) {
    let outcome = catch_unwind(AssertUnwindSafe(|| parse(bytes)));
    let result = outcome.unwrap_or_else(|_| panic!("{parser} panicked on valid seed"));
    assert!(
        result.is_ok(),
        "{parser} rejected its valid seed: {result:?}"
    );
}

fn assert_all_rejected(cases: impl IntoIterator<Item = MalformedCase>) {
    for case in cases {
        case.assert_rejected();
    }
}

fn write_u16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn text_field(value: &[u8]) -> Vec<u8> {
    let length = u32::try_from(value.len()).expect("synthetic text field");
    let mut field = length.to_le_bytes().to_vec();
    field.extend_from_slice(value);
    field
}

fn parse_as_unit<T>(result: Result<T, FileError>) -> Result<(), FileError> {
    result.map(|_| ())
}

fn parse_control_bytes(bytes: &[u8]) -> Result<(), FileError> {
    let text = std::str::from_utf8(bytes).map_err(|_| FileError::UnknownFormat {
        magic: bytes.to_vec(),
    })?;
    parse_as_unit(parse_control_list(text))
}

fn opi_seed() -> Vec<u8> {
    let body = b"hello";
    let table_at = OPI_HEADER_LEN + body.len();
    let mut bytes = vec![0u8; table_at + OPI_ENTRY_LEN];
    write_u32(&mut bytes, 0, 2);
    write_u32(&mut bytes, 4, 1);
    bytes[OPI_HEADER_LEN..table_at].copy_from_slice(body);
    bytes[table_at] = 1;
    bytes[table_at + OPI_NAME_OFF..table_at + OPI_NAME_OFF + 5].copy_from_slice(b"a.ojs");
    write_u32(
        &mut bytes,
        table_at + OPI_OFFSET_OFF,
        u32::try_from(OPI_HEADER_LEN).expect("small fixture"),
    );
    write_u32(
        &mut bytes,
        table_at + OPI_SIZE_OFF,
        u32::try_from(body.len()).expect("small fixture"),
    );
    write_u32(
        &mut bytes,
        table_at + OPI_RESERVED_OFF,
        u32::try_from(body.len()).expect("small fixture"),
    );
    bytes
}

fn ojn_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 300];
    bytes[0..4].copy_from_slice(b"ojn\0");
    for offset in [284, 288, 292, 296] {
        write_u32(&mut bytes, offset, 300);
    }
    bytes
}

fn ojm_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 20 + 52 + 4];
    bytes[0..4].copy_from_slice(b"M30\0");
    write_u32(&mut bytes, 12, 1);
    write_u32(&mut bytes, 16, 20);
    bytes[24..28].copy_from_slice(b"key\0");
    write_u32(&mut bytes, 20 + 32, 4);
    bytes[72..76].copy_from_slice(b"OggS");
    bytes
}

fn omc_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 16 + 36 + 4];
    bytes[0..4].copy_from_slice(b"OMC\0");
    write_u32(&mut bytes, 12, 16);
    bytes[24..28].copy_from_slice(b"bgm\0");
    write_u32(&mut bytes, 16 + 32, 4);
    bytes[52..56].copy_from_slice(b"OggS");
    bytes
}

fn bnd_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 6];
    bytes[0..4].copy_from_slice(&[0xFF; 4]);
    bytes
}

fn control_list_seed() -> Vec<u8> {
    b"NUMBER_OF_STATE 0x01\nSTATE_OK 0x01\n{\nBOUND \"ok.bnd\"\nIDC_OK 0X00000001 \"ok.ojs\"\n}\n"
        .to_vec()
}

fn item_table_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 84];
    write_u32(&mut bytes, 0, 2);
    let mut row = vec![0u8; 30];
    write_u32(&mut row, 0, 1);
    row.extend(text_field(b"A"));
    row.extend(text_field(b"B"));
    for _ in 0..42 {
        row.push(0);
        row.extend(text_field(b""));
    }
    bytes.extend(row);
    bytes
}

fn set_info_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 81];
    write_u32(&mut bytes, 0, 1);
    let mut row = vec![0u8; 17 * 4 + 1];
    write_u32(&mut row, 0, 1);
    row.extend(text_field(b"A"));
    row.extend(text_field(b"B"));
    bytes.extend(row);
    bytes
}

fn song_list_seed() -> Vec<u8> {
    let mut bytes = vec![0u8; 300];
    bytes[8..11].copy_from_slice(b"ojn");
    bytes
}

#[test]
fn opi_parser_rejects_corpus_mutations() {
    let seed = opi_seed();
    assert_accepted("opi", &seed, |bytes| parse_as_unit(parse_opi(bytes)));

    let table_at = OPI_HEADER_LEN + 5;
    let overlong = {
        let mut bytes = seed.clone();
        write_u32(&mut bytes, table_at + OPI_SIZE_OFF, 999);
        bytes
    };
    let flipped_count = {
        let mut bytes = seed.clone();
        write_u32(&mut bytes, 4, u32::MAX);
        bytes
    };
    assert_all_rejected([
        MalformedCase::new(
            "opi",
            Mutation::Truncated,
            seed[..seed.len() - 1].to_vec(),
            |bytes| parse_as_unit(parse_opi(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "opi",
            Mutation::OverlongLength,
            overlong,
            |bytes| parse_as_unit(parse_opi(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "opi",
            Mutation::BitFlippedMarker,
            flipped_count,
            |bytes| parse_as_unit(parse_opi(bytes)),
            NamedError::Truncated,
        ),
    ]);
}

#[test]
fn ojn_parser_rejects_corpus_mutations() {
    let seed = ojn_seed();
    assert_accepted("ojn header", &seed, |bytes| parse_as_unit(parse_ojn(bytes)));
    assert_accepted("ojn levels", &seed, |bytes| {
        parse_as_unit(parse_ojn_levels(bytes))
    });

    let flipped_magic = {
        let mut bytes = seed.clone();
        bytes[0] = b'X';
        bytes
    };
    let overlong_slice = {
        let mut bytes = seed.clone();
        write_u32(&mut bytes, 288, u32::MAX);
        bytes
    };
    assert_all_rejected([
        MalformedCase::new(
            "ojn header",
            Mutation::Truncated,
            seed[..16].to_vec(),
            |bytes| parse_as_unit(parse_ojn(bytes)),
            NamedError::TooShort,
        ),
        MalformedCase::new(
            "ojn header",
            Mutation::BitFlippedMarker,
            flipped_magic,
            |bytes| parse_as_unit(parse_ojn(bytes)),
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "ojn levels",
            Mutation::OverlongLength,
            overlong_slice,
            |bytes| parse_as_unit(parse_ojn_levels(bytes)),
            NamedError::Truncated,
        ),
    ]);
}

#[test]
fn ojm_parser_rejects_corpus_mutations() {
    let seed = ojm_seed();
    let omc = omc_seed();
    assert_accepted("ojm m30", &seed, |bytes| parse_as_unit(parse_ojm(bytes)));
    assert_accepted("ojm omc", &omc, |bytes| parse_as_unit(parse_ojm(bytes)));

    let overlong = {
        let mut bytes = seed.clone();
        write_u32(&mut bytes, 20 + 32, 999);
        bytes
    };
    let flipped_magic = {
        let mut bytes = seed.clone();
        bytes[0] = b'X';
        bytes
    };
    let omc_truncated = omc[..omc.len() - 1].to_vec();
    assert_all_rejected([
        MalformedCase::new(
            "ojm m30",
            Mutation::Truncated,
            seed[..seed.len() - 1].to_vec(),
            |bytes| parse_as_unit(parse_ojm(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "ojm m30",
            Mutation::OverlongLength,
            overlong,
            |bytes| parse_as_unit(parse_ojm(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "ojm m30",
            Mutation::BitFlippedMarker,
            flipped_magic,
            |bytes| parse_as_unit(parse_ojm(bytes)),
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "ojm omc",
            Mutation::Truncated,
            omc_truncated,
            |bytes| parse_as_unit(parse_ojm(bytes)),
            NamedError::Truncated,
        ),
    ]);
}

#[test]
fn bnd_parser_rejects_corpus_mutations() {
    let seed = bnd_seed();
    assert_accepted("bnd", &seed, |bytes| parse_as_unit(parse_bnd(bytes)));

    let overlong_count = {
        let mut bytes = seed.clone();
        write_u16(&mut bytes, 4, 2);
        bytes
    };
    let flipped_magic = {
        let mut bytes = seed.clone();
        bytes[0] = 0xFE;
        bytes
    };
    assert_all_rejected([
        MalformedCase::new(
            "bnd",
            Mutation::Truncated,
            seed[..5].to_vec(),
            |bytes| parse_as_unit(parse_bnd(bytes)),
            NamedError::TooShort,
        ),
        MalformedCase::new(
            "bnd",
            Mutation::OverlongLength,
            overlong_count,
            |bytes| parse_as_unit(parse_bnd(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "bnd",
            Mutation::BitFlippedMarker,
            flipped_magic,
            |bytes| parse_as_unit(parse_bnd(bytes)),
            NamedError::UnknownFormat,
        ),
    ]);
}

#[test]
fn control_list_parser_rejects_corpus_mutations() {
    let seed = control_list_seed();
    assert_accepted("control list", &seed, parse_control_bytes);

    let truncated_code = String::from_utf8(seed.clone())
        .expect("ascii seed")
        .replace("0X00000001", "0X")
        .into_bytes();
    let overlong_braces = {
        let mut bytes = seed.clone();
        bytes.extend_from_slice(b"}\n");
        bytes
    };
    let flipped_prefix = {
        let text = String::from_utf8(seed.clone()).expect("ascii seed");
        text.replace("0X00000001", "0Z00000001").into_bytes()
    };
    assert_all_rejected([
        MalformedCase::new(
            "control list",
            Mutation::Truncated,
            truncated_code,
            parse_control_bytes,
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "control list",
            Mutation::OverlongLength,
            overlong_braces,
            parse_control_bytes,
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "control list",
            Mutation::BitFlippedMarker,
            flipped_prefix,
            parse_control_bytes,
            NamedError::UnknownFormat,
        ),
    ]);
}

#[test]
fn item_table_parsers_reject_corpus_mutations() {
    let items = item_table_seed();
    let sets = set_info_seed();
    assert_accepted("item table", &items, |bytes| {
        parse_as_unit(parse_avatar_item_table(bytes))
    });
    assert_accepted("set info table", &sets, |bytes| {
        parse_as_unit(parse_set_info_table(bytes))
    });

    let items_overlong = {
        let mut bytes = items.clone();
        write_u32(&mut bytes, 0, 99);
        bytes
    };
    let items_flipped = {
        let mut bytes = items.clone();
        bytes[84 + 30 + 4] = 0xFF;
        bytes
    };
    let sets_truncated_text = {
        let mut bytes = sets.clone();
        write_u32(&mut bytes, 81 + 17 * 4 + 1, 999);
        bytes
    };
    let sets_flipped_id = {
        let mut bytes = sets.clone();
        write_u32(&mut bytes, 81, 2);
        bytes
    };
    assert_all_rejected([
        MalformedCase::new(
            "item table",
            Mutation::Truncated,
            items[..items.len() - 1].to_vec(),
            |bytes| parse_as_unit(parse_avatar_item_table(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "item table",
            Mutation::OverlongLength,
            items_overlong,
            |bytes| parse_as_unit(parse_avatar_item_table(bytes)),
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "item table",
            Mutation::BitFlippedMarker,
            items_flipped,
            |bytes| parse_as_unit(parse_avatar_item_table(bytes)),
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "set info table",
            Mutation::Truncated,
            sets[..sets.len() - 1].to_vec(),
            |bytes| parse_as_unit(parse_set_info_table(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "set info table",
            Mutation::OverlongLength,
            sets_truncated_text,
            |bytes| parse_as_unit(parse_set_info_table(bytes)),
            NamedError::Truncated,
        ),
        MalformedCase::new(
            "set info table",
            Mutation::BitFlippedMarker,
            sets_flipped_id,
            |bytes| parse_as_unit(parse_set_info_table(bytes)),
            NamedError::TooShort,
        ),
    ]);
}

#[test]
fn song_list_parser_rejects_corpus_mutations() {
    let seed = song_list_seed();
    assert_accepted("song list", &seed, |bytes| {
        parse_as_unit(parse_ojn_list(bytes))
    });

    let flipped_magic = {
        let mut bytes = seed.clone();
        bytes[8] = b'X';
        bytes
    };
    assert_all_rejected([
        MalformedCase::new(
            "song list",
            Mutation::Truncated,
            seed[..299].to_vec(),
            |bytes| parse_as_unit(parse_ojn_list(bytes)),
            NamedError::UnknownFormat,
        ),
        MalformedCase::new(
            "song list",
            Mutation::BitFlippedMarker,
            flipped_magic,
            |bytes| parse_as_unit(parse_ojn_list(bytes)),
            NamedError::UnknownFormat,
        ),
    ]);
}
