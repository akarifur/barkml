use logos::Lexer;

use crate::ast::Location;
use crate::error;

use super::lexer::Token;
use crate::Result;

pub trait Read<'source> {
    fn peek(&mut self) -> Result<Option<Token>>;
    fn next(&mut self) -> Result<Option<Token>>;
    fn discard(&mut self);
    fn location(&mut self) -> Location;
    /// Location of the true end of input, derived from the complete source
    /// (not the last consumed token). Valid once the stream is exhausted.
    fn eof_location(&mut self) -> Location;
}

pub struct TokenReader<'source> {
    pub module_name: String,
    pub lexer: Lexer<'source, Token>,
    pub(crate) peeked: Option<Option<Token>>,
    pub location: Location,
}

impl<'source> TokenReader<'source> {
    fn error_token(&self) -> Token {
        // Lexing errors (e.g. a stray `$` from removed control statements)
        // surface as an Error token so the parser can report them with
        // source context.
        let span = self.lexer.span();
        Token::Error(Location {
            module: Some(self.module_name.clone()),
            line: self.location.line,
            column: span.start.saturating_sub(self.location.column),
            source_text: Some(self.lexer.slice().to_string()),
            length: span.end - span.start,
            file_path: self.location.file_path.clone(),
        })
    }
}

impl<'source> Read<'source> for TokenReader<'source> {
    fn peek(&mut self) -> Result<Option<Token>> {
        if self.peeked.is_none() {
            self.peeked = Some(match self.lexer.next() {
                Some(Ok(token)) => Some(token),
                // Undifferentiated lex failures (e.g. a stray `$` from removed
                // control statements) surface as an Error token so the parser
                // can report them with source context; specific failures
                // (unterminated strings, bad escapes) propagate directly.
                Some(Err(error::Error::Unknown)) => Some(self.error_token()),
                Some(Err(e)) => return Err(e),
                None => None,
            });
        }
        Ok(self.peeked.as_ref().unwrap().clone())
    }

    fn next(&mut self) -> Result<Option<Token>> {
        let result = match self.peeked.take() {
            Some(buffered) => buffered,
            None => match self.lexer.next() {
                Some(Ok(token)) => Some(token),
                Some(Err(error::Error::Unknown)) => Some(self.error_token()),
                Some(Err(e)) => return Err(e),
                None => None,
            },
        };

        if let Some(token) = result {
            // Update location with more detailed information
            let mut loc = token.location(Some(self.module_name.clone()));

            // Preserve file path if it exists in the current location
            if let Some(file_path) = &self.location.file_path {
                loc.file_path = Some(file_path.clone());
            }

            self.location = loc;
            Ok(Some(token.clone()))
        } else {
            Ok(None)
        }
    }

    fn discard(&mut self) {
        let _ = self.next();
    }

    fn location(&mut self) -> Location {
        self.location.clone()
    }

    fn eof_location(&mut self) -> Location {
        let end = self.lexer.source().len();
        Location {
            module: Some(self.module_name.clone()),
            line: self.lexer.extras.line,
            column: end.saturating_sub(self.lexer.extras.column),
            source_text: None,
            length: 0,
            file_path: self.location.file_path.clone(),
        }
    }
}
