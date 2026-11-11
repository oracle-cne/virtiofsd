// Copyright 2024 Red Hat, Inc. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/*!
 * Provides structures to represent ID maps on the command line.
 *
 * The actual conversion of the [`Vec<cmdline::IdMap>`](IdMap) we get from the command line to a
 * proper [`super::IdMap`] for runtime use is implemented in [`super`]
 * ([`super::IdMap as
 * TryFrom<Vec<cmdline::IdMap>>`](`super::IdMap#impl-TryFrom<Vec<IdMap>>-for-IdMap<Guest,+Host>`)).
 */

use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;

/// Command-line configuration for UID/GID translation between host and guest.
#[derive(Clone, Debug)]
pub enum IdMap {
    /// 1:1 translate a guest ID range to a host ID range.
    Guest {
        /// First ID in the guest range.
        from_guest: u32,
        /// First ID in the host range.
        to_host: u32,
        /// Range length.
        count: u32,
    },

    /// 1:1 translate a host ID range to a guest ID range.
    Host {
        /// First ID in the host range.
        from_host: u32,
        /// First ID in the guest range.
        to_guest: u32,
        /// Range length.
        count: u32,
    },

    /// n:1 translate a guest ID range to a single host ID.
    SquashGuest {
        /// First ID in the guest range.
        from_guest: u32,
        /// Single target host ID.
        to_host: u32,
        /// Guest range length.
        count: u32,
    },

    /// n:1 translate a host ID range to a single guest ID.
    SquashHost {
        /// First ID in the host range.
        from_host: u32,
        /// Single target guest ID.
        to_guest: u32,
        /// Host range length.
        count: u32,
    },

    /// 1:1 translate between a guest ID range and a host ID range, both directions.
    Bidirectional {
        /// First ID in the guest range.
        guest: u32,
        /// First ID in the host range.
        host: u32,
        /// Range length.
        count: u32,
    },

    /// Prohibit using the given range of guest IDs, returning an error when attempted.
    ForbidGuest {
        /// First ID in the guest range.
        from_guest: u32,
        /// Range length.
        count: u32,
    },
}

/// Errors that can occur when parsing an `IdMap` argument.
#[derive(Debug)]
pub enum IdMapError {
    /// Invalid/unknown mapping type prefix.
    InvalidPrefix(
        /// The prefix in question.
        String,
    ),

    /// Invalid number of arguments.
    InvalidLength {
        /// Number of arguments expected.
        expected: usize,
        /// Number of arguments actually seen.
        seen: usize,
    },

    /// Range overflow: base + count exceeds `u32::MAX`.
    RangeOverflow {
        /// Range base.
        base: u32,
        /// Range length.
        count: u32,
    },

    /// Destination ID is not valid.
    InvalidDestination {
        /// The destination value.
        id: u32,
    },

    /// Error parsing an integer.
    InvalidValue {
        /// The value in question that could not be parsed.
        value: String,
        /// The error we got.
        error: ParseIntError,
    },

    /// Empty range: count is zero.
    EmptyRange,
}

impl std::error::Error for IdMapError {}

impl Display for IdMapError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            IdMapError::InvalidPrefix(prefix) => write!(f, "Invalid ID map prefix {prefix}"),
            IdMapError::InvalidLength { expected, seen } => write!(
                f,
                "Invalid ID map length (expected {expected} elements, got {seen} elements)"
            ),
            IdMapError::EmptyRange => write!(f, "Empty range in ID map: count is zero"),
            IdMapError::RangeOverflow { base, count } => {
                write!(
                    f,
                    "Range overflow in ID map: {base} + {count} exceeds maximum"
                )
            }
            IdMapError::InvalidDestination { id } => {
                write!(f, "Destination ID {id} is not valid")
            }
            IdMapError::InvalidValue { value, error } => {
                write!(f, "Invalid value {value} in ID map: {error}")
            }
        }
    }
}

impl FromStr for IdMap {
    type Err = IdMapError;

