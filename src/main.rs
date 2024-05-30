use clap::{Args, Parser};
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

    println!("Compiling! (..not...)");
    if cli.step.lex {
        println!("Wrapping it up after Lexing.");
        fs::remove_file(prep_file).expect("Could not remove preprocessed file.");
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
