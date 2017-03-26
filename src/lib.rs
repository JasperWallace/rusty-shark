/*
 * Copyright 2015 Jonathan Anderson
 *
 * Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
 * http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
 * http://opensource.org/licenses/MIT>, at your option. This file may not be
 * copied, modified, or distributed except according to those terms.
 */

//!
//! `rshark`, the Rusty Shark library, is a library for deep inspection
//! of malicious packets.
//!
//! # Background
//!
//! [Wireshark](https://www.wireshark.org) is a very useful tool for network
//! debugging, but it's had its
//! [fair share of security vulnerabilities](https://www.wireshark.org/security).
//! It's generally accepted that, to succeed at Capture the Flag, one should fuzz
//! Wireshark for awhile before the competition to find a few new vulnerabilities
//! (don't worry, they're there, you'll find some) and use those offensively to
//! blind one's opponents.
//! This speaks to both the indispensability of packet capture/dissection tools
//! and the fundamental difficulty of ``just making Wireshark secure''.
//! Wireshark has a *lot* of dissectors, which are written using a
//! [complex C API](https://www.wireshark.org/docs/wsdg_html_chunked/ChDissectAdd.html)
//! (although some are now written in Lua).
//!
//! `rshark` uses the type safety of Rust to enable the dissection of
//! malicious packets without worry of buffer overflows or other common memory errors.
//! Rusty Shark dissectors can make mistakes, but those logical errors should only
//! affect the interpretation of the *current* data, rather than *all* data.
//! That is to say, Rusty Shark is compartmentalized to minimize the damage that
//! can be done by a successful adversary. The submarine metaphors write themselves.
//!
//! # Usage
//!
//! *note: for help on the `rshark` command-line client,
//! run `man rshark` or `rshark --help`.*
//!
//! The `rshark` library provides packet dissection functions such as
//! `rshark::ethernet::dissect()`. Every such dissection function, which should
//! conform to the `rshark::Dissector` function type, takes as input a slice of bytes
//! and returns an `rshark::Result` (which defaults to
//! `Result<rshark::Val, rshark::Error>`).
//! Usage is pretty simple:
//!
//! ```
//! let data = vec![];
//!
//! match rshark::ethernet::dissect(&data) {
//!     Err(e) => println!["Error: {}", e],
//!     Ok(val) => print!["{}", val.pretty_print(0)],
//! }
//! ```
//!
//! A `Val` can represent an arbitrary tree of structured data
//! (useful in graphical displays) and can be pretty-printed with indentation for
//! sub-objects.

#![doc(html_logo_url = "https://raw.githubusercontent.com/musec/rusty-shark/master/artwork/wordmark.png")]

extern crate byteorder;
extern crate num;
extern crate promising_future;

use byteorder::ByteOrder;
pub use promising_future::Future;
use std::fmt;


/// A description of a protocol, including code that can parse it.
pub trait Protocol {
    /// A short name that can fit in a user display, e.g., "IPv6".
    fn short_name(&self) -> &'static str;

    /// A complete, unambigous protocol name, e.g., "Internet Protocol version 6"
    fn full_name(&self) -> &'static str;

    /// A function to dissect some bytes according to the protocol.
    fn dissect(&self, &[u8]) -> Result;
}


/// A value parsed from a packet.
///
/// # TODO
/// This value type isn't as expressive as would be required for a real
/// Wireshark replacement just yet. Additional needs include:
///
///  * tracking original bytes (by reference or by index?)
///
#[derive(Debug)]
pub enum Val {
    /// A signed integer, in machine-native representation.
    Signed(i64),

    /// An unsigned integer, in machine-native representation, and a radix (base) for
    /// when we display the value to the user (e.g., 0x100 vs 256).
    Unsigned { value: u64, radix: u8 },

    /// An integer value that represents a symbolic value.
    Enum(u64, &'static str),

    /// A UTF-8–encoded string.
    String(String),

    /// A network address, which can have its own special encoding.
    Address { bytes: Vec<u8>, encoded: String },

    /// A set of named values from an encapsulated packet (e.g., TCP within IP).
    Subpacket(Vec<NamedValue>),

    /// Raw bytes, e.g., a checksum or just unparsed data.
    Bytes(Vec<u8>),

    /// An error that stopped further parsing.
    Error(Error),

    /// A problem with a packet that did not stop further parsing (e.g., bad checksum).
    Warning(Error),
}

impl Val {
    pub fn base2<T>(x: T) -> Result<Val>
        where T: num::ToPrimitive + std::fmt::Display
    {
        Val::unsigned(x, 2)
    }

    pub fn base10<T>(x: T) -> Result<Val>
        where T: num::ToPrimitive + std::fmt::Display
    {
        Val::unsigned(x, 10)
    }

    pub fn base16<T>(x: T) -> Result<Val>
        where T: num::ToPrimitive + std::fmt::Display
    {
        Val::unsigned(x, 16)
    }

    pub fn unsigned<T>(x: T, radix:u8) -> Result<Val>
        where T: num::ToPrimitive + std::fmt::Display
    {
        x.to_u64()
         .map(|value| Val::Unsigned { value: value, radix: radix })
         .ok_or(Error::InvalidData(format!["Cannot convert {} to u64", x]))
    }

    pub fn str<Str>(s: Str) -> Val
        where Str: Into<String>
    {
        Val::String(s.into())
    }

