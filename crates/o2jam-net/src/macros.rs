//! `wire_enum!`: one discriminant table per wire enum.
//!
//! The command/relay/rooms/wallet enums each hand-synced a `from_id`
//! (or `from_code`/`from_u32`) match against the enum arms; every new
//! opcode was two edits, and the second one was easy to miss. The
//! macro generates the enum, the resolver, and optionally the wire
//! encoder from a single table, so the match cannot drift again.
//! Method names stay per-enum (`from_code`, `as_u32`, ...) to keep
//! the existing call sites untouched.

/// Define a fieldless wire enum plus its value resolver (and,
/// with `encode`, its wire encoder) from one discriminant table.
macro_rules! wire_enum {
    ($(#[$emeta:meta])* $vis:vis enum $name:ident: $repr:ident, resolve $resolver:ident, encode $encoder:ident, {
        $($(#[$vmeta:meta])* $variant:ident = $value:expr,)*
    }) => {
        $(#[$emeta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr($repr)]
        $vis enum $name {
            $($(#[$vmeta])* $variant = $value,)*
        }

        impl $name {
            /// Resolve a received wire value; `None` means unknown.
            #[must_use]
            $vis fn $resolver(value: $repr) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)*
                    _ => None,
                }
            }

            /// Numeric wire value.
            #[must_use]
            $vis fn $encoder(self) -> $repr {
                self as $repr
            }
        }
    };
    ($(#[$emeta:meta])* $vis:vis enum $name:ident: $repr:ident, resolve $resolver:ident, {
        $($(#[$vmeta:meta])* $variant:ident = $value:expr,)*
    }) => {
        $(#[$emeta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr($repr)]
        $vis enum $name {
            $($(#[$vmeta])* $variant = $value,)*
        }

        impl $name {
            /// Resolve a received wire value; `None` means unknown.
            #[must_use]
            $vis fn $resolver(value: $repr) -> Option<Self> {
                match value {
                    $($value => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

pub(crate) use wire_enum;