    fn from_str(s: &str) -> Result<Self, IdMapError> {
        let (prefix, fields) = Self::pre_parse(s)?;

        match prefix.as_str() {
            "guest" => {
                Self::check_arg_count(&fields, 3)?;
                Self::check_range(fields[0], fields[2])?;
                Self::check_range(fields[1], fields[2])?;
                Ok(IdMap::Guest {
                    from_guest: fields[0],
                    to_host: fields[1],
                    count: fields[2],
                })
            }

            "host" => {
                Self::check_arg_count(&fields, 3)?;
                Self::check_range(fields[0], fields[2])?;
                Self::check_range(fields[1], fields[2])?;
                Ok(IdMap::Host {
                    from_host: fields[0],
                    to_guest: fields[1],
                    count: fields[2],
                })
            }

            "squash-guest" => {
                Self::check_arg_count(&fields, 3)?;
                Self::check_range(fields[0], fields[2])?;
                Self::check_destination(fields[1])?;
                Ok(IdMap::SquashGuest {
                    from_guest: fields[0],
                    to_host: fields[1],
                    count: fields[2],
                })
            }

            "squash-host" => {
                Self::check_arg_count(&fields, 3)?;
                Self::check_range(fields[0], fields[2])?;
                Self::check_destination(fields[1])?;
                Ok(IdMap::SquashHost {
                    from_host: fields[0],
                    to_guest: fields[1],
                    count: fields[2],
                })
            }

            "forbid-guest" => {
                Self::check_arg_count(&fields, 2)?;
                Self::check_range(fields[0], fields[1])?;
                Ok(IdMap::ForbidGuest {
                    from_guest: fields[0],
                    count: fields[1],
                })
            }

            "map" => {
                Self::check_arg_count(&fields, 3)?;
                Self::check_range(fields[0], fields[2])?;
                Self::check_range(fields[1], fields[2])?;
                Ok(IdMap::Bidirectional {
                    guest: fields[0],
                    host: fields[1],
                    count: fields[2],
                })
            }

            _ => Err(IdMapError::InvalidPrefix(prefix)),
        }
    }
}

impl Display for IdMap {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            IdMap::Guest {
                from_guest,
                to_host,
                count,
            } => {
                write!(f, "guest:{from_guest}:{to_host}:{count}")
            }
            IdMap::Host {
                from_host,
                to_guest,
                count,
            } => {
                write!(f, "host:{from_host}:{to_guest}:{count}")
            }
            IdMap::SquashGuest {
                from_guest,
                to_host,
                count,
            } => {
                write!(f, "squash-guest:{from_guest}:{to_host}:{count}")
            }
            IdMap::SquashHost {
                from_host,
                to_guest,
                count,
            } => {
                write!(f, "squash-host:{from_host}:{to_guest}:{count}")
            }
            IdMap::ForbidGuest { from_guest, count } => {
                write!(f, "forbid-guest:{from_guest}:{count}")
            }
            IdMap::Bidirectional { guest, host, count } => {
                write!(f, "map:{guest}:{host}:{count}")
            }
        }
    }
}

impl IdMap {
    /**
     * Helper for [`Self::from_str()`].
     *
     * Pre-parse an argument of the form `/^[a-zA-Z0-9_-]*(:[0-9]+){expected_len}$/` (separator
     * given as a colon here, but is allowed to be any non-alphanumeric separator except `-` and
     * `_`, though it must be the same for all fields).
     *
     * The prefix is returned as a string, the remaining numerical values as a parsed vector.
     */
    fn pre_parse(s: &str) -> Result<(String, Vec<u32>), IdMapError> {
        let mut chars = s.chars();
        let mut prefix = String::new();

        let separator = loop {
            let Some(c) = chars.next() else {
                return Err(IdMapError::InvalidLength {
                    // Not entirely right, but not entirely wrong either.  1 argument is always
                    // expected.
                    expected: 1,
                    seen: 0,
                });
            };

            if c.is_alphanumeric() || c == '-' || c == '_' {
                for c in c.to_lowercase() {
                    prefix.push(c);
                }
            } else {
                break c;
            }
        };

        let values: Vec<&str> = chars.as_str().split(separator).collect();

        let values = values
            .into_iter()
            .map(|v| {
                v.parse().map_err(|error| IdMapError::InvalidValue {
                    value: String::from(v),
                    error,
                })
            })
            .collect::<Result<Vec<u32>, IdMapError>>()?;

        Ok((prefix, values))
    }

