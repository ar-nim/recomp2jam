//! Count-prefixed fixed-row blocks in their three pinned decodings.
//!
//! The 7.55 handlers bound their count loops three different ways, and
//! each is pinned by exe evidence — the variants must never be
//! unified:
//!
//! - [`rows_reject_negative`]: a negative count is
//!   `Error::InvalidField` (`message` channel list, `play` score
//!   completed). Offered in an exact-length and a min-length form.
//! - [`rows_zero_on_negative`]: a negative count trips the exe's
//!   signed loop exit test, skipping the copy loops, so the client
//!   decodes an empty list (`relay` peer list, member music ids).
//!
//! Cursor-shaped copies (variable-length rows, `Reader`-driven
//! decoders, unsigned counts) keep their hand-rolled loops: their
//! truncation and trailing-byte error shapes are pinned per handler
//! and do not fit the upfront-length plans below.

use super::error::Error;

/// How a signed count prefix treats negative values.
enum NegativeCount {
    /// `Error::InvalidField` with the caller's field and reason.
    Reject(&'static str, &'static str),
    /// Clamp to zero rows: the exe's signed loop exit test skips the
    /// copy loops, so the client decodes an empty list.
    ZeroRows,
}

/// Whether bytes may follow the last row.
enum Trailing {
    /// `Error::TrailingBytes` on any byte past the plan length.
    Reject,
    /// The body legitimately continues after the rows.
    Allow,
}

/// Decoded row count for one count-prefixed fixed-row block.
struct RowPlan {
    count: usize,
}

/// Shared length plan for one count-prefixed fixed-row block.
///
/// `min_len` is the pinned upfront gate: bodies shorter than it
/// report `TruncatedMessage { need: min_len, .. }` before the count
/// is read. It must cover `head_len + count_len`.
#[allow(clippy::too_many_arguments)]
fn plan_rows(
    body: &[u8],
    min_len: usize,
    head_len: usize,
    count_len: usize,
    row_len: usize,
    tail_len: usize,
    negative: NegativeCount,
    count_field: &'static str,
    overflow_reason: &'static str,
    trailing: Trailing,
) -> Result<RowPlan, Error> {
    debug_assert!(min_len >= head_len + count_len);
    if body.len() < min_len {
        return Err(Error::TruncatedMessage {
            need: min_len,
            have: body.len(),
        });
    }
    let count_at = head_len;
    let raw = match count_len {
        2 => i32::from(i16::from_le_bytes([body[count_at], body[count_at + 1]])),
        4 => i32::from_le_bytes([
            body[count_at],
            body[count_at + 1],
            body[count_at + 2],
            body[count_at + 3],
        ]),
        _ => unreachable!("count prefix is 2 or 4 bytes"),
    };
    let count = match negative {
        NegativeCount::Reject(field, reason) => {
            if raw < 0 {
                return Err(Error::InvalidField { field, reason });
            }
            raw as usize
        }
        NegativeCount::ZeroRows => usize::try_from(raw).unwrap_or(0),
    };
    let rows_len = count.checked_mul(row_len).ok_or(Error::InvalidField {
        field: count_field,
        reason: overflow_reason,
    })?;
    let need = head_len
        .checked_add(count_len)
        .and_then(|base| base.checked_add(rows_len))
        .and_then(|base| base.checked_add(tail_len))
        .ok_or(Error::InvalidField {
            field: count_field,
            reason: overflow_reason,
        })?;
    if body.len() < need {
        return Err(Error::TruncatedMessage {
            need,
            have: body.len(),
        });
    }
    if let Trailing::Reject = trailing {
        if body.len() > need {
            return Err(Error::TrailingBytes(body.len() - need));
        }
    }
    Ok(RowPlan { count })
}

/// Split the planned rows out of `body` and parse each with `read_row`.
fn take_rows<T>(
    body: &[u8],
    plan: &RowPlan,
    head_len: usize,
    count_len: usize,
    row_len: usize,
    mut read_row: impl FnMut(&[u8]) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let first_row = head_len + count_len;
    let mut rows = Vec::with_capacity(plan.count);
    for index in 0..plan.count {
        let at = first_row + index * row_len;
        rows.push(read_row(&body[at..at + row_len])?);
    }
    Ok(rows)
}

/// Negative count is `InvalidField`; the body must end after the
/// (optional) tail — any further byte is `TrailingBytes`.
///
/// Pinned at `message` channel list and `play` score completed.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rows_reject_negative_exact<T>(
    body: &[u8],
    min_len: usize,
    head_len: usize,
    count_len: usize,
    row_len: usize,
    tail_len: usize,
    count_field: &'static str,
    negative_reason: &'static str,
    overflow_reason: &'static str,
    read_row: impl FnMut(&[u8]) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let plan = plan_rows(
        body,
        min_len,
        head_len,
        count_len,
        row_len,
        tail_len,
        NegativeCount::Reject(count_field, negative_reason),
        count_field,
        overflow_reason,
        Trailing::Reject,
    )?;
    take_rows(body, &plan, head_len, count_len, row_len, read_row)
}

