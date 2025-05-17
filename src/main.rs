use crate::parser::cpp::header::parse_cpp_header;
use std::fs::DirEntry;
use std::path::Path;
use std::{fs, io};

mod generator;
mod parser;
mod types;

fn main() {
    println!("Hello, world!");
}
