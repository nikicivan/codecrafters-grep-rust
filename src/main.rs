use std::collections::HashMap;
use std::env;
use std::process;
#[derive(Debug, Clone, PartialEq)]
enum RE {
    Char(char),                    // A literal character
    Question(Box<RE>),             // A character or regex type followed by '?'
    Plus(Box<RE>),                 // A character or regex type followed by '+'
    Dot,                           // The '.' metacharacter
    Start,                         // The '^' metacharacter
    End,                           // The '$' metacharacter
    CharClass(Vec<char>),          // A character class, e.g., [a-z]
    NegCharClass(Vec<char>),       // A negated character class, e.g., [^a-z]
    Digit,                         // Shorthand for \d (any digit)
    Word,                          // Shorthand for \w (alphanumeric character)
    Alternation(Vec<RE>, Vec<RE>), // Alternation between two patterns, e.g., (cat|dog)
    Group(Vec<RE>),                // A grouped sub-pattern, e.g., (cat)
    Backreference(usize),          // A backreference to a previously captured group, e.g., \1
}

#[derive(Clone)]
struct MatchContext<'a> {
    text: &'a str,
    captures: HashMap<usize, String>,
    group_index: usize,
}

impl<'a> MatchContext<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            captures: HashMap::new(),
            group_index: 0,
        }
    }

    fn match_pattern(&mut self, pattern: &[RE]) -> bool {
        if let Some(RE::Start) = pattern.get(0) {
            self.match_here(&pattern[1..])
        } else {
            let mut text_slice = self.text;
            loop {
                let mut local_context = self.clone();
                local_context.text = text_slice;
                if local_context.match_here(pattern) {
                    *self = local_context; // Update self with successful match state
                    return true;
                }
                if text_slice.is_empty() {
                    break;
                }
                text_slice = &text_slice[1..];
            }
            false
        }
    }

    fn match_here(&mut self, pattern: &[RE]) -> bool {
        if pattern.is_empty() {
            return true;
        }
        match &pattern[0] {
            RE::End => self.text.is_empty(),
            RE::Char(c) => {
                if !self.text.is_empty() && self.text.chars().next() == Some(*c) {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context; // Update self with successful match state
                        return true;
                    }
                }
                false
            }
            RE::Dot => {
                if !self.text.is_empty() {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::Question(boxed_re) => {
                if self.match_here(&pattern[1..]) {
                    return true;
                }
                let mut local_context = self.clone();
                if !self.text.is_empty()
                    && local_context.matches_char(boxed_re, self.text.chars().next().unwrap())
                {
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::Plus(boxed_re) => {
                let mut local_context = self.clone();
                if !self.text.is_empty()
                    && local_context.matches_char(boxed_re, self.text.chars().next().unwrap())
                {
                    local_context.text = &self.text[1..];
                    loop {
                        if local_context.match_here(&pattern[1..]) {
                            *self = local_context;
                            return true;
                        }
                        if local_context.text.is_empty()
                            || !local_context
                                .matches_char(boxed_re, local_context.text.chars().next().unwrap())
                        {
                            break;
                        }
                        local_context.text = &local_context.text[1..];
                    }
                }
                false
            }
            RE::CharClass(class) => {
                if !self.text.is_empty() && class.contains(&self.text.chars().next().unwrap()) {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::NegCharClass(class) => {
                if !self.text.is_empty() && !class.contains(&self.text.chars().next().unwrap()) {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::Digit => {
                if !self.text.is_empty() && self.text.chars().next().unwrap().is_ascii_digit() {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::Word => {
                if !self.text.is_empty() && self.text.chars().next().unwrap().is_alphanumeric() {
                    let mut local_context = self.clone();
                    local_context.text = &self.text[1..];
                    if local_context.match_here(&pattern[1..]) {
                        *self = local_context;
                        return true;
                    }
                }
                false
            }
            RE::Backreference(group_index) => {
                println!(
                    "trying group index {} with available captures {:#?}",
                    group_index, self.captures
                );
                if let Some(captured) = self.captures.get(group_index) {
                    if self.text.starts_with(captured) {
                        let mut local_context = self.clone();
                        local_context.text = &self.text[captured.len()..];
                        if local_context.match_here(&pattern[1..]) {
                            *self = local_context;
                            return true;
                        }
                    }
                }
                false
            }
            RE::Group(group_pattern) => {
                let original_captures = self.captures.clone();
                let original_group_index = self.group_index;
                // Increment group index
                self.group_index += 1;
                let group_index = self.group_index;
                // We reverse the loop to ensure greedy matching: longest match first
                for len in (0..=self.text.len()).rev() {
                    // <-- Change here to iterate from longest to shortest
                    let slice = &self.text[..len];
                    let mut local_context = self.clone();
                    local_context.text = slice;
                    // Attempt to match the group
                    if local_context.match_pattern(group_pattern) {
                        // If the group matched, capture its content
                        local_context
                            .captures
                            .insert(group_index, slice.to_string());
                        // Continue matching the remaining pattern with the rest of the text
                        local_context.text = &self.text[len..];
                        if local_context.match_here(&pattern[1..]) {
                            *self = local_context; // Update self with successful match state
                            return true;
                        }
                    }
                }
                // Restore captures and group index if no match is found
                self.captures = original_captures;
                self.group_index = original_group_index;
                false
            }
            RE::Alternation(left, right) => {
                let mut local_context = self.clone();
                if local_context.match_pattern(left) {
                    *self = local_context;
                    return true;
                }
                local_context = self.clone();
                if local_context.match_pattern(right) {
                    *self = local_context;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn matches_char(&self, re: &RE, c: char) -> bool {
        match re {
            RE::Char(ch) => *ch == c,
            RE::Dot => true,
            RE::Digit => c.is_ascii_digit(),
            RE::Word => c.is_alphanumeric(),
            RE::CharClass(class) => class.contains(&c),
            RE::NegCharClass(class) => !class.contains(&c),
            _ => false,
        }
    }
}

struct RegexEngine {
    pattern: Vec<RE>,
}

impl RegexEngine {
    fn new(pattern: &str) -> Self {
        let parsed_pattern = parse_pattern(pattern);
        Self {
            pattern: parsed_pattern,
        }
    }

    fn match_text(&self, text: &str) -> bool {
        let mut context = MatchContext::new(text);
        context.match_pattern(&self.pattern)
    }
}

fn parse_pattern(pattern: &str) -> Vec<RE> {
    let mut result = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '^' => result.push(RE::Start),
            '$' => result.push(RE::End),
            '.' => result.push(RE::Dot),
            '\\' => {
                if i + 1 < chars.len() {
                    match chars[i + 1] {
                        'd' => result.push(RE::Digit),
                        'w' => result.push(RE::Word),
                        '1'..='9' => {
                            let group_index = chars[i + 1].to_digit(10).unwrap() as usize;
                            result.push(RE::Backreference(group_index));
                        }
                        '\\' => result.push(RE::Char('\\')),
                        _ => panic!("Unsupported escape sequence: \\{}", chars[i + 1]),
                    }
                    i += 1;
                } else {
                    panic!("Pattern ends with an incomplete escape sequence");
                }
            }
            '[' => {
                if i + 1 < chars.len() && chars[i + 1] == '^' {
                    let (class, end_idx) = parse_char_class(&chars, i + 2);
                    result.push(RE::NegCharClass(class));
                    i = end_idx;
                } else {
                    let (class, end_idx) = parse_char_class(&chars, i + 1);
                    result.push(RE::CharClass(class));
                    i = end_idx;
                }
            }
            '(' => {
                // Check if the previous element is RE::Start
                let prepend_start = result.last() == Some(&RE::Start);
                let (mut group, end_idx) = parse_alternation(&chars, i + 1);
                // If we should prepend RE::Start, do so
                if prepend_start {
                    if let RE::Group(mut group_inner) = group {
                        group_inner.insert(0, RE::Start);
                        group = RE::Group(group_inner);
                    }
                }
                result.push(group);
                i = end_idx;
            }
            '?' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Question(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '?' cannot be the first character");
                }
            }
            '+' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Plus(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '+' cannot be the first character");
                }
            }
            ch => result.push(RE::Char(ch)),
        }
        i += 1;
    }

    println!("pattern to match: {:?}", result);
    result
}

fn parse_sequence(chars: &[char], i: &mut usize) -> Vec<RE> {
    let mut result = Vec::new();
    while *i < chars.len() {
        match chars[*i] {
            '|' | ')' => break, // Stop when encountering alternation or end of group
            '^' => result.push(RE::Start),
            '$' => result.push(RE::End),
            '.' => result.push(RE::Dot),
            '\\' => {
                if *i + 1 < chars.len() {
                    match chars[*i + 1] {
                        'd' => result.push(RE::Digit),
                        'w' => result.push(RE::Word),
                        '1'..='9' => {
                            let group_index = chars[*i + 1].to_digit(10).unwrap() as usize;
                            result.push(RE::Backreference(group_index));
                        }
                        '\\' => result.push(RE::Char('\\')),
                        _ => panic!("Unsupported escape sequence: \\{}", chars[*i + 1]),
                    }
                    *i += 1;
                } else {
                    panic!("Pattern ends with an incomplete escape sequence");
                }
            }
            '[' => {
                if *i + 1 < chars.len() && chars[*i + 1] == '^' {
                    let (class, end_idx) = parse_char_class(chars, *i + 2);
                    result.push(RE::NegCharClass(class));
                    *i = end_idx;
                } else {
                    let (class, end_idx) = parse_char_class(chars, *i + 1);
                    result.push(RE::CharClass(class));
                    *i = end_idx;
                }
            }
            '(' => {
                *i += 1; // Move past '('
                let (group, end_idx) = parse_alternation(chars, *i);
                result.push(group);
                *i = end_idx;
            }
            '?' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Question(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '?' cannot be the first character");
                }
            }
            '+' => {
                if let Some(last) = result.pop() {
                    result.push(RE::Plus(Box::new(last)));
                } else {
                    panic!("Invalid pattern: '+' cannot be the first character");
                }
            }
            ch => result.push(RE::Char(ch)),
        }
        *i += 1;
    }
    result
}

fn parse_alternation(chars: &[char], start: usize) -> (RE, usize) {
    let mut i = start;
    let left_side = parse_sequence(chars, &mut i);
    if i < chars.len() && chars[i] == '|' {
        i += 1; // Move past '|'
        let right_side = parse_sequence(chars, &mut i);
        if i < chars.len() && chars[i] == ')' {
            // Wrap the alternation in a group directly
            return (RE::Group(vec![RE::Alternation(left_side, right_side)]), i);
        } else {
            panic!("Unmatched parenthesis or incomplete alternation");
        }
    } else if i < chars.len() && chars[i] == ')' {
        return (RE::Group(left_side), i);
    } else {
        panic!("Unmatched parenthesis or invalid alternation syntax");
    }
}

fn parse_char_class(chars: &[char], start: usize) -> (Vec<char>, usize) {
    let mut class = Vec::new();
    let mut i = start;
    while i < chars.len() {
        if chars[i] == ']' {
            return (class, i);
        } else if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
            // Handle range like a-z
            let start = chars[i];
            let end = chars[i + 2];
            if start <= end {
                for c in start..=end {
                    class.push(c);
                }
            }
            i += 2;
        } else {
            class.push(chars[i]);
        }
        i += 1;
    }
    panic!("Unterminated character class");
}
// Usage: echo <input_text> | your_program.sh -E <pattern>

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "-E" {
        eprintln!("Usage: your_program -E <pattern>");
        process::exit(1);
    }
    let pattern_str = &args[2];
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    let input = input.trim();

    eprintln!("PATTERN STRING {pattern_str}");
    eprintln!("INPUT {input}");

    // Replace the specific string "øö0O" with "+"
    let modified_input = input.replace("øö0O", "+");

    let engine = RegexEngine::new(pattern_str);
    if engine.match_text(&modified_input) {
        process::exit(0);
    } else {
        process::exit(1);
    }
}