/// Negative count is `InvalidField`; the body may legitimately
/// continue after the rows (a flag byte, a wider message tail).
///
/// Pinned at `play` music list answer.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rows_reject_negative_min<T>(
    body: &[u8],
    min_len: usize,
    head_len: usize,
    count_len: usize,
    row_len: usize,
    tail_len: usize,
    count_field: &'static str,
    negative_reason: &'static str,
    overflow_reason: &'static str,
    read_row: impl FnMut(&[u8]) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let plan = plan_rows(
        body,
        min_len,
        head_len,
        count_len,
        row_len,
        tail_len,
        NegativeCount::Reject(count_field, negative_reason),
        count_field,
        overflow_reason,
        Trailing::Allow,
    )?;
    take_rows(body, &plan, head_len, count_len, row_len, read_row)
}

/// Negative count decodes zero rows (the exe's signed loop exit
/// test); the body must end after the rows.
///
/// Pinned at `relay` peer list; the same clamp backs the member
/// music-id sub-list, whose inline (mid-row) count keeps its local
/// `nonnegative_count` call.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rows_zero_on_negative_exact<T>(
    body: &[u8],
    min_len: usize,
    head_len: usize,
    count_len: usize,
    row_len: usize,
    tail_len: usize,
    count_field: &'static str,
    overflow_reason: &'static str,
    read_row: impl FnMut(&[u8]) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let plan = plan_rows(
        body,
        min_len,
        head_len,
        count_len,
        row_len,
        tail_len,
        NegativeCount::ZeroRows,
        count_field,
        overflow_reason,
        Trailing::Reject,
    )?;
    take_rows(body, &plan, head_len, count_len, row_len, read_row)
}

#[cfg(test)]
mod rows_tests {
    use super::*;

    const ROW_LEN: usize = 3;

    /// One row: `u16` id + one flag byte, both preserved.
    fn read_test_row(row: &[u8]) -> Result<(u16, u8), Error> {
        Ok((u16::from_le_bytes([row[0], row[1]]), row[2]))
    }

    fn row(id: u16, flag: u8) -> [u8; ROW_LEN] {
        let mut bytes = id.to_le_bytes().to_vec();
        bytes.push(flag);
        bytes.try_into().expect("test row is 3 bytes")
    }

    fn body_with(count: i32, rows: &[[u8; ROW_LEN]], tail: &[u8]) -> Vec<u8> {
        let mut body = count.to_le_bytes().to_vec();
        for bytes in rows {
            body.extend_from_slice(bytes);
        }
        body.extend_from_slice(tail);
        body
    }

    mod reject_negative_exact {
        use super::*;

        fn decode(body: &[u8]) -> Result<Vec<(u16, u8)>, Error> {
            rows_reject_negative_exact(
                body,
                4,
                0,
                4,
                ROW_LEN,
                0,
                "test count",
                "negative",
                "overflow",
                read_test_row,
            )
        }

        #[test]
        fn decodes_rows_in_wire_order() {
            let body = body_with(2, &[row(7, 1), row(9, 0)], &[]);
            assert_eq!(decode(&body), Ok(vec![(7, 1), (9, 0)]));
        }

        #[test]
        fn negative_count_is_invalid_field() {
            let body = body_with(-1, &[], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::InvalidField {
                    field: "test count",
                    reason: "negative"
                })
            );
        }

