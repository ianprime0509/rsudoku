extern crate clap;
#[macro_use]
extern crate error_chain;

extern crate rsudoku;

use std::fs::File;
use std::io::{self, BufReader, Read};

use clap::{Arg, ArgMatches, App, SubCommand};

use rsudoku::Sudoku;
use rsudoku::errors::*;
use rsudoku::tui;

quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("RSudoku")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Ian Johnson <ianprime0509@gmail.com>")
        .subcommand(
            SubCommand::with_name("generate")
                .about("generates a random Sudoku grid")
                .arg(Arg::with_name("pretty").short("p").long("pretty").help(
                    "Pretty prints the output",
                )),
        )
        .subcommand(
            SubCommand::with_name("print")
                .about("prints a Sudoku grid")
                .arg(Arg::with_name("pretty").short("p").long("pretty").help(
                    "Pretty prints the output",
                ))
                .arg(
                    Arg::with_name("INPUT")
                        .help("Sets the input file to use")
                        .default_value("-"),
                ),
        )
        .subcommand(
            SubCommand::with_name("solve")
                .about("solves a Sudoku puzzle")
                .arg(Arg::with_name("all").short("a").long("all").help(
                    "Prints all solutions",
                ))
                .arg(Arg::with_name("pretty").short("p").long("pretty").help(
                    "Pretty prints the output",
                ))
                .arg(
                    Arg::with_name("INPUT")
                        .help("Sets the input file to use")
                        .default_value("-"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("generate", Some(m)) => generate(m),
        ("print", Some(m)) => print(m),
        ("solve", Some(m)) => solve(m),
        _ => play(&ArgMatches::new()),
    }
}

fn generate(m: &ArgMatches) -> Result<()> {
    let s = Sudoku::generate();
    if m.is_present("pretty") {
        println!("{:#}", s);
    } else {
        println!("{}", s);
    }

    Ok(())
}

fn play(m: &ArgMatches) -> Result<()> {
    tui::Game::run()
}

fn print(m: &ArgMatches) -> Result<()> {
    // We can safely unwrap here since we set a default value
    let input = m.value_of("INPUT").unwrap();
    let mut br = if input == "-" {
        BufReader::new(Box::new(io::stdin()) as Box<Read>)
    } else {
        BufReader::new(Box::new(File::open(input).chain_err(|| {
            format!("could not open file `{}`", input)
        })?) as Box<Read>)
    };
    let mut contents = String::new();
    br.read_to_string(&mut contents).chain_err(|| {
        format!("could not read contents of file `{}`", input)
    })?;

    let s = contents.parse::<Sudoku>()?;
    if m.is_present("pretty") {
        println!("{:#}", s);
    } else {
        println!("{}", s);
    }

    Ok(())
}

fn solve(m: &ArgMatches) -> Result<()> {
    // We can safely unwrap here since we set a default value
    let input = m.value_of("INPUT").unwrap();
    let mut br = if input == "-" {
        BufReader::new(Box::new(io::stdin()) as Box<Read>)
    } else {
        BufReader::new(Box::new(File::open(input).chain_err(|| {
            format!("could not open file `{}`", input)
        })?) as Box<Read>)
    };
    let mut contents = String::new();
    br.read_to_string(&mut contents).chain_err(|| {
        format!("could not read contents of file `{}`", input)
    })?;

    let s = contents.parse::<Sudoku>()?;
    if m.is_present("all") {
        let sols = s.solutions().collect::<Vec<_>>();
        let nsols = sols.len();
        for sol in sols {
            if m.is_present("pretty") {
                println!("{:#}", sol);
                println!("============");
            } else {
                println!("{}", sol);
                println!("=========");
            }
        }
        println!(
            "Found {} solution{}",
            nsols,
            if nsols == 1 { "" } else { "s" }
        );
    } else {
        match s.solutions().next() {
            None => println!("No solution found"),
            Some(sol) => {
                if m.is_present("pretty") {
                    println!("{:#}", sol);
                } else {
                    println!("{}", sol);
                }
            }
        }
    }

    Ok(())
}
