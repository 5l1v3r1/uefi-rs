//! Facilities for handling UEFI strings
//!
//! UEFI uses UCS-2 and Latin-1 strings with C / Windows conventions. This
//! module provides facilities for handling such strings safely and converting
//! them to and from Rust strings.

use super::chars::{Char16, Char8, Character};
use core::result::Result;
use core::slice;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// Generalization of `std::ffi::CStr` to UEFI use cases
///
/// This type is heavily inspired by `std::ffi::CStr`, but extended to support
/// UEFI peculiarities such as coexistence of multiple text encoding and UCS-2.
///
/// You should refer to the documentation of `std::ffi::CStr` for more details
/// on the overall semantics. This module will only summarize them, and explain
/// where we diverge from them.
pub struct CStr<Char: Character>([Char]);

/// Errors which can occur during checked [uN] -> CStrN conversions
#[derive(Debug)]
pub enum FromIntsWithNulError {
    /// An invalid character was encountered before the end of the slice
    InvalidChar(usize),

    /// A null character was encountered before the end of the slice
    InteriorNul(usize),

    /// The slice was not null-terminated
    NotNulTerminated,
}

impl<Char: Character> CStr<Char> {
    /// Wraps a raw UEFI string with a safe C string wrapper
    pub unsafe fn from_ptr<'ptr>(ptr: *const Char) -> &'ptr Self {
        let mut len = 0;
        while *ptr.add(len) != Char::NUL {
            len += 1
        }
        let ptr = ptr as *const Char::IntRepr;
        Self::from_ints_with_nul_unchecked(slice::from_raw_parts(ptr, len + 1))
    }

    /// Creates a C string wrapper from a nul-terminated slice of integers
    ///
    /// Unlike traditional `CStr::from_bytes_with_nul`, this function also
    /// checks character validity, as needed when handling UCS-2 data.
    pub fn from_ints_with_nul(codes: &[Char::IntRepr]) -> Result<&Self, FromIntsWithNulError> {
        for (pos, &code) in codes.iter().enumerate() {
            match Char::try_from(code) {
                // FIXME: Workaround for lack of associated consts in patterns
                Ok(c) if c == Char::NUL => {
                    if pos != codes.len() - 1 {
                        return Err(FromIntsWithNulError::InteriorNul(pos));
                    } else {
                        return Ok(unsafe { Self::from_ints_with_nul_unchecked(codes) });
                    }
                }
                Err(_) => {
                    return Err(FromIntsWithNulError::InvalidChar(pos));
                }
                _ => {}
            }
        }
        Err(FromIntsWithNulError::NotNulTerminated)
    }

    /// Unsafely creates a C string wrapper from a u16 slice.
    pub unsafe fn from_ints_with_nul_unchecked(codes: &[Char::IntRepr]) -> &Self {
        &*(codes as *const [Char::IntRepr] as *const Self)
    }

    /// Returns the inner pointer to this C string
    pub fn as_ptr(&self) -> *const Char {
        self.0.as_ptr()
    }

    /// Converts this C string to a slice of integers
    pub fn to_ints_slice(&self) -> &[Char::IntRepr] {
        let chars = self.to_ints_slice_with_nul();
        &chars[..chars.len() - 1]
    }

    /// Converts this C string to an int slice containing the trailing 0 char
    pub fn to_ints_slice_with_nul(&self) -> &[Char::IntRepr] {
        unsafe { &*(&self.0 as *const [Char] as *const [Char::IntRepr]) }
    }

    // Iterate over the code points of this string
    // FIXME: Remove this API and re-express everything in terms of Char instead.
    //        Consider renaming Char -> CharType along the way
    fn char_indices(&self) -> impl DoubleEndedIterator + Iterator<Item=(usize, Char)> + '_ {
        self.0[..self.0.len()-1].iter().cloned().map(|c| c.into()).enumerate()
    }
}

/// A Latin-1 null-terminated string
pub type CStr8 = CStr<Char8>;

/// An UCS-2 null-terminated string
pub type CStr16 = CStr<Char16>;

/// Things that can go wrong during Rust -> UEFI string conversions
#[derive(Debug)]
pub enum StrEncodeError {
    /// Not enough output buffer space to encode any input grapheme
    ///
    /// This can happen if the input features many combining characters (think
    /// about "Zalgo" Unicode abuse) or if the output buffer is very small.
    ///
    /// Unicode's UAX #15 recommends at least 32 code points of storage.
    BufferTooSmall,

    /// The input string contains a character which has no equivalent in the
    /// output encoding at the specified index.
    UnsupportedChar(usize),

    /// The input string contains an inner NUL characters at the specified index
    ///
    /// This is illegal in UEFI's C-style NUL-terminated strings.
    InteriorNul(usize),
}

