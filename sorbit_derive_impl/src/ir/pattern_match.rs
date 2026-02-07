#![allow(unused)]

use itertools::Itertools;
use std::collections::HashMap;

#[cfg(test)]
macro_rules! assert_matches {
    ($operation:expr, $pattern:expr) => {
        match crate::ir::pattern_match::matches($operation, $pattern) {
            Ok(_) => (),
            Err(message) => assert!(false, "{message}"),
        }
    };
}

#[cfg(test)]
pub(crate) use assert_matches;

#[cfg(test)]
pub fn matches(operation: impl AsRef<str>, pattern: impl AsRef<str>) -> Result<(), String> {
    let op_tokens = tokenize(&operation)?;
    let pat_tokens = tokenize(&pattern)?;
    let mut sym_pat_to_op = HashMap::new();
    let mut sym_op_to_pat = HashMap::new();
    for (token_idx, (op_token, pat_token)) in op_tokens.iter().zip(pat_tokens.iter()).enumerate() {
        use Token::*;
        let is_content_same = match (op_token, pat_token) {
            (Empty, Empty) => Ok(()),
            (Pipe, Pipe) => Ok(()),
            (BracketOpen, BracketOpen) => Ok(()),
            (BracketClose, BracketClose) => Ok(()),
            (Symbol(o), Symbol(p)) if o == p => Ok(()),
            (Value(o), Value(p)) => {
                if sym_pat_to_op.get(p) == Some(&o) && sym_op_to_pat.get(o) == Some(&p) {
                    Ok(())
                } else if sym_pat_to_op.get(p) == None && sym_op_to_pat.get(o) == None {
                    sym_pat_to_op.insert(p, o);
                    sym_op_to_pat.insert(o, p);
                    Ok(())
                } else {
                    Err((token_idx, &Token::Value(sym_pat_to_op.get(p).unwrap_or(&p).to_string())))
                }
            }
            (_, token) => Err((token_idx, token)),
        };
        if let Err((token_idx, expected)) = is_content_same {
            let start_idx = std::cmp::max(6, token_idx) - 6;
            let matching_section = op_tokens[start_idx..token_idx].iter().join(" ");
            let following_op_section =
                op_tokens[token_idx + 1..std::cmp::min(op_tokens.len(), token_idx + 6)].iter().join(" ");
            let following_pat_section =
                pat_tokens[token_idx + 1..std::cmp::min(pat_tokens.len(), token_idx + 6)].iter().join(" ");
            let found = op_tokens[token_idx].to_string();
            let padding = String::from_iter(std::iter::repeat(' ').take(matching_section.len()));
            let caret = String::from_iter(std::iter::repeat('^').take(found.len()));
            let message = format!(
                "operation does not match pattern:\n   expected: {matching_section} {expected} {following_pat_section}\n   found:    {matching_section} {found} {following_op_section}\n             {padding} {caret}",
            );
            return Err(message);
        }
    }
    if op_tokens.len() != pat_tokens.len() {
        return Err(format!(
            "operation differs from pattern in length\noperation:\n{}\npattern:\n{}",
            operation.as_ref(),
            pattern.as_ref()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[derive(Debug, Clone)]
enum Token {
    Empty,
    Pipe,
    BracketOpen,
    BracketClose,
    Symbol(String),
    Value(String),
}

#[cfg(test)]
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Empty => write!(f, ""),
            Token::Pipe => write!(f, "|"),
            Token::BracketOpen => write!(f, "["),
            Token::BracketClose => write!(f, "]"),
            Token::Symbol(s) => write!(f, "{s}"),
            Token::Value(s) => write!(f, "%{s}"),
        }
    }
}

#[cfg(test)]
fn tokenize(pattern: impl AsRef<str>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    for char in pattern.as_ref().chars() {
        match char {
            ' ' | '\t' | '\n' | ',' => tokens.push(Token::Empty),
            '|' => tokens.push(Token::Pipe),
            '[' => tokens.push(Token::BracketOpen),
            ']' => tokens.push(Token::BracketClose),
            '%' => tokens.push(Token::Value(String::new())),
            ch => match tokens.last_mut() {
                Some(Token::Symbol(s)) => s.push(ch),
                Some(Token::Value(s)) => s.push(ch),
                _ => tokens.push(Token::Symbol(ch.into())),
            },
        };
    }
    tokens.retain(|token| !matches!(token, Token::Empty));
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    #[test]
    fn matches_same_value() {
        let operation = "%a %a";
        let pattern = "%a %a";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_value() {
        let operation = "%a %a";
        let pattern = "%a %b";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_value_reverse() {
        let operation = "%a %b";
        let pattern = "%a %a";
        assert_matches!(operation, pattern);
    }

    #[test]
    fn matches_same_symbol() {
        let operation = "a";
        let pattern = "a";
        assert_matches!(operation, pattern);
    }

    #[test]
    #[should_panic]
    fn matches_different_symbol() {
        let operation = "a";
        let pattern = "b";
        assert_matches!(operation, pattern);
    }
}
