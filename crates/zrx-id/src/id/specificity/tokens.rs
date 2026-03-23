// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the `Software`), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED `AS IS`, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Iterator over tokens.

mod convert;

pub use convert::AsTokens;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token<'a> {
    /// Literal - `foo`
    Literal(&'a str),
    /// Dot - `.`
    Dot,
    /// Any character - `?`
    Any,
    /// Single asterisk - `*`
    Star,
    /// Double asterisk - `**`
    StarStar,
    /// Character class start - `[`
    CharacterStart,
    /// Character class end - `]`
    CharacterEnd,
    /// Exclamation mark - `!`
    Exclamation,
    /// Alternate group start - `{`
    GroupStart,
    /// Alternate group end - `}`
    GroupEnd,
    /// Comma - `,`
    Comma,
    /// Separator - `/`
    Separator,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over tokens.
///
/// This data type provides an iterator over the tokens of a pattern string,
/// allowing to parse and analyze the structure of a [`Glob`][] to compute the
/// [`Specificity`][] for tie-breaking. Note that the validity of the pattern
/// is not checked, as this is the responsibility of the caller.
///
/// [`Glob`]: globset::Glob
/// [`Specificity`]: crate::id::specificity::Specificity
pub struct Tokens<'a> {
    /// Pattern.
    value: &'a str,
    /// Current index.
    index: usize,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Token<'a> {
    /// Returns the string representation.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        match self {
            Token::Literal(literal) => literal,
            Token::Dot => ".",
            Token::Any => "?",
            Token::Star => "*",
            Token::StarStar => "**",
            Token::CharacterStart => "[",
            Token::CharacterEnd => "]",
            Token::Exclamation => "!",
            Token::GroupStart => "{",
            Token::GroupEnd => "}",
            Token::Comma => ",",
            Token::Separator => "/",
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a> From<&'a str> for Tokens<'a> {
    /// Creates an iterator over the tokens of a pattern.
    #[inline]
    fn from(value: &'a str) -> Self {
        Self { value, index: 0 }
    }
}

// ----------------------------------------------------------------------------

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    /// Returns the next token.
    ///
    /// Note that this parser does not check the validity of a [`Glob`][] - it
    /// assumes that the pattern has been parsed and is considered valid. This
    /// means that specificity should only be computed for valid patterns, as
    /// invalid patterns may lead to unexpected results.
    ///
    /// [`Glob`]: globset::Glob
    fn next(&mut self) -> Option<Self::Item> {
        let value = self.value.as_bytes();

        // Handle end of pattern
        let start = self.index;
        if start == value.len() {
            return None;
        }

        // Handle current character
        self.index += 1;
        match value[start] {
            b'.' => Some(Token::Dot),
            b'?' => Some(Token::Any),
            b'[' => Some(Token::CharacterStart),
            b']' => Some(Token::CharacterEnd),
            b'!' => Some(Token::Exclamation),
            b'{' => Some(Token::GroupStart),
            b'}' => Some(Token::GroupEnd),
            b',' => Some(Token::Comma),
            b'/' => Some(Token::Separator),

            // Consume a `*` or `**`
            b'*' => {
                if self.index < value.len() && value[self.index] == b'*' {
                    self.index += 1;
                    Some(Token::StarStar)
                } else {
                    Some(Token::Star)
                }
            }

            // Consume a literal
            _ => {
                while self.index < value.len() {
                    if is_special(value[self.index]) {
                        break;
                    }
                    self.index += 1;
                }
                Some(Token::Literal(&self.value[start..self.index]))
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Returns whether the given character is a special character.
#[inline]
fn is_special(char: u8) -> bool {
    matches!(
        char,
        b'.' | b'?' | b'*' | b'[' | b']' | b'!' | b'{' | b'}' | b',' | b'/'
    )
}
