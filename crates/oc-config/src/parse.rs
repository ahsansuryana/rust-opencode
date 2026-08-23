//! Ported from: packages/opencode/src/config/parse.ts
//! Parser JSONC adalah port 1:1 algoritma vscode `jsonc-parser@3.3.1`
//! (lib/esm/impl/scanner.js + parser.js `visit`/`parse`), termasuk kode error,
//! offset, perilaku fault-tolerant, dan format pesan `JsonError`.

use serde_json::{Map, Value};

use crate::v1::error::{InvalidError, Issue, JsonError};

// --- ParseErrorCode (jsonc-parser main.js) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorCode {
    InvalidSymbol = 1,
    InvalidNumberFormat = 2,
    PropertyNameExpected = 3,
    ValueExpected = 4,
    ColonExpected = 5,
    CommaExpected = 6,
    CloseBraceExpected = 7,
    CloseBracketExpected = 8,
    EndOfFileExpected = 9,
    InvalidCommentToken = 10,
    UnexpectedEndOfComment = 11,
    UnexpectedEndOfString = 12,
    UnexpectedEndOfNumber = 13,
    InvalidUnicode = 14,
    InvalidEscapeCharacter = 15,
    InvalidCharacter = 16,
}

/// Ported from: jsonc-parser main.js printParseErrorCode
pub fn print_parse_error_code(code: ParseErrorCode) -> &'static str {
    match code {
        ParseErrorCode::InvalidSymbol => "InvalidSymbol",
        ParseErrorCode::InvalidNumberFormat => "InvalidNumberFormat",
        ParseErrorCode::PropertyNameExpected => "PropertyNameExpected",
        ParseErrorCode::ValueExpected => "ValueExpected",
        ParseErrorCode::ColonExpected => "ColonExpected",
        ParseErrorCode::CommaExpected => "CommaExpected",
        ParseErrorCode::CloseBraceExpected => "CloseBraceExpected",
        ParseErrorCode::CloseBracketExpected => "CloseBracketExpected",
        ParseErrorCode::EndOfFileExpected => "EndOfFileExpected",
        ParseErrorCode::InvalidCommentToken => "InvalidCommentToken",
        ParseErrorCode::UnexpectedEndOfComment => "UnexpectedEndOfComment",
        ParseErrorCode::UnexpectedEndOfString => "UnexpectedEndOfString",
        ParseErrorCode::UnexpectedEndOfNumber => "UnexpectedEndOfNumber",
        ParseErrorCode::InvalidUnicode => "InvalidUnicode",
        ParseErrorCode::InvalidEscapeCharacter => "InvalidEscapeCharacter",
        ParseErrorCode::InvalidCharacter => "InvalidCharacter",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanError {
    None,
    UnexpectedEndOfComment,
    UnexpectedEndOfString,
    UnexpectedEndOfNumber,
    InvalidUnicode,
    InvalidEscapeCharacter,
    InvalidCharacter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntaxKind {
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
    Colon,
    NullKeyword,
    TrueKeyword,
    FalseKeyword,
    StringLiteral,
    NumericLiteral,
    LineCommentTrivia,
    BlockCommentTrivia,
    LineBreakTrivia,
    Trivia,
    Unknown,
    Eof,
}

const CH_0: u32 = 48;
const CH_9: u32 = 57;

fn is_digit_code(code: u32) -> bool {
    (CH_0..=CH_9).contains(&code)
}

fn is_line_break(code: u32) -> bool {
    code == 10 || code == 13
}

// --- Scanner (port impl/scanner.js createScanner) ---

struct Scanner<'a> {
    text: &'a [char],
    pos: usize,
    value: String,
    token_offset: usize,
    token: SyntaxKind,
    scan_error: ScanError,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a [char]) -> Self {
        Scanner {
            text,
            pos: 0,
            value: String::new(),
            token_offset: 0,
            token: SyntaxKind::Unknown,
            scan_error: ScanError::None,
        }
    }

    fn code_at(&self, index: usize) -> Option<u32> {
        self.text.get(index).map(|c| *c as u32)
    }

    /// Port scanHexDigits(count, exact)
    fn scan_hex_digits(&mut self, count: usize, exact: bool) -> i64 {
        const A_UP: u32 = 65;
        const F_UP: u32 = 70;
        const A_LO: u32 = 97;
        const F_LO: u32 = 102;
        let mut digits = 0usize;
        let mut value: i64 = 0;
        while digits < count || !exact {
            let Some(code) = self.code_at(self.pos) else {
                break;
            };
            if is_digit_code(code) {
                value = value * 16 + (code - CH_0) as i64;
            } else if (A_UP..=F_UP).contains(&code) {
                value = value * 16 + (code - A_UP) as i64 + 10;
            } else if (A_LO..=F_LO).contains(&code) {
                value = value * 16 + (code - A_LO) as i64 + 10;
            } else {
                break;
            }
            self.pos += 1;
            digits += 1;
        }
        if digits < count {
            value = -1;
        }
        value
    }

    /// Port scanNumber()
    fn scan_number(&mut self) -> String {
        let start = self.pos;
        if self.code_at(self.pos) == Some(CH_0) {
            self.pos += 1;
        } else {
            self.pos += 1;
            while self.code_at(self.pos).is_some_and(is_digit_code) {
                self.pos += 1;
            }
        }
        if self.code_at(self.pos) == Some(46) {
            // dot
            self.pos += 1;
            if self.code_at(self.pos).is_some_and(is_digit_code) {
                self.pos += 1;
                while self.code_at(self.pos).is_some_and(is_digit_code) {
                    self.pos += 1;
                }
            } else {
                self.scan_error = ScanError::UnexpectedEndOfNumber;
                return self.text[start..self.pos].iter().collect();
            }
        }
        let mut end = self.pos;
        if let Some(code) = self.code_at(self.pos) {
            if code == 69 || code == 101 {
                // E / e
                self.pos += 1;
                if matches!(self.code_at(self.pos), Some(43) | Some(45)) {
                    self.pos += 1;
                }
                if self.code_at(self.pos).is_some_and(is_digit_code) {
                    self.pos += 1;
                    while self.code_at(self.pos).is_some_and(is_digit_code) {
                        self.pos += 1;
                    }
                    end = self.pos;
                } else {
                    self.scan_error = ScanError::UnexpectedEndOfNumber;
                }
            }
        }
        self.text[start..end].iter().collect()
    }

    /// Port scanString()
    fn scan_string(&mut self) -> String {
        let mut result = String::new();
        let mut start = self.pos;
        loop {
            if self.pos >= self.text.len() {
                result.extend(self.text[start..self.pos].iter());
                self.scan_error = ScanError::UnexpectedEndOfString;
                break;
            }
            let ch = self.code_at(self.pos).unwrap();
            if ch == 34 {
                // double quote
                result.extend(self.text[start..self.pos].iter());
                self.pos += 1;
                break;
            }
            if ch == 92 {
                // backslash
                result.extend(self.text[start..self.pos].iter());
                self.pos += 1;
                if self.pos >= self.text.len() {
                    self.scan_error = ScanError::UnexpectedEndOfString;
                    break;
                }
                let ch2 = self.code_at(self.pos).unwrap();
                self.pos += 1;
                match ch2 {
                    34 => result.push('"'),
                    92 => result.push('\\'),
                    47 => result.push('/'),
                    98 => result.push('\u{8}'),
                    102 => result.push('\u{c}'),
                    110 => result.push('\n'),
                    114 => result.push('\r'),
                    116 => result.push('\t'),
                    117 => {
                        let ch3 = self.scan_hex_digits(4, true);
                        if ch3 >= 0 {
                            if let Some(c) = char::from_u32(ch3 as u32) {
                                result.push(c);
                            }
                        } else {
                            self.scan_error = ScanError::InvalidUnicode;
                        }
                    }
                    _ => {
                        self.scan_error = ScanError::InvalidEscapeCharacter;
                    }
                }
                start = self.pos;
                continue;
            }
            if ch <= 0x1f {
                if is_line_break(ch) {
                    result.extend(self.text[start..self.pos].iter());
                    self.scan_error = ScanError::UnexpectedEndOfString;
                    break;
                } else {
                    self.scan_error = ScanError::InvalidCharacter;
                }
            }
            self.pos += 1;
        }
        result
    }

    /// Port scanNext()
    fn scan_next(&mut self) -> SyntaxKind {
        self.value.clear();
        self.scan_error = ScanError::None;
        self.token_offset = self.pos;
        if self.pos >= self.text.len() {
            self.token_offset = self.text.len();
            self.token = SyntaxKind::Eof;
            return self.token;
        }
        let code = self.code_at(self.pos).unwrap();
        // trivia: whitespace (space/tab saja, sesuai isWhiteSpace JS)
        if code == 32 || code == 9 {
            loop {
                self.pos += 1;
                match self.code_at(self.pos) {
                    Some(next) if next == 32 || next == 9 => continue,
                    _ => break,
                }
            }
            self.token = SyntaxKind::Trivia;
            return self.token;
        }
        // trivia: line break
        if is_line_break(code) {
            self.pos += 1;
            if code == 13 && self.code_at(self.pos) == Some(10) {
                self.pos += 1;
            }
            self.token = SyntaxKind::LineBreakTrivia;
            return self.token;
        }
        self.token = match code {
            123 => {
                self.pos += 1;
                SyntaxKind::OpenBrace
            }
            125 => {
                self.pos += 1;
                SyntaxKind::CloseBrace
            }
            91 => {
                self.pos += 1;
                SyntaxKind::OpenBracket
            }
            93 => {
                self.pos += 1;
                SyntaxKind::CloseBracket
            }
            58 => {
                self.pos += 1;
                SyntaxKind::Colon
            }
            44 => {
                self.pos += 1;
                SyntaxKind::Comma
            }
            34 => {
                self.pos += 1;
                self.value = self.scan_string();
                SyntaxKind::StringLiteral
            }
            47 => {
                let start = self.pos - 1;
                if self.code_at(self.pos + 1) == Some(47) {
                    self.pos += 2;
                    while self.pos < self.text.len() {
                        if is_line_break(self.code_at(self.pos).unwrap()) {
                            break;
                        }
                        self.pos += 1;
                    }
                    self.value = self.text[start..self.pos].iter().collect();
                    SyntaxKind::LineCommentTrivia
                } else if self.code_at(self.pos + 1) == Some(42) {
                    self.pos += 2;
                    let safe_length = self.text.len().saturating_sub(1);
                    let mut comment_closed = false;
                    while self.pos < safe_length {
                        let ch = self.code_at(self.pos).unwrap();
                        if ch == 42 && self.code_at(self.pos + 1) == Some(47) {
                            self.pos += 2;
                            comment_closed = true;
                            break;
                        }
                        self.pos += 1;
                        if is_line_break(ch) && ch == 13 && self.code_at(self.pos) == Some(10) {
                            self.pos += 1;
                        }
                    }
                    if !comment_closed {
                        self.pos += 1;
                        self.scan_error = ScanError::UnexpectedEndOfComment;
                    }
                    self.value = self.text[start..self.pos.min(self.text.len())]
                        .iter()
                        .collect();
                    SyntaxKind::BlockCommentTrivia
                } else {
                    self.value.push(char::from_u32(code).unwrap());
                    self.pos += 1;
                    SyntaxKind::Unknown
                }
            }
            45 => {
                self.value.push('-');
                self.pos += 1;
                if self.code_at(self.pos).is_some_and(is_digit_code) {
                    let number = self.scan_number();
                    self.value.push_str(&number);
                    SyntaxKind::NumericLiteral
                } else {
                    SyntaxKind::Unknown
                }
            }
            c if is_digit_code(c) => {
                let number = self.scan_number();
                self.value.push_str(&number);
                SyntaxKind::NumericLiteral
            }
            _ => {
                while self
                    .code_at(self.pos)
                    .is_some_and(is_unknown_content_character)
                {
                    self.pos += 1;
                }
                if self.token_offset != self.pos {
                    self.value = self.text[self.token_offset..self.pos].iter().collect();
                    match self.value.as_str() {
                        "true" => SyntaxKind::TrueKeyword,
                        "false" => SyntaxKind::FalseKeyword,
                        "null" => SyntaxKind::NullKeyword,
                        _ => SyntaxKind::Unknown,
                    }
                } else {
                    self.value.push(char::from_u32(code).unwrap());
                    self.pos += 1;
                    SyntaxKind::Unknown
                }
            }
        };
        self.token
    }
}

