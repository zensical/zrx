// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Segment set conversions.

use std::iter::Peekable;

use crate::id::specificity::tokens::{AsTokens, Token, Tokens};

use super::atom::{Atom, Character, Wildcard};
use super::segments::Segments;
use super::Segment;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion to [`Segments`].
pub trait ToSegments {
    /// Converts to a segments set.
    fn to_segments(&self) -> Segments<'_>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> ToSegments for T
where
    T: AsTokens,
{
    #[inline]
    fn to_segments(&self) -> Segments<'_> {
        parse(&mut self.as_tokens().peekable(), false)
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Peekable iterator over tokens.
type Iter<'a> = Peekable<Tokens<'a>>;

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Parses a sequence of tokens into a segment set.
fn parse<'a>(iter: &mut Iter<'a>, group: bool) -> Segments<'a> {
    let mut segments = Vec::new();

    // Consume tokens until comma or group end if in group
    while let Some(token) = iter.peek() {
        match token {
            Token::Comma | Token::GroupEnd if group => break,
            _ => segments.push(parse_segment(iter, group)),
        }
    }

    // Return segment set
    Segments::from_iter(segments)
}

/// Parses a sequence of tokens into a segment.
fn parse_segment<'a>(iter: &mut Iter<'a>, group: bool) -> Segment<'a> {
    let mut atoms = Vec::new();

    // Consume tokens until comma or group end if in group
    while let Some(token) = iter.peek() {
        if group && matches!(token, Token::Comma | Token::GroupEnd) {
            break;
        }

        // Consume tokens until separator
        atoms.push(match iter.next().expect("invariant") {
            Token::Any => Atom::Wildcard(Wildcard::Character),
            Token::Star => Atom::Wildcard(Wildcard::Sequence),
            Token::StarStar => Atom::Wildcard(Wildcard::Traversal),
            Token::CharacterStart => Atom::Character(parse_character(iter)),
            Token::GroupStart => Atom::Group(parse_group(iter)),
            Token::Separator => break,
            other => Atom::Literal(other.as_str()),
        });
    }

    // Return segment
    Segment::from_iter(atoms)
}

/// Parses a sequence of tokens into a character class.
fn parse_character<'a>(iter: &mut Iter<'a>) -> Character<'a> {
    let mut values = Vec::new();

    // Consume negation marker if present
    let negate = match iter.next() {
        Some(Token::Exclamation) => true,
        None => false,
        Some(token) => {
            values.push(token.as_str());
            false
        }
    };

    // Consume tokens until character class end
    for token in iter.by_ref() {
        values.push(match token {
            Token::CharacterEnd => break,
            other => other.as_str(),
        });
    }

    // Return character class
    Character { negate, values }
}

/// Parses a sequence of tokens into a group of segments.
fn parse_group<'a>(iter: &mut Iter<'a>) -> Vec<Segments<'a>> {
    let mut group = Vec::new();
    loop {
        group.push(Segments::from_iter(parse(iter, true)));
        match iter.peek() {
            Some(Token::Comma) => iter.next(),
            Some(Token::GroupEnd) => {
                iter.next();
                break;
            }
            _ => break,
        };
    }

    // Return group of segments
    group
}
