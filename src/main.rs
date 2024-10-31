use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use std::env;
use std::io;
use std::iter::Peekable;
use std::process;
use std::str::FromStr;

#[derive(Debug)]
enum RegexElement {
    Wildcard,
    Literal(char),
    Class(RegexClass),
    CharGroup {
        is_positive: bool,
        options: Vec<char>,
    },
    StartAnchor,
    EndAnchor,
    Quantifier {
        min: usize,
        max: Option<usize>,
        content: Box<RegexElement>,
    },
}

impl RegexElement {
    fn read<T: Iterator<Item = char>>(chars: &mut Peekable<T>) -> Result<Option<Self>> {
        let result = match chars.next() {
            Some('.') => RegexElement::Wildcard,
            Some('\\') => match chars.next() {
                Some('d') => RegexElement::Class(RegexClass::Digit),
                Some('w') => RegexElement::Class(RegexClass::Alphanumeric),
                Some(c) => bail!("Unknown escape sequence: \\{c}"),
                None => bail!("Expected character after '\\'"),
            },
            // FIXME: should fail if we reach the end of the string without closing ']'
            // FIXME: handle escape sequences inside char groups
            Some('[') => {
                let is_positive = chars.next_if_eq(&'^').is_none();
                RegexElement::CharGroup {
                    is_positive,
                    options: chars.take_while(|c| c != &']').collect(),
                }
            }
            Some('^') => RegexElement::StartAnchor,
            Some('$') => RegexElement::EndAnchor,
            Some(c) => RegexElement::Literal(c),
            None => return Ok(None),
        };
        let result = match chars.peek() {
            Some('+') => {
                chars.next();
                Self::Quantifier {
                    min: 1,
                    max: None,
                    content: Box::new(result),
                }
            }
            Some('*') => {
                chars.next();
                Self::Quantifier {
                    min: 0,
                    max: None,
                    content: Box::new(result),
                }
            }
            Some('?') => {
                chars.next();
                Self::Quantifier {
                    min: 0,
                    max: Some(1),
                    content: Box::new(result),
                }
            }
            Some(_) | None => result,
        };
        Ok(Some(result))
    }
    fn matches<'a>(&self, full_str: &'a str, start_index: usize) -> Option<&'a str> {
        let str = &full_str.get(start_index..).unwrap_or_default();
        println!("Trying to match {self:?} in {:?}", str);
        let matches: Option<&'a str> = match self {
            RegexElement::StartAnchor => {
                if start_index == 0 {
                    Some(Default::default())
                } else {
                    None
                }
            }
            RegexElement::EndAnchor => {
                if str.is_empty() {
                    Some(Default::default())
                } else {
                    None
                }
            }
            RegexElement::Wildcard => {
                if !str.is_empty() {
                    Some(&str[..1])
                } else {
                    None
                }
            }
            RegexElement::Literal(c) => {
                if str.starts_with(*c) {
                    Some(&str[..1])
                } else {
                    None
                }
            }
            RegexElement::Class(RegexClass::Digit) => {
                if str.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    Some(&str[..1])
                } else {
                    None
                }
            }
            RegexElement::Class(RegexClass::Alphanumeric) => {
                if str
                    .chars()
                    .next()
                    .map_or(false, |c| c.is_ascii_alphanumeric() || c == '_')
                {
                    Some(&str[..1])
                } else {
                    None
                }
            }
            RegexElement::CharGroup {
                is_positive,
                options,
            } => {
                if str
                    .chars()
                    .next()
                    .map_or(false, |c| options.contains(&c) == *is_positive)
                {
                    Some(&str[..1])
                } else {
                    None
                }
            }
            RegexElement::Quantifier { min, max, content } => {
                let mut end_index = 0;
                let mut match_count: usize = 0;
                while let Some(inner_match) = content.matches(str, end_index) {
                    match_count += 1;
                    end_index += inner_match.len();
                    if let Some(max) = max {
                        if *max == match_count {
                            break;
                        }
                    }
                }
                if match_count >= *min {
                    Some(&str[..end_index])
                } else {
                    None
                }
            }
        };
        // #[cfg(debug_assertions)]
        // {
        //     let str_end = iter.clone().collect::<String>();
        //     let str_match = &str_start[..str_start.len() - str_end.len()];
        //     println!("Matched {self:?} in {str_match:?}? {matches}. Remaining chars: {str_end:?}",);
        // }
        println!("Pattern {self:?} matched {matches:?} in {str:?}",);
        matches
    }
}

#[derive(Debug)]
enum RegexClass {
    Digit,
    Alphanumeric,
}

#[derive(Debug)]
struct Regex(Vec<RegexElement>);

impl Regex {
    fn matches(&self, s: &str) -> bool {
        'regex_loop: for start_index in 0..=s.len() {
            let mut start_index = start_index;
            println!("Trying to match {self:?} in {s:?} starting at {start_index}");
            for element in &self.0 {
                let matches = element.matches(s, start_index.min(s.len()));
                println!(
                    "Input {:?} matched {element:?}? {}",
                    &s[start_index..],
                    matches.is_some(),
                );
                if let Some(str_match) = matches {
                    start_index += str_match.len();
                } else {
                    start_index += 1;
                    continue 'regex_loop;
                }
            }
            return true;
        }
        false
    }
}

impl FromStr for Regex {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut chars = s.chars().peekable();
        let elements = std::iter::from_fn(move || RegexElement::read(&mut chars).transpose())
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| anyhow!("Failed to parse regex"))?;
        if elements.is_empty() {
            bail!("Empty regex");
        }
        println!("Parsed regex elements: {elements:?}");
        Ok(Self(elements))
    }
}

fn match_pattern(input_line: &str, pattern: &str) -> Result<bool> {
    Ok(Regex::from_str(pattern)?.matches(input_line))
}
// Usage: echo <input_text> | your_program.sh -E <pattern>

fn main() -> Result<()> {
    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }
    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();
    if match_pattern(&input_line, &pattern)? {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_quantifier() {
        let cases = [
            ("a*", "", true),
            ("a*", "a", true),
            ("a*", "aa", true),
            ("a*b", "aaa", false),
            ("a*b", "aaab", true),
            ("a*b", "b", true),
            ("a+", "", false),
            ("a+", "a", true),
            ("a+", "aa", true),
            ("a+b", "aaa", false),
            ("a+b", "aaab", true),
            ("a+b", "b", false),
            ("a?", "", true),
            ("a?", "a", true),
            ("a?", "aa", true),
            ("a?b", "aaa", false),
            // FIXME: this should be true
            // ("a?b", "aaab", true),
            ("a?b", "aaab", true),
            ("a?b", "b", true),
            ("ca+t", "caaats", true),
        ];

        for (pattern, input, expected) in &cases {
            println!("\nTesting {pattern:?} against {input:?} with expected result = {expected}");
            assert_eq!(
                Regex::from_str(pattern).unwrap().matches(input),
                *expected,
                "Expected {pattern:?} {}to match {input:?}",
                if *expected { "" } else { "not " }
            );
        }
    }

    #[test]
    fn test_start_anchor() {
        let cases = [
            ("^a", "a", true),
            ("^a", "aa", true),
            ("^a", "ba", false),
            ("^a", "", false),
            ("^log", "slog", false),
        ];
        for (pattern, input, expected) in &cases {
            println!("\nTesting {pattern:?} against {input:?} with expected result = {expected}");
            assert_eq!(
                Regex::from_str(pattern).unwrap().matches(input),
                *expected,
                "Expected {pattern:?} {}to match {input:?}",
                if *expected { "" } else { "not " }
            );
        }
    }
}