fn is_unknown_content_character(code: u32) -> bool {
    if code == 32 || code == 9 || is_line_break(code) {
        return false;
    }
    !matches!(code, 125 | 93 | 123 | 91 | 34 | 58 | 44 | 47)
}

// --- Parser (port parser.js visit/parse) ---

enum Frame {
    /// Map + nama properti yang sedang menunggu nilai (per-frame, meniru
    /// `currentProperty` TS yang reset saat masuk kontainer baru).
    Object(Map<String, Value>, Option<String>),
    Array(Vec<Value>),
}

struct ParserState<'a> {
    scanner: Scanner<'a>,
    errors: Vec<(ParseErrorCode, usize)>,
    allow_trailing_comma: bool,
    stack: Vec<Frame>,
    root: Vec<Value>,
}

impl<'a> ParserState<'a> {
    fn scan_next_token(&mut self) -> SyntaxKind {
        loop {
            let token = self.scanner.scan_next();
            let offset = self.scanner.token_offset;
            match self.scanner.scan_error {
                ScanError::None => {}
                ScanError::UnexpectedEndOfComment => {
                    self.push_err(ParseErrorCode::UnexpectedEndOfComment, offset)
                }
                ScanError::UnexpectedEndOfString => {
                    self.push_err(ParseErrorCode::UnexpectedEndOfString, offset)
                }
                ScanError::UnexpectedEndOfNumber => {
                    self.push_err(ParseErrorCode::UnexpectedEndOfNumber, offset)
                }
                ScanError::InvalidUnicode => self.push_err(ParseErrorCode::InvalidUnicode, offset),
                ScanError::InvalidEscapeCharacter => {
                    self.push_err(ParseErrorCode::InvalidEscapeCharacter, offset)
                }
                ScanError::InvalidCharacter => {
                    self.push_err(ParseErrorCode::InvalidCharacter, offset)
                }
            }
            match token {
                SyntaxKind::LineCommentTrivia
                | SyntaxKind::BlockCommentTrivia
                | SyntaxKind::LineBreakTrivia
                | SyntaxKind::Trivia => {}
                SyntaxKind::Unknown => self.push_err(ParseErrorCode::InvalidSymbol, offset),
                other => return other,
            }
        }
    }

