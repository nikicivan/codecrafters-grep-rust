use anyhow::{anyhow, bail, Context, Result};
use std::{env, io, iter, process, str::FromStr};

#[derive(Debug)]
enum RegexClass {
    Digit,
    Alphanumeric,
}

#[derive(Debug)]
enum RegexElement {
    Literal(char),
    Class(RegexClass),
    CharGroup {
        is_positive: bool,
        options: Vec<char>,
    },
}

impl RegexElement {
    fn read<T: Iterator<Item = char>>(mut chars: T) -> Result<Option<Self>> {
        let result = match chars.next() {
            Some('\\') => match chars.next() {
                Some('d') => RegexElement::Class(RegexClass::Digit),
                Some('w') => RegexElement::Class(RegexClass::Alphanumeric),
                Some(c) => bail!("Unknown escape sequence: \\{c}"),
                None => bail!("Expected character after '\\'"),
            },
            // FIXME: should fail if we reach the end of the string without closing ']'
            // FIXME: handle escape sequences inside char groups
            Some('[') => {
                let mut chars = chars.peekable();

                let is_positive = chars.next_if_eq(&'^').is_none();

                RegexElement::CharGroup {
                    is_positive,
                    options: chars.take_while(|c| c != &']').collect(),
                }
            }
            Some(c) => RegexElement::Literal(c),
            None => return Ok(None),
        };

        Ok(Some(result))
    }

    fn matches<T: Iterator<Item = char>>(&self, mut iter: T) -> bool {
        match self {
            RegexElement::Literal(c) => iter.next() == Some(*c),
            RegexElement::Class(RegexClass::Digit) => iter.next().map_or(false, |c| c.is_digit(10)),
            RegexElement::Class(RegexClass::Alphanumeric) => iter
                .next()
                .map_or(false, |c| c.is_ascii_alphanumeric() || c == '_'),
            RegexElement::CharGroup {
                is_positive,
                options,
            } => iter
                .next()
                .map_or(false, |c| options.contains(&c) == *is_positive),
        }
    }
}

#[derive(Debug)]
struct Regex(Vec<RegexElement>);

impl Regex {
    fn matches(&self, s: &str) -> bool {
        let mut chars = s.chars();

        let first_element = self.0.first().expect("Empty regex");

        while !first_element.matches(&mut chars) {
            if chars.next().is_none() {
                return false;
            }
        }

        for element in &self.0[1..] {
            if !element.matches(&mut chars) {
                return false;
            }
        }

        true
    }
}

impl FromStr for Regex {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut chars = s.chars();

        let elements = iter::from_fn(move || RegexElement::read(&mut chars).transpose())
            .collect::<Result<Vec<_>>>()
            .with_context(|| anyhow!("Failed to parse regex"))?;

        if elements.is_empty() {
            bail!("Empty regex");
        }

        eprintln!("Parsed regex elements: {elements:?}");

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
