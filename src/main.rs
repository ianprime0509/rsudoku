// Copyright (C) 2017 Ian Johnson
//
// This file is part of RSudoku.
//
// RSudoku is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// RSudoku is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with RSudoku.  If not, see <http://www.gnu.org/licenses/>.
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

fn play(_m: &ArgMatches) -> Result<()> {
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