    fn push_err(&mut self, code: ParseErrorCode, offset: usize) {
        self.errors.push((code, offset));
    }

    /// Port handleError(error, skipUntilAfter, skipUntil)
    fn handle_error(
        &mut self,
        error: ParseErrorCode,
        skip_until_after: &[SyntaxKind],
        skip_until: &[SyntaxKind],
    ) {
        let offset = self.scanner.token_offset;
        self.errors.push((error, offset));
        if !skip_until_after.is_empty() || !skip_until.is_empty() {
            let mut token = self.scanner.token;
            while token != SyntaxKind::Eof {
                if skip_until_after.contains(&token) {
                    self.scan_next_token();
                    break;
                } else if skip_until.contains(&token) {
                    break;
                }
                token = self.scan_next_token();
            }
        }
    }

    /// Meniru onValue(): nilai selesai dilampirkan ke kontainer induknya.
    fn attach(&mut self, value: Value) {
        match self.stack.last_mut() {
            Some(Frame::Object(map, pending)) => {
                if let Some(name) = pending.take() {
                    map.insert(name, value);
                }
            }
            Some(Frame::Array(items)) => items.push(value),
            None => self.root.push(value),
        }
    }

    fn parse_string(&mut self, is_value: bool) -> bool {
        let value = self.scanner.value.clone();
        if is_value {
            self.attach(Value::String(value));
        } else {
            if let Some(Frame::Object(_, pending)) = self.stack.last_mut() {
                *pending = Some(value);
            }
        }
        self.scan_next_token();
        true
    }