    pub fn pretty_print(self, indent_level:usize) -> String {
        match self {
            Val::Subpacket(values) => {
                let indent:String = std::iter::repeat(" ").take(2 * indent_level).collect();

                "\n".to_string() + &values.into_iter()
                    .map(|(k,v)| {
                        format!["{}{}: ", indent, k]
                        + &match v {
                            Ok(val) => val.pretty_print(indent_level + 1),
                            Err(e) => format!["<< Error: {} >>", e],
                        }
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            }

            Val::Signed(i) => format!["{}", i],
            Val::Unsigned { value, radix } => match radix {
                2 => format!["{:b}", value],
                8 => format!["{:o}", value],
                10 => format!["{}", value],
                16 => format!["0x{:x}", value],
                _ => format!["{:?} (base {})", value, radix],
            },
            Val::Enum(i, s) => format!["{} ({})", i, s],
            Val::String(ref s) => format!["{}", s],
            Val::Address { ref encoded, .. } => format!["{}", encoded],
            Val::Bytes(ref bytes) => {
                let mut s = format!["{} B [", bytes.len()];

                let to_print:&[u8] =
                    if bytes.len() < 16 { bytes }
                    else { &bytes[..16] }
                    ;

                for b in to_print {
                    s = s + &format![" {:02x}", b];
                }

                if bytes.len() > 16 {
                    s = s + " ...";
                }

                s + " ]"
            },
            Val::Warning(w) => format!["Warning: {}", w],
            Val::Error(e) => format!["Error: {}", e],
        }
    }
}


/// An error related to packet dissection (underflow, bad value, etc.).
#[derive(Clone, Debug)]
pub enum Error {
    Underflow { expected: usize, have: usize, subject: String, },
    InvalidData(String),
}

impl Error
{
    fn underflow<T,S>(expected: usize, have: usize, subject: S) -> Result<T>
        where S: Into<String>
    {
        Err(Error::Underflow { expected: expected, have: have, subject: subject.into() })
    }

    fn inval<T,S>(message: S) -> Result<T>
        where S: Into<String>
    {
        Err(Error::InvalidData(message.into()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f : &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::Underflow { expected: expect, have, ref subject } =>
                write![f, "underflow: {} expected {} B, have {}", subject, expect, have],

            &Error::InvalidData(ref msg) => write![f, "invalid data: {}", msg],
        }
    }
}

/// The result of a dissection function.
pub type Result<T=Val> = ::std::result::Result<T,Error>;


/// A named value-or-error.
pub type NamedValue = (&'static str,Result<Val>);


/// Parse a signed integer of a given endianness from a byte buffer.
///
/// The size of the buffer will be used to determine the size of the integer
/// that should be parsed (i8, i16, i32 or i64).
pub fn signed<T, E>(buffer: &[u8]) -> Result<T>
    where T: num::FromPrimitive, E: ByteOrder
{
    let len = buffer.len();
    let conversion_error = "Failed to convert integer";

    match len {
        1 => T::from_u8(buffer[0]).ok_or(conversion_error),
        2 => T::from_i16(E::read_i16(buffer)).ok_or(conversion_error),
        4 => T::from_i32(E::read_i32(buffer)).ok_or(conversion_error),
        8 => T::from_i64(E::read_i64(buffer)).ok_or(conversion_error),
        _ => Err("Invalid integer size"),
    }
    .map_err(|s| format!["{} ({} B)", s, len])
    .map_err(Error::InvalidData)
}

/// Parse an unsigned integer of a given endianness from a byte buffer.
///
/// The size of the buffer will be used to determine the size of the integer
/// that should be parsed (u8, u16, u32 or u64).
pub fn unsigned<T, E>(buffer: &[u8]) -> Result<T>
    where T: num::FromPrimitive, E: ByteOrder
{
    let len = buffer.len();
    let conversion_error = "Failed to convert {} B integer";

    match len {
        1 => T::from_u8(buffer[0]).ok_or(conversion_error),
        2 => T::from_u16(E::read_u16(buffer)).ok_or(conversion_error),
        4 => T::from_u32(E::read_u32(buffer)).ok_or(conversion_error),
        8 => T::from_u64(E::read_u64(buffer)).ok_or(conversion_error),
        _ => Err("Invalid integer size: {}"),
    }
    .map_err(|s| format!["{} ({} B)", s, len])
    .map_err(Error::InvalidData)
}


/// Dissector of last resort: store raw bytes without interpretation.
pub struct RawBytes {
    short_name: &'static str,
    full_name: &'static str,
}

impl RawBytes {
    /// Convenience function to wrap `String::from` and `Box::new`.
    fn boxed(short_name: &'static str, full_name: &'static str) -> Box<RawBytes>
    {
        Box::new(RawBytes {
            short_name: short_name,
            full_name: full_name,
        })
    }

    fn unknown_protocol(description: &'static str) -> Box<RawBytes> {
        Box::new(RawBytes {
            short_name: "UNKNOWN",
            full_name: description,
        })
    }
}

impl Protocol for RawBytes {
    fn short_name(&self) -> &'static str { self.short_name }
    fn full_name(&self) -> &'static str { self.full_name }

    fn dissect(&self, data: &[u8]) -> Result {
        Ok(Val::Subpacket(
            vec![("raw data", Ok(Val::Bytes(data.to_vec())))]
        ))
    }
}


pub mod ethernet;
pub mod ip;
