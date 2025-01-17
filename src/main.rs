use clap::{Args, Parser as ClapParser};
use regex::Regex;
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

#[derive(ClapParser)]
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

#[derive(Debug, PartialEq, Copy, Clone)]
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
    // fn eof() -> Self {
    //     Self::new(TokenKind::Eof, "")
    // }
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
    // token.push(Token::eof());
    token
}

#[derive(Debug, PartialEq, Clone)]
enum TreeKind {
    Program,
    Function,
    Return,
    ErrorTree,
}
#[derive(Debug, PartialEq, Clone)]
struct Tree {
    kind: TreeKind,
    children: Vec<Child>,
}
#[derive(Debug, PartialEq, Clone)]
enum Child {
    Token(Token),
    Tree(Tree),
}

#[derive(Debug, PartialEq)]
enum Event {
    Open { kind: TreeKind },
    Close,
    Advance,
}
struct MarkOpened {
    index: usize,
}
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    fuel: Cell<u32>,
    events: Vec<Event>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            fuel: Cell::new(256),
            events: Vec::default(),
        }
    }

    fn open(&mut self) -> MarkOpened {
        let mark = MarkOpened {
            index: self.events.len(),
        };
        self.events.push(Event::Open {
            kind: TreeKind::ErrorTree,
        });
        mark
    }
    fn advance(&mut self) {
        assert!(!self.eof());
        self.fuel.set(256);
        self.events.push(Event::Advance);
        self.pos += 1;
    }

    fn eof(&self) -> bool {
        self.pos == self.tokens.len()
    }
    fn close(&mut self, m: MarkOpened, kind: TreeKind) {
        self.events[m.index] = Event::Open { kind };
        self.events.push(Event::Close);
    }

    fn nth(&self, lookahead: usize) -> TokenKind {
        if self.fuel.get() == 0 {
            panic!("parser is stuck")
        }
        self.fuel.set(self.fuel.get() - 1);
        self.tokens
            .get(self.pos + lookahead)
            .map_or(TokenKind::Eof, |t| t.kind)
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.nth(0) == kind
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.at(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) {
        if self.eat(kind) {
            return;
        }
        eprintln!("expected {kind:?}");
        process::exit(1);
    }

    fn build_tree(self) -> Tree {
        let mut tokens = self.tokens.into_iter();
        let mut events = self.events;
        let mut stack = Vec::new();

        assert!(matches!(events.pop(), Some(Event::Close)));

        for event in events {
            match event {
                Event::Open { kind } => stack.push(Tree {
                    kind,
                    children: Vec::new(),
                }),
                Event::Close => {
                    let tree = stack.pop().unwrap();
                    stack.last_mut().unwrap().children.push(Child::Tree(tree));
                }
                Event::Advance => {
                    let token = tokens.next().unwrap();
                    stack.last_mut().unwrap().children.push(Child::Token(token))
                }
            }
        }

        assert!(stack.len() == 1);
        assert!(tokens.next().is_none());

        stack.pop().unwrap()
    }

    fn pretty_print(tree: &Tree, depth: usize, show_kind: bool) {
        //  Program(
        //      Function(
        //          name="main",
        //          body=Return(
        //              Constant(2)
        //          )
        //      )
        //  )
        if show_kind {
            println!("{:depth$}{:?}(", "", tree.kind);
        }
        match tree.kind {
            TreeKind::Program => {
                for child in &tree.children {
                    if let Child::Tree(t) = child {
                        Parser::pretty_print(t, depth + 4, true);
                    }
                }
            }
            TreeKind::Function => {
                if let Some(Child::Token(Token {
                    text,
                    kind: TokenKind::Identifier,
                })) = tree.children.get(1)
                {
                    println!("{:depth$}name = \"{text}\"", "", depth = depth + 4);
                }
                if let Some(Child::Tree(Tree { kind, children })) = tree.children.get(6) {
                    println!("{:depth$}body = {kind:?}(", "", depth = depth + 4);
                    Parser::pretty_print(
                        &Tree {
                            kind: kind.clone(),
                            children: children.clone(),
                        },
                        depth + 4,
                        false,
                    );
                    println!("{:depth$})", "", depth = depth + 4);
                }
            }
            TreeKind::Return => {
                if let Some(Child::Token(Token { text, kind })) = tree.children.get(1) {
                    println!("{:depth$}{kind:?}({text})", "", depth = depth + 4);
                }
            }
            TreeKind::ErrorTree => {}
        }
        if show_kind {
            println!("{:depth$})", "");
        }
    }
}

fn parse_program(p: &mut Parser) {
    let m = p.open();

    dbg!(p.tokens.len());

    while !p.eof() {
        dbg!(p.nth(0));
        dbg!(p.pos);
        if p.at(TokenKind::Keyword) {
            parse_function(p)
        } else {
            panic!("expected a keyword");
        }
    }
    p.close(m, TreeKind::Program);
    //    Tree {
    //        kind: TreeKind::Program,
    //        children: vec![Child::Tree(Tree {
    //            kind: TreeKind::Function,
    //            children: vec![Child::Tree(Tree {
    //                kind: TreeKind::Return,
    //                children: vec![Child::Token(Token::Constant(2))],
    //            })],
    //        })],
    //    }
}