    fn parse_literal(&mut self) -> bool {
        match self.scanner.token {
            SyntaxKind::NumericLiteral => {
                let token_value = self.scanner.value.clone();
                match token_value.parse::<f64>() {
                    Ok(number) => {
                        // JSON.stringify JS mencetak bilangan bulat tanpa ".0";
                        // samakan supaya byte-for-byte setara.
                        let number_value = if number.is_finite()
                            && number.fract() == 0.0
                            && number.abs() <= 9_007_199_254_740_991.0
                        {
                            Value::Number((number as i64).into())
                        } else {
                            serde_json::Number::from_f64(number)
                                .map(Value::Number)
                                .unwrap_or(Value::Null)
                        };
                        self.attach(number_value);
                    }
                    Err(_) => {
                        self.handle_error(ParseErrorCode::InvalidNumberFormat, &[], &[]);
                        self.attach(Value::from(0));
                    }
                }
            }
            SyntaxKind::NullKeyword => self.attach(Value::Null),
            SyntaxKind::TrueKeyword => self.attach(Value::Bool(true)),
            SyntaxKind::FalseKeyword => self.attach(Value::Bool(false)),
            _ => return false,
        }
        self.scan_next_token();
        true
    }

    /// Port parseProperty()
    fn parse_property(&mut self) -> bool {
        if self.scanner.token != SyntaxKind::StringLiteral {
            self.handle_error(
                ParseErrorCode::PropertyNameExpected,
                &[],
                &[SyntaxKind::CloseBrace, SyntaxKind::Comma],
            );
            return false;
        }
        self.parse_string(false);
        if self.scanner.token == SyntaxKind::Colon {
            self.scan_next_token();
            if !self.parse_value() {
                self.handle_error(
                    ParseErrorCode::ValueExpected,
                    &[],
                    &[SyntaxKind::CloseBrace, SyntaxKind::Comma],
                );
            }
        } else {
            self.handle_error(
                ParseErrorCode::ColonExpected,
                &[],
                &[SyntaxKind::CloseBrace, SyntaxKind::Comma],
            );
        }
        true
    }

