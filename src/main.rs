use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::path::Path;
use std::{env, time};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 0 {
        panic!("please provide file path");
    }
    println!("The file path is {}", args[0]);
    let start_time = time::SystemTime::now();
    let mut reader: BufReader<File> = open(&args[1])?;
    assert!(reader.buffer().is_empty());
    let mut lines: Vec<String> = Vec::new();
    loop {
        let mut line = String::new();
        let len = reader.read_line(&mut line)?;
        if len != 0 {
            lines.push(line);
        } else {
            break;
        }
    }
    println!(
        "duration since:{}s",
        time::SystemTime::now()
            .duration_since(start_time)
            .expect("error times")
            .as_secs()
    );
    //一系列and规则 对应相应的下标指针数组
    //预设规则，log level,time
    let patterns = get_patterns();
    let mut parent_pattern = Pattern {
        regex: String::from(""),
        label: String::from("parent"),
        sub: patterns,
        indx: lines
            .iter()
            .enumerate()
            .map(|(index, _)| Ind {
                line: index,
                index: 0,
            })
            .collect::<Vec<_>>(),
    };
    resolve_pattern(&lines, &mut parent_pattern);
    println!("{:?}", parent_pattern);
    Ok(())
}

fn resolve_pattern(lines: &Vec<String>, pattern: &mut Pattern) {
    if !pattern.indx.is_empty() {
        for ind in &pattern.indx {
            for sub_pattern in &mut pattern.sub {
                let line = &lines[ind.line];
                let regex = Regex::new(&sub_pattern.regex).unwrap();
                match regex.shortest_match(&line) {
                    Some(index) => {
                        sub_pattern.indx.push(Ind {
                            line: ind.line,
                            index: index as i32,
                        });
                    }
                    _ => {}
                }
                resolve_pattern(lines, sub_pattern);
            }
        }
    }
}

//open file
fn open<P: AsRef<Path>>(path: P) -> Result<BufReader<File>, Error> {
    let input = File::open(path)?;
    Ok(BufReader::new(input))
}

fn get_patterns() -> Vec<Pattern> {
    let mut patterns: Vec<Pattern> = Vec::new();
    patterns.push(Pattern {
        regex: String::from("xxx"),
        label: String::from("xxx"),
        sub: Vec::new(),
        indx: Vec::new(),
    });
    patterns
}

#[derive(Debug)]
struct Pattern {
    // 匹配表达式
    regex: String,
    //显示标签名
    label: String,
    //对应index
    indx: Vec<Ind>,
    //子表达式
    sub: Vec<Pattern>,
}

#[derive(Debug)]
struct Ind {
    //对应行号
    line: usize,
    //行中匹配的index
    index: i32,
}