/// Encode a Rust string into an UEFI string of the specified character type
///
/// The output characters will be stored into a user-provided buffer, followed
/// by a NUL terminator. If that buffer is not large enough, the output string
/// will be truncated on the previous grapheme cluster boundary.
///
/// As an output, this function returns an UEFI-compatible &CStr and the part of
/// the input string that was not converted (if any). Failure to convert any
/// character from a non-empty string is reported as an error in order to
/// prevent accidental endless loops on the caller side.
pub fn encode<'buf, 'inp, Char: Character>(
    input: &'inp str,
    buffer: &'buf mut [Char::IntRepr],
) -> Result<(&'buf CStr<Char>, Option<&'inp str>), StrEncodeError> {
    // Save up a char at the end of the buffer for the terminating NUL
    let buffer_capacity = buffer.len() - 1;

    // We will convert the input with grapheme cluster granularity
    let (mut parsed_input_len, mut encoded_output_len, mut output_idx) = (0, 0, 0);
    'graphemes: for (grapheme_idx, grapheme) in input.grapheme_indices(true) {
        // Iterate over this input grapheme's code points
        for (input_idx, input_char) in grapheme.char_indices() {
            // Reject NUL characters, which are not supported by C strings
            if input_char == '\0' {
                return Err(StrEncodeError::InteriorNul(input_idx));
            }

            // Translate Rust line endings to UEFI line endings by adding a '\r'
            if input_char == '\n' {
                if output_idx < buffer_capacity {
                    buffer[output_idx] = Char::CARRIAGE_RETURN.into();
                    output_idx += 1;
                } else {
                    break 'graphemes;
                }
            }

            // Convert the input character to the output encoding
            let output_char = Char::try_from(input_char)
                .map_err(|_| StrEncodeError::UnsupportedChar(input_idx))?;

            // Write the converted code point to the buffer, or terminate the
            // loop if we have exhausted the available buffer capacity.
            if output_idx < buffer_capacity {
                buffer[output_idx] = output_char.into();
                output_idx += 1;
            } else {
                break 'graphemes;
            }
        }

        // Every time we get through a grapheme cluster, advance public cursors
        parsed_input_len = grapheme_idx + grapheme.len();
        encoded_output_len = output_idx;
    }

    // Keep track of leftover input
    let remaining_input = if parsed_input_len < input.len() {
        Some(&input[parsed_input_len..])
    } else {
        None
    };

    // Treat failure to make any progress as an error
    if (parsed_input_len == 0) && remaining_input.is_some() {
        return Err(StrEncodeError::BufferTooSmall);
    }

    // Construct the output
    buffer[encoded_output_len] = Char::NUL.into();
    let output = unsafe { CStr::from_ints_with_nul_unchecked(buffer) };
    Ok((output, remaining_input))
}

/// Things that can go wrong during UEFI -> Rust string conversions
#[derive(Debug)]
enum StrDecodeError {
    /// Not enough output buffer space to encode any input grapheme
    ///
    /// This can happen if the input features many combining characters (think
    /// about "Zalgo" Unicode abuse) or if the output buffer is very small.
    ///
    /// Unicode's UAX #15 recommends at least 32 code points of storage, but due
    /// to limitations of our current Unicode segmentation algorithm, you should
    /// provide space for two grapheme clusters. In UTF-8, that's 256 bytes.
    BufferTooSmall,
}

// Decode an UEFI string of the specified character type into a Rust string
//
// The output characters will be stored into a user-provided buffer. If that
// buffer is not large enough, the output string will be truncated on the
// previous grapheme cluster boundary.
//
// As an output, this function returns a Rust &str and the part of the input
// UEFI string that was not converted (if any). Failure to convert any character
// from a non-empty string is reported as an error in order to prevent
// accidental endless loops on the caller side.
//
// FIXME: Clarify and finalize this documentation
fn decode<'buf, 'inp, Char: Character>(
    input: &'inp CStr<Char>,
    buffer: &'buf mut [u8],
) -> Result<(&'buf str, Option<&'inp CStr<Char>>), StrDecodeError> {
    // Convert as many chars as possible to UTF-8 + Rust line feeds
    let mut output_idx = 0;
    let (mut after_carriage_return, mut truncated) = (false, false);
    let mut input_iter = input.char_indices();
    while let Some((_, ch)) = input_iter.next() {
        // Convert the code point to a Rust char
        let ch: char = ch.into();

        // Try to add this code point to our UTF-8 buffer
        if buffer.len() - output_idx >= ch.len_utf8() {
            output_idx += ch.encode_utf8(&mut buffer[output_idx..]).len();
        } else {
            truncated = true;
            break;
        }

        // Convert CR/LF sequences to Rust-style line feeds
        if after_carriage_return && ch == '\n' {
            buffer[output_idx-2] = b'\n';
            output_idx -= 1;
        }
        after_carriage_return = ch == '\r';
    }

    // Build a Rust string from the UTF-8 sequence that we just encoded
    let mut output = core::str::from_utf8(&buffer[..output_idx])
        .expect("The decoded UTF-8 should be correct");

    // If we have consumed all the input, we can stop there
    if !truncated {
        return Ok((output, None));
    }

    // If some input could not be converted, we must backtrack on the last
    // grapheme cluster from the input, as it may have been truncated.
    //
    // We don't know the UTF-8 length of the remaining input, so we must lie
    // about it, but it's okay as the GraphemeCursor really only wants to know
    // whether we have access to the full string or not...
    let mut cursor = GraphemeCursor::new(output_idx, output_idx + 1, true);
    let last_grapheme_idx = cursor.prev_boundary(output, 0)
        .expect("No error condition of GraphemeCursor seems reachable here")
        .unwrap_or(0);
    let discarded_output_chars = output[..last_grapheme_idx].chars().count();
    output = &output[..last_grapheme_idx];

    // Abort if we only converted a single grapheme cluster (=> no progress!)
    if output.is_empty() {
        return Err(StrDecodeError::BufferTooSmall);
    }

    // Extract the remaining input, including the characters that we just
    // discarded from the output. Beware that an input CR/LF sequence only maps
    // to a single 
    // FIXME: Figure out how to do this when we have carried out lossy CRLF -> LF transforms!
    // IDEA: Backtrach on the input_iter?
    //       Can use # of output chars, but only after fixing CR/LFs
    unimplemented!()
}