    /// Port parseObject()
    fn parse_object(&mut self) -> bool {
        self.stack.push(Frame::Object(Map::new(), None));
        self.scan_next_token();
        let mut needs_comma = false;
        while self.scanner.token != SyntaxKind::CloseBrace && self.scanner.token != SyntaxKind::Eof
        {
            if self.scanner.token == SyntaxKind::Comma {
                if !needs_comma {
                    self.handle_error(ParseErrorCode::ValueExpected, &[], &[]);
                }
                self.scan_next_token();
                if self.scanner.token == SyntaxKind::CloseBrace && self.allow_trailing_comma {
                    break;
                }
            } else if needs_comma {
                self.handle_error(ParseErrorCode::CommaExpected, &[], &[]);
            }
            if !self.parse_property() {
                self.handle_error(
                    ParseErrorCode::ValueExpected,
                    &[],
                    &[SyntaxKind::CloseBrace, SyntaxKind::Comma],
                );
            }
            needs_comma = true;
        }
        let frame = self.stack.pop();
        if let Some(Frame::Object(map, _)) = frame {
            self.attach(Value::Object(map));
        }
        if self.scanner.token != SyntaxKind::CloseBrace {
            self.handle_error(
                ParseErrorCode::CloseBraceExpected,
                &[SyntaxKind::CloseBrace],
                &[],
            );
        } else {
            self.scan_next_token();
        }
        true
    }