        #[test]
        fn short_body_reports_full_row_need() {
            let body = body_with(2, &[row(1, 0)], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::TruncatedMessage {
                    need: 4 + 2 * ROW_LEN,
                    have: 4 + ROW_LEN
                })
            );
        }

        #[test]
        fn head_only_reports_min_len() {
            assert_eq!(
                decode(&[0x01, 0x02]),
                Err(Error::TruncatedMessage { need: 4, have: 2 })
            );
        }

        #[test]
        fn extra_byte_is_trailing() {
            let body = body_with(1, &[row(3, 1)], &[0xAA]);
            assert_eq!(decode(&body), Err(Error::TrailingBytes(1)));
        }

        #[test]
        fn row_error_propagates() {
            let body = body_with(1, &[row(3, 1)], &[]);
            let err: Result<Vec<(u16, u8)>, Error> = rows_reject_negative_exact(
                &body,
                4,
                0,
                4,
                ROW_LEN,
                0,
                "test count",
                "negative",
                "overflow",
                |_| {
                    Err(Error::InvalidField {
                        field: "test row",
                        reason: "bad",
                    })
                },
            );
            assert_eq!(
                err,
                Err(Error::InvalidField {
                    field: "test row",
                    reason: "bad"
                })
            );
        }

        #[test]
        fn huge_count_reports_full_row_need() {
            // Largest signed count: the plan rejects the body, it
            // never wraps the length arithmetic.
            let body = body_with(i32::MAX, &[], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::TruncatedMessage {
                    need: 4 + i32::MAX as usize * ROW_LEN,
                    have: 4
                })
            );
        }

        #[test]
        fn absurd_row_len_overflows_to_invalid_field() {
            // 32-bit hygiene: a row length that wraps the plan on a
            // narrow address space errors instead of wrapping.
            let body = body_with(2, &[], &[]);
            let err: Result<Vec<(u16, u8)>, Error> = rows_reject_negative_exact(
                &body,
                4,
                0,
                4,
                1usize << 63,
                0,
                "test count",
                "negative",
                "overflow",
                read_test_row,
            );
            assert_eq!(
                err,
                Err(Error::InvalidField {
                    field: "test count",
                    reason: "overflow"
                })
            );
        }
    }

    mod reject_negative_min {
        use super::*;

        const TAIL: &[u8] = &[0x01];

        fn decode(body: &[u8]) -> Result<Vec<(u16, u8)>, Error> {
            rows_reject_negative_min(
                body,
                2,
                0,
                2,
                ROW_LEN,
                TAIL.len(),
                "test count",
                "negative",
                "overflow",
                read_test_row,
            )
        }

        fn min_body_with(count: i16, rows: &[[u8; ROW_LEN]], extra: &[u8]) -> Vec<u8> {
            let mut body = count.to_le_bytes().to_vec();
            for bytes in rows {
                body.extend_from_slice(bytes);
            }
            body.extend_from_slice(TAIL);
            body.extend_from_slice(extra);
            body
        }

        #[test]
        fn decodes_rows_and_allows_continuation_bytes() {
            let body = min_body_with(1, &[row(5, 1)], &[0xBB, 0xCC]);
            assert_eq!(decode(&body), Ok(vec![(5, 1)]));
        }

        #[test]
        fn negative_count_is_invalid_field() {
            let body = min_body_with(-1, &[], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::InvalidField {
                    field: "test count",
                    reason: "negative"
                })
            );
        }

        #[test]
        fn missing_tail_byte_reports_full_need() {
            let mut body = 0i16.to_le_bytes().to_vec();
            assert_eq!(
                decode(&body),
                Err(Error::TruncatedMessage { need: 3, have: 2 })
            );
            body.push(0x01);
            assert_eq!(decode(&body), Ok(vec![]));
        }

        #[test]
        fn truncated_rows_report_full_need() {
            let body = min_body_with(2, &[row(1, 0)], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::TruncatedMessage {
                    need: 2 + 2 * ROW_LEN + 1,
                    have: 2 + ROW_LEN + 1
                })
            );
        }
    }

    mod zero_on_negative_exact {
        use super::*;

        fn decode(body: &[u8]) -> Result<Vec<(u16, u8)>, Error> {
            rows_zero_on_negative_exact(
                body,
                4,
                0,
                4,
                ROW_LEN,
                0,
                "test count",
                "overflow",
                read_test_row,
            )
        }

        #[test]
        fn negative_count_decodes_zero_rows() {
            let body = body_with(-5, &[], &[]);
            assert_eq!(decode(&body), Ok(vec![]));
        }

        #[test]
        fn zero_count_decodes_empty() {
            let body = body_with(0, &[], &[]);
            assert_eq!(decode(&body), Ok(vec![]));
        }

        #[test]
        fn decodes_rows_in_wire_order() {
            let body = body_with(2, &[row(4, 0), row(6, 1)], &[]);
            assert_eq!(decode(&body), Ok(vec![(4, 0), (6, 1)]));
        }

        #[test]
        fn extra_byte_is_trailing() {
            let body = body_with(0, &[], &[0xAA]);
            assert_eq!(decode(&body), Err(Error::TrailingBytes(1)));
        }

        #[test]
        fn overflowing_count_is_invalid_field() {
            // 32-bit hygiene: an absurd row length wraps the plan on
            // a narrow address space; the checked math errors first.
            let body = body_with(2, &[], &[]);
            let err: Result<Vec<(u16, u8)>, Error> = rows_zero_on_negative_exact(
                &body,
                4,
                0,
                4,
                1usize << 63,
                0,
                "test count",
                "overflow",
                read_test_row,
            );
            assert_eq!(
                err,
                Err(Error::InvalidField {
                    field: "test count",
                    reason: "overflow"
                })
            );
        }

        #[test]
        fn short_body_reports_full_row_need() {
            let body = body_with(1, &[], &[]);
            assert_eq!(
                decode(&body),
                Err(Error::TruncatedMessage { need: 7, have: 4 })
            );
        }
    }
}
