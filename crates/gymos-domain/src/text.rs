//! User-entered text: names and notes.
//!
//! Text is trimmed and length-checked in characters, not bytes. Control
//! characters are rejected (notes may keep newlines and tabs), so stored text
//! can't smuggle terminal escapes or invisible content into exports or logs.

use thiserror::Error;

/// Why a piece of text was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TextError {
    /// Empty, or only whitespace.
    #[error("text must not be empty")]
    Empty,
    /// Longer than the limit, in characters.
    #[error("text must be at most {0} characters")]
    TooLong(usize),
    /// Contains a control character that isn't allowed here.
    #[error("text contains a control character")]
    ControlCharacter,
}

fn clean(input: &str, max_chars: usize, allow_line_breaks: bool) -> Result<String, TextError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(TextError::Empty);
    }
    if trimmed.chars().count() > max_chars {
        return Err(TextError::TooLong(max_chars));
    }
    let allowed = |c: char| allow_line_breaks && matches!(c, '\n' | '\t');
    if trimmed.chars().any(|c| c.is_control() && !allowed(c)) {
        return Err(TextError::ControlCharacter);
    }
    Ok(trimmed.to_owned())
}

/// A short, single-line name: an exercise, a session title, a template.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Name(String);

impl Name {
    /// The longest name accepted, in characters.
    pub const MAX_CHARS: usize = 100;

    /// Builds a name from user input, trimming surrounding whitespace.
    ///
    /// # Errors
    ///
    /// [`TextError`] if the name is empty, too long or contains any control
    /// character (including newlines).
    pub fn new(input: &str) -> Result<Self, TextError> {
        clean(input, Self::MAX_CHARS, false).map(Self)
    }

    /// The name as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A free-text note on a set, exercise or session. May span several lines.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Note(String);

impl Note {
    /// The longest note accepted, in characters.
    pub const MAX_CHARS: usize = 2_000;

    /// Builds a note from user input, trimming surrounding whitespace.
    ///
    /// # Errors
    ///
    /// [`TextError`] if the note is empty, too long or contains a control
    /// character other than newline or tab.
    pub fn new(input: &str) -> Result<Self, TextError> {
        clean(input, Self::MAX_CHARS, true).map(Self)
    }

    /// The note as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            Name::new("  Bench Press \n").unwrap().as_str(),
            "Bench Press"
        );
    }

    #[test]
    fn rejects_blank_text() {
        assert_eq!(Name::new("   "), Err(TextError::Empty));
        assert_eq!(Note::new(""), Err(TextError::Empty));
    }

    #[test]
    fn counts_characters_not_bytes() {
        let hundred_accents = "é".repeat(Name::MAX_CHARS);
        assert!(Name::new(&hundred_accents).is_ok());
        assert_eq!(
            Name::new(&format!("{hundred_accents}é")),
            Err(TextError::TooLong(Name::MAX_CHARS))
        );
    }

    #[test]
    fn names_are_single_line_but_notes_may_wrap() {
        assert_eq!(Name::new("Push\nA"), Err(TextError::ControlCharacter));
        assert!(Note::new("felt strong\n\tlast set grindy").is_ok());
    }

    #[test]
    fn rejects_escape_sequences() {
        assert_eq!(
            Note::new("hi \u{1b}[31mred"),
            Err(TextError::ControlCharacter)
        );
    }
}