// function = "int" <identifier> "(" "void" ")" "{" <statement> "}"
fn parse_function(p: &mut Parser) {
    // TODO: is this enough as a guard? or do we need lookahead later?
    assert!(p.at(TokenKind::Keyword));
    let m = p.open();

    p.expect(TokenKind::Keyword);
    p.expect(TokenKind::Identifier);
    p.expect(TokenKind::OpenParen);
    p.expect(TokenKind::Keyword);
    p.expect(TokenKind::CloseParen);
    p.expect(TokenKind::OpenBrace);
    parse_statement(p);
    p.expect(TokenKind::CloseBrace);

    p.close(m, TreeKind::Function);
}

// "return" <exp> ";"
fn parse_statement(p: &mut Parser) {
    let m = p.open();
    p.expect(TokenKind::Keyword);
    p.expect(TokenKind::Constant);
    p.expect(TokenKind::Semicolon);

    p.close(m, TreeKind::Return);
}

// program = Program(function_definition)
// function_definition = Function(identifier name, instruction* instructions)
// instruction = Mov(operand src, operand dst) | Ret
// operand = Imm(int) | Register

#[derive(Debug, PartialEq, Clone)]
struct ASMProgram(ASMFunction);
#[derive(Debug, PartialEq, Clone)]
struct ASMFunction {
    identifier: String,
    instructions: Vec<ASMInstruction>,
}
#[derive(Debug, PartialEq, Clone)]
enum ASMInstruction {
    Mov { src: ASMOperand, dst: ASMOperand },
    Ret,
}
#[derive(Debug, PartialEq, Clone)]
enum ASMOperand {
    Imm(u32),
    Register,
}

fn generate_assembly(tree: &Tree) -> ASMProgram {
    match tree.kind {
        TreeKind::Program => {
            if let Some(Child::Tree(tree)) = tree.children.first() {
                ASMProgram(generate_function(tree))
            } else {
                panic!("Should have had a Tree Child");
            }
        }
        _ => panic!("should have been a program here."),
    }
}

fn generate_function(tree: &Tree) -> ASMFunction {
    match tree.kind {
        TreeKind::Function => {
            if let Some(Child::Token(Token {
                text,
                kind: TokenKind::Identifier,
            })) = tree.children.get(1)
            {
                if let Some(Child::Tree(tree)) = tree.children.get(6) {
                    ASMFunction {
                        identifier: text.to_owned(),
                        instructions: generate_return(tree),
                    }
                } else {
                    panic!("could not find body");
                }
            } else {
                panic!("could not find identifier");
            }
        }
        _ => panic!("should have been a function."),
    }
}

fn generate_return(tree: &Tree) -> Vec<ASMInstruction> {
    match tree.kind {
        TreeKind::Return => {
            if let Some(Child::Token(Token {
                text,
                kind: TokenKind::Constant,
            })) = tree.children.get(1)
            {
                vec![
                    ASMInstruction::Mov {
                        src: ASMOperand::Imm(text.parse().unwrap()),
                        dst: ASMOperand::Register,
                    },
                    ASMInstruction::Ret,
                ]
            } else {
                panic!("No constant found where one was expected");
            }
        }
        _ => panic!("should have been a function."),
    }
}

fn emit_program(asm: &ASMProgram) -> Vec<u8> {
    let mut output = vec![];

    let ASMProgram(f) = asm;
    let ASMFunction {
        identifier,
        instructions,
    } = f;
    output.extend_from_slice(b"\t.globl\t_");
    output.extend_from_slice(identifier.as_bytes());
    output.extend_from_slice(b"\n");
    output.extend_from_slice(b"_");
    output.extend_from_slice(identifier.as_bytes());
    output.extend_from_slice(b":\n");
    for instruction in instructions {
        match instruction {
            ASMInstruction::Mov { src, dst } => {
                output.extend_from_slice(b"\tmovl\t");
                output.extend(emit_op(src));
                output.extend_from_slice(b", ");
                output.extend(emit_op(dst));
                output.extend_from_slice(b"\n");
            }
            ASMInstruction::Ret => {
                output.extend_from_slice(b"\tret\n");
            }
        }
    }
    output
}

fn emit_op(op: &ASMOperand) -> Vec<u8> {
    let mut output = vec![];
    match op {
        ASMOperand::Imm(i) => {
            output.extend_from_slice(b"$");
            output.extend_from_slice(i.to_string().as_bytes());
        }
        ASMOperand::Register => {
            output.extend_from_slice(b"%eax");
        }
    }
    output
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

    println!("Lexing!");
    let text = fs::read_to_string(prep_file).expect("Failed to read input file.");
    let tokens = lexer(text);

    dbg!(&tokens);
    if cli.step.lex {
        println!("Wrapping it up after Lexing.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        if tokens.iter().any(|t| t.kind == TokenKind::ErrorToken) {
            process::exit(1);
        }
        process::exit(0);
    }

    let mut parser = Parser::new(tokens);
    parse_program(&mut parser);
    dbg!(&parser.events);
    let tree = parser.build_tree();
    //dbg!(&tree);
    dbg!(Parser::pretty_print(&tree, 0, true));

    if cli.step.parse {
        println!("Wrapping it up after Parsing.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        process::exit(0);
    }

    let asm_tree = generate_assembly(&tree);
    dbg!(&asm_tree);

    if cli.step.codegen {
        println!("Wrapping it up after Code generation.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
        process::exit(0);
    }

    let ass_file = &cli.path.with_extension("s");
    fs::write(ass_file, emit_program(&asm_tree)).unwrap();

    fs::remove_file(prep_file).expect("Could not remove preprocessed file.");

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