    /// Port parseArray()
    fn parse_array(&mut self) -> bool {
        self.stack.push(Frame::Array(Vec::new()));
        self.scan_next_token();
        let mut needs_comma = false;
        while self.scanner.token != SyntaxKind::CloseBracket
            && self.scanner.token != SyntaxKind::Eof
        {
            if self.scanner.token == SyntaxKind::Comma {
                if !needs_comma {
                    self.handle_error(ParseErrorCode::ValueExpected, &[], &[]);
                }
                self.scan_next_token();
                if self.scanner.token == SyntaxKind::CloseBracket && self.allow_trailing_comma {
                    break;
                }
            } else if needs_comma {
                self.handle_error(ParseErrorCode::CommaExpected, &[], &[]);
            }
            if !self.parse_value() {
                self.handle_error(
                    ParseErrorCode::ValueExpected,
                    &[],
                    &[SyntaxKind::CloseBracket, SyntaxKind::Comma],
                );
            }
            needs_comma = true;
        }
        let frame = self.stack.pop();
        if let Some(Frame::Array(items)) = frame {
            self.attach(Value::Array(items));
        }
        if self.scanner.token != SyntaxKind::CloseBracket {
            self.handle_error(
                ParseErrorCode::CloseBracketExpected,
                &[SyntaxKind::CloseBracket],
                &[],
            );
        } else {
            self.scan_next_token();
        }
        true
    }

    /// Port parseValue()
    fn parse_value(&mut self) -> bool {
        match self.scanner.token {
            SyntaxKind::OpenBracket => self.parse_array(),
            SyntaxKind::OpenBrace => self.parse_object(),
            SyntaxKind::StringLiteral => self.parse_string(true),
            _ => self.parse_literal(),
        }
    }
}

/// Port jsonc-parser parse(text, errors, {allowTrailingComma:true}) — mengembalikan
/// Value (undefined pada input tanpa nilai direpresentasikan sebagai Null).
fn parse_impl(text: &[char], errors: &mut Vec<(ParseErrorCode, usize)>) -> Value {
    let scanner = Scanner::new(text);
    let mut state = ParserState {
        scanner,
        errors: std::mem::take(errors),
        allow_trailing_comma: true,
        stack: Vec::new(),
        root: Vec::new(),
    };

    state.scan_next_token();
    if state.scanner.token == SyntaxKind::Eof {
        state.handle_error(ParseErrorCode::ValueExpected, &[], &[]);
    } else {
        if !state.parse_value() {
            state.handle_error(ParseErrorCode::ValueExpected, &[], &[]);
        }
        if state.scanner.token != SyntaxKind::Eof {
            state.handle_error(ParseErrorCode::EndOfFileExpected, &[], &[]);
        }
    }
    *errors = state.errors;
    state.root.into_iter().next().unwrap_or(Value::Null)
}

/// Port jsonc-parser `parse(text, errors, {allowTrailingComma: true})` pada
/// level paling rendah — dipakai golden test agar bisa membandingkan kode
/// error + offset persis dengan fixture dari paket TS asli.
pub fn parse_fault_tolerant(text: &str) -> (Value, Vec<(ParseErrorCode, usize)>) {
    let chars: Vec<char> = text.chars().collect();
    let mut errors: Vec<(ParseErrorCode, usize)> = Vec::new();
    let value = parse_impl(&chars, &mut errors);
    (value, errors)
}

