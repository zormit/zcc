use clap::{Args, Parser};
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Driver {
    /// Path to the file to compile
    path: PathBuf,
    #[command(flatten)]
    step: Step,
}

#[derive(Args)]
#[group(multiple = false)]
struct Step {
    /// Run the lexer, but stop before parsing
    #[arg(long, action)]
    lex: bool,
    /// Run the lexer and parser, but stop before assembly generation
    #[arg(long, action)]
    parse: bool,
    /// Perform lexing, parsing, and assembly generation, but stop before code emission
    #[arg(long, action)]
    codegen: bool,
}

#[derive(Debug, PartialEq, Clone)]
enum TokenKind {
    Identifier,
    Constant,
    Keyword,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Eof,
    ErrorToken,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(dead_code)]
struct Token {
    kind: TokenKind,
    text: String,
}

impl Token {
    fn new(kind: TokenKind, text: &str) -> Self {
        Token {
            kind,
            text: text.into(),
        }
    }
    fn open_paren() -> Self {
        Self::new(TokenKind::OpenParen, "(")
    }
    fn close_paren() -> Self {
        Self::new(TokenKind::CloseParen, ")")
    }
    fn open_brace() -> Self {
        Self::new(TokenKind::OpenBrace, "{")
    }
    fn close_brace() -> Self {
        Self::new(TokenKind::CloseBrace, "}")
    }
    fn semicolon() -> Self {
        Self::new(TokenKind::Semicolon, ";")
    }
    fn constant(text: &str) -> Self {
        Self::new(TokenKind::Constant, text)
    }
    fn keyword(text: &str) -> Self {
        Self::new(TokenKind::Keyword, text)
    }
    fn identifier(text: &str) -> Self {
        Self::new(TokenKind::Identifier, text)
    }
    fn error() -> Self {
        Self::new(TokenKind::ErrorToken, "")
    }
    fn eof() -> Self {
        Self::new(TokenKind::Eof, "")
    }
}

fn lexer(text: String) -> Vec<Token> {
    // while input isn't empty:
    //   if input starts with whitespace:
    //     trim whitespace from start of input
    //   else:
    //     find longest match at start of input for any regex in Table 1-1
    //     if no match is found, raise an error
    //     convert matching substring into a token
    //     remove matching substring from start of input
    let mut token = vec![];
    let mut input = text.as_str();
    while !input.is_empty() {
        let char = input.chars().next().expect("Should have had a character");

        if char.is_whitespace() {
        } else if char == '(' {
            token.push(Token::open_paren());
        } else if char == ')' {
            token.push(Token::close_paren());
        } else if char == '{' {
            token.push(Token::open_brace());
        } else if char == '}' {
            token.push(Token::close_brace());
        } else if char == ';' {
            token.push(Token::semicolon());
        } else {
            let keyword = Regex::new(r"^(void|int|return)\b").unwrap();
            let constant = Regex::new(r"^([0-9]+)\b").unwrap();
            let identifier = Regex::new(r"^([a-zA-Z_]\w*)\b").unwrap();
            if constant.is_match(input) {
                let caps = constant.captures(input).unwrap();
                let matched_const = caps.get(0).unwrap().as_str();
                input = &input[matched_const.len()..];
                token.push(Token::constant(matched_const));
                continue;
            } else if identifier.is_match(input) {
                if keyword.is_match(input) {
                    let caps = keyword.captures(input).unwrap();
                    let matched_keyword = caps.get(0).unwrap().as_str();
                    input = &input[matched_keyword.len()..];
                    token.push(Token::keyword(matched_keyword));
                    continue;
                }
                let caps = identifier.captures(input).unwrap();
                let matched_identifier = caps.get(0).unwrap().as_str();
                input = &input[matched_identifier.len()..];
                token.push(Token::identifier(matched_identifier));
                continue;
            } else {
                token.push(Token::error())
            }
        }

        input = &input[1..];
    }
    token.push(Token::eof());
    token
}

fn main() {
    let cli = Driver::parse();
    println!("Starting to compile {}", cli.path.display());

    println!("Preprocessing");
    let input_file = &cli.path;
    let prep_file = &cli.path.with_extension("i");
    println!(
        "gcc -E -P {} -o {}",
        input_file.display(),
        prep_file.display()
    );
    let prep = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(input_file)
        .arg("-o")
        .arg(prep_file)
        .status()
        .unwrap();
    println!("Preprocess finished with: {prep}");

    println!("Lexing! (..not...)");
    let text = fs::read_to_string(prep_file).expect("Failed to read input file.");
    let tokens = lexer(text);

    if cli.step.lex {
        dbg!(&tokens);
        println!("Wrapping it up after Lexing.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        if tokens.iter().any(|t| t.kind == TokenKind::ErrorToken) {
            process::exit(1);
        }
        process::exit(0);
    }
    if cli.step.parse {
        println!("Wrapping it up after Parsing.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        process::exit(0);
    }
    if cli.step.codegen {
        println!("Wrapping it up after Code generation.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        process::exit(0);
    }

    fs::remove_file(prep_file).expect("Could not remove preprocessed file.");

    let ass_file = &cli.path.with_extension("s");
    let out_file = &cli.path.with_extension("");
    println!("gcc {} -o {}", ass_file.display(), out_file.display());
    let assemble = Command::new("gcc")
        .arg(ass_file)
        .arg("-o")
        .arg(out_file)
        .status()
        .unwrap();
    println!("Preprocess finished with: {assemble}");
}
