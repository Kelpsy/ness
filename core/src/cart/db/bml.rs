use core::{
    fmt::{self, Display},
    iter::Enumerate,
    str::Lines,
};
use std::{borrow::Cow, error::Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    IndentedRootNode,
    InvalidValue,
    UnescapedMultilineValue,
    InvalidAttribute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub line: usize,
}

impl Error for ParseError {}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}",
            match self.kind {
                ParseErrorKind::IndentedRootNode => "Indented BML root node",
                ParseErrorKind::InvalidValue => "Invalid BML value",
                ParseErrorKind::UnescapedMultilineValue => "Unescaped BML multi-line value",
                ParseErrorKind::InvalidAttribute => "Invalid BML attribute",
            },
            self.line
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node<'a> {
    pub name: &'a str,
    pub value: Option<Cow<'a, str>>,
    pub attrs: Vec<Node<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ValueAttrError<'a> {
    Missing,
    MissingValue,
    UnexpectedAttrs(Vec<Node<'a>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum MarkerAttrError<'a> {
    UnexpectedValue(Cow<'a, str>),
    UnexpectedAttrs(Vec<Node<'a>>),
}

impl<'a> Node<'a> {
    pub(super) fn remove_attr(&mut self, name: &str) -> Option<Node<'a>> {
        self.attrs
            .iter()
            .position(|attr| attr.name == name)
            .map(|i| self.attrs.remove(i))
    }

    pub(super) fn remove_value_attr(
        &mut self,
        name: &str,
    ) -> Result<Cow<'a, str>, ValueAttrError<'a>> {
        match self.remove_attr(name) {
            Some(attr) => {
                if attr.attrs.is_empty() {
                    if let Some(value) = attr.value {
                        Ok(value)
                    } else {
                        Err(ValueAttrError::MissingValue)
                    }
                } else {
                    Err(ValueAttrError::UnexpectedAttrs(attr.attrs))
                }
            }
            None => Err(ValueAttrError::Missing),
        }
    }

    pub(super) fn remove_marker(&mut self, name: &str) -> Result<bool, MarkerAttrError<'a>> {
        match self.remove_attr(name) {
            Some(attr) => {
                if let Some(value) = attr.value {
                    Err(MarkerAttrError::UnexpectedValue(value))
                } else if !attr.attrs.is_empty() {
                    Err(MarkerAttrError::UnexpectedAttrs(attr.attrs))
                } else {
                    Ok(true)
                }
            }
            None => Ok(false),
        }
    }
}

struct Parser<'a> {
    lines: Enumerate<Lines<'a>>,
    cur_line: Option<(usize, &'a str)>,
}

impl<'a> Parser<'a> {
    fn consume_line(&mut self) {
        self.cur_line = self.lines.next();
    }

    fn is_valid_name_char(char: char) -> bool {
        matches!(char, 'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.')
    }

    fn line_should_be_skipped(line: &str) -> bool {
        line.is_empty() || line.starts_with("//")
    }

    fn parse_indent(line: &mut &str) -> usize {
        let prev = line.len();
        *line = line.trim_start();
        prev - line.len()
    }

    fn parse_kv(line_i: usize, mut line: &str) -> Result<(Node, &str), ParseError> {
        if let Some((name_end, _)) = line
            .char_indices()
            .find(|(_, char)| !Self::is_valid_name_char(*char))
        {
            let name = &line[..name_end];
            line = &line[name_end..];
            let (value, remaining) = if let Some(line) = line.strip_prefix(':') {
                (line.trim(), "")
            } else if let Some(line) = line.strip_prefix("=\"") {
                let end_index = line.find('"').ok_or(ParseError {
                    kind: ParseErrorKind::UnescapedMultilineValue,
                    line: line_i,
                })?;
                (&line[..end_index], &line[end_index + 1..])
            } else if let Some(line) = line.strip_prefix('=') {
                let end_index = match line.char_indices().find(|(_, char)| char.is_whitespace()) {
                    Some((i, _)) => i,
                    None => line.len(),
                };
                let value = &line[..end_index];
                if value.contains('"') {
                    return Err(ParseError {
                        kind: ParseErrorKind::InvalidValue,
                        line: line_i,
                    });
                }
                (value, &line[end_index..])
            } else {
                return Ok((
                    Node {
                        name,
                        value: None,
                        attrs: vec![],
                    },
                    line,
                ));
            };
            Ok((
                Node {
                    name,
                    value: Some(Cow::Borrowed(value)),
                    attrs: vec![],
                },
                remaining,
            ))
        } else {
            Ok((
                Node {
                    name: line.trim_end(),
                    value: None,
                    attrs: vec![],
                },
                "",
            ))
        }
    }

    fn parse_node(
        &mut self,
        indent: usize,
        line_i: usize,
        line: &'a str,
    ) -> Result<Node<'a>, ParseError> {
        let (mut node, mut line) = Self::parse_kv(line_i, line)?;
        while !line.is_empty() {
            if Self::parse_indent(&mut line) == 0 && !line.is_empty() {
                return Err(ParseError {
                    kind: ParseErrorKind::InvalidAttribute,
                    line: line_i,
                });
            }
            if Self::line_should_be_skipped(line) {
                break;
            }
            let (attr, remaining) = Self::parse_kv(line_i, line)?;
            node.attrs.push(attr);
            line = remaining;
        }
        self.consume_line();
        while let Some((line_i, mut line)) = self.cur_line {
            let line_indent = Self::parse_indent(&mut line);
            if Self::line_should_be_skipped(line) {
                self.consume_line();
                continue;
            }
            if line_indent <= indent {
                break;
            }
            if line.starts_with(':') {
                let value = line.trim_start_matches(':').trim_start();
                node.value
                    .get_or_insert(Cow::Borrowed(""))
                    .to_mut()
                    .push_str(value);
                self.consume_line();
                continue;
            }
            node.attrs.push(self.parse_node(line_indent, line_i, line)?);
        }
        Ok(node)
    }

    fn parse(input: &str) -> Result<Vec<Node>, ParseError> {
        let mut lines = input.lines().enumerate();
        let mut parser = Parser {
            cur_line: lines.next(),
            lines,
        };
        let mut result = vec![];
        while let Some((line_i, mut line)) = parser.cur_line {
            let indent = Self::parse_indent(&mut line);
            if Self::line_should_be_skipped(line) {
                parser.consume_line();
                continue;
            }
            if indent != 0 {
                return Err(ParseError {
                    kind: ParseErrorKind::IndentedRootNode,
                    line: line_i,
                });
            }
            result.push(parser.parse_node(indent, line_i, line)?);
        }
        Ok(result)
    }
}

pub(super) fn parse(input: &str) -> Result<Vec<Node>, ParseError> {
    Parser::parse(input)
}