/// Ported from: packages/opencode/src/config/parse.ts:8-33 (jsonc)
pub fn jsonc(text: &str, filepath: &str) -> Result<Value, JsonError> {
    let chars: Vec<char> = text.chars().collect();
    let mut errors: Vec<(ParseErrorCode, usize)> = Vec::new();
    let data = parse_impl(&chars, &mut errors);
    if !errors.is_empty() {
        let lines: Vec<&str> = text.split('\n').collect();
        let issues = errors
            .iter()
            .map(|&(code, offset)| {
                let before_len = offset.min(chars.len());
                let before = &chars[..before_len];
                let line = before.iter().filter(|&&c| c == '\n').count() + 1;
                let column = match before.iter().rposition(|&c| c == '\n') {
                    Some(i) => before.len() - i - 1,
                    None => before.len(),
                } + 1;
                let problem_line = lines.get(line.saturating_sub(1)).copied();
                let err_str = format!(
                    "{} at line {}, column {}",
                    print_parse_error_code(code),
                    line,
                    column
                );
                match problem_line {
                    None => err_str,
                    Some(problem) => format!(
                        "{}\n   Line {}: {}\n{}^",
                        err_str,
                        line,
                        problem,
                        " ".repeat(column + 9)
                    ),
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(JsonError {
            path: filepath.to_string(),
            message: Some(format!(
                "\n--- JSONC Input ---\n{text}\n--- Errors ---\n{issues}\n--- End ---"
            )),
        });
    }
    Ok(data)
}

/// Ported from: packages/opencode/src/config/parse.ts:35-61 (schema)
/// Catatan deviasi: serde berhenti di issue pertama (lihat DEVIATIONS.md).
pub fn schema_decode<T: serde::de::DeserializeOwned>(
    data: Value,
    source: &str,
) -> Result<T, InvalidError> {
    serde_json::from_value(data).map_err(|error| InvalidError {
        path: source.to_string(),
        issues: Some(vec![Issue {
            message: error.to_string(),
            path: Vec::new(),
            extra: Map::new(),
        }]),
        message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let out = jsonc(r#"{"a":1,"b":[true,null,"x"]}"#, "t.json").unwrap();
        assert_eq!(out, serde_json::json!({"a":1,"b":[true,null,"x"]}));
    }

    #[test]
    fn parses_comments_and_trailing_comma() {
        let text = "{\n  // komentar\n  /* block */\n  \"a\": 1,\n  \"b\": [1, 2,],\n}";
        let out = jsonc(text, "t.json").unwrap();
        assert_eq!(out, serde_json::json!({"a":1,"b":[1,2]}));
    }

    #[test]
    fn error_message_format_matches_ts_shape() {
        let err = jsonc("{\n  \"a\": ?,\n}", "cfg.json").unwrap_err();
        assert_eq!(err.path, "cfg.json");
        let msg = err.message.unwrap();
        assert!(msg.starts_with("\n--- JSONC Input ---\n{\n  \"a\": ?,\n}\n--- Errors ---\n"));
        assert!(msg.contains("InvalidSymbol at line 2, column 8"));
        assert!(msg.contains("   Line 2:   \"a\": ?,"));
        assert!(msg.ends_with("\n--- End ---"));
        // caret alignment: kolom 8 → 17 spasi sebelum ^
        assert!(msg.contains("\n                 ^"));
    }

    #[test]
    fn trailing_garbage_yields_end_of_file_expected() {
        // Sesuai TS: token Unknown dikonsumsi fault-tolerant sampai EOF,
        // sehingga HANYA InvalidSymbol yang tercatat.
        let err = jsonc("{} x", "t.json").unwrap_err();
        let msg = err.message.unwrap();
        assert!(msg.contains("InvalidSymbol at line 1, column 4"));
        assert!(msg.contains(
            "
             ^"
        ));
    }

    #[test]
    fn unterminated_string_reports_unexpected_end_of_string() {
        let err = jsonc("{\"a\": \"unterminated}", "t.json").unwrap_err();
        let msg = err.message.unwrap();
        assert!(msg.contains("UnexpectedEndOfString"));
    }

    #[test]
    fn empty_input_yields_value_expected_and_null_result() {
        // errors non-empty → tetap Err meski parser fault-tolerant
        let err = jsonc("", "t.json").unwrap_err();
        assert!(err
            .message
            .unwrap()
            .contains("ValueExpected at line 1, column 1"));
    }

    #[test]
    fn number_edge_cases_match_js_semantics() {
        let out = jsonc("{\"c\": 1e3, \"d\": -5, \"e\": 2.5}", "t.json").unwrap();
        assert_eq!(out["c"], serde_json::json!(1000));
        assert_eq!(out["d"], serde_json::json!(-5));
        assert_eq!(out["e"], serde_json::json!(2.5));
    }

    #[test]
    fn fault_tolerant_cases_still_report_errors() {
        // leading zero: parser lanjut fault-tolerant tapi errors tercatat
        let err = jsonc("{\"a\": 01}", "t.json").unwrap_err();
        let msg = err.message.unwrap();
        assert!(msg.contains("CommaExpected"));
        assert!(msg.contains("PropertyNameExpected"));
        // "1." men-trigger UnexpectedEndOfNumber seperti di JS
        let err = jsonc("{\"b\": 1.}", "t.json").unwrap_err();
        assert!(err.message.unwrap().contains("UnexpectedEndOfNumber"));
    }
}