    /// Verifies that `count` is non-zero and `base + count` does not overflow `u32`.
    fn check_range(base: u32, count: u32) -> Result<(), IdMapError> {
        if count == 0 {
            return Err(IdMapError::EmptyRange);
        }
        let _ = base
            .checked_add(count)
            .ok_or(IdMapError::RangeOverflow { base, count })?;
        Ok(())
    }

    /// Rejects `u32::MAX` as a mapping destination.
    fn check_destination(id: u32) -> Result<(), IdMapError> {
        if id == crate::fuse::INVALID_UIDGID {
            return Err(IdMapError::InvalidDestination { id });
        }
        Ok(())
    }

    /// Verifies that `args`’s length is `expected_count`, returning an error otherwise.
    fn check_arg_count(args: &[u32], expected_count: usize) -> Result<(), IdMapError> {
        if args.len() != expected_count {
            Err(IdMapError::InvalidLength {
                expected: expected_count,
                seen: args.len(),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_overflow_rejected() {
        // count=1 is the tightest overflow: u32::MAX + 1 wraps.
        let cases = [
            "guest:4294967295:0:1",
            "guest:0:4294967295:1",
            "host:4294967295:0:1",
            "host:0:4294967295:1",
            "map:4294967295:0:1",
            "map:0:4294967295:1",
            "forbid-guest:4294967295:1",
            "squash-guest:4294967295:0:1",
            "squash-host:4294967295:0:1",
        ];
        for input in cases {
            let err = input.parse::<IdMap>().unwrap_err();
            assert!(
                matches!(err, IdMapError::RangeOverflow { .. }),
                "{}: expected RangeOverflow, got: {}",
                input,
                err
            );
        }
    }

    #[test]
    fn empty_range_rejected() {
        let cases = [
            "guest:0:0:0",
            "host:0:0:0",
            "map:0:0:0",
            "forbid-guest:0:0",
            "squash-guest:0:0:0",
            "squash-host:0:0:0",
        ];
        for input in cases {
            let err = input.parse::<IdMap>().unwrap_err();
            assert!(
                matches!(err, IdMapError::EmptyRange),
                "{}: expected EmptyRange, got: {}",
                input,
                err
            );
        }
    }

    #[test]
    fn valid_ranges_accepted() {
        let cases = [
            "guest:0:1000:100",
            "host:0:1000:100",
            "map:0:1000:100",
            "forbid-guest:0:100",
            "squash-guest:0:1000:100",
            "squash-host:0:1000:100",
        ];
        for input in cases {
            assert!(input.parse::<IdMap>().is_ok(), "{}: must succeed", input);
        }
    }

    #[test]
    fn invalid_destination_rejected() {
        let cases = [
            "squash-guest:0:4294967295:100",
            "squash-host:0:4294967295:100",
        ];
        for input in cases {
            let err = input.parse::<IdMap>().unwrap_err();
            assert!(
                matches!(err, IdMapError::InvalidDestination { .. }),
                "{}: expected InvalidDestination, got: {}",
                input,
                err
            );
        }
    }

    #[test]
    fn max_valid_range_accepted() {
        // base=0, count=u32::MAX covers 0..4294967295 (excludes u32::MAX itself).
        let cases = ["guest:0:0:4294967295", "forbid-guest:0:4294967295"];
        for input in cases {
            assert!(input.parse::<IdMap>().is_ok(), "{}: must succeed", input);
        }
    }
}
