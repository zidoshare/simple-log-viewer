use crate::mmap::MemoryMap;
use crate::range::Range;
use regex::bytes::RegexBuilder;
use std::cmp::min;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Error;

pub struct RegexConfig {
    pub case_insensitive: bool,
    pub case_smart: bool,
    pub multi_line: bool,
    pub dot_matches_new_line: bool,
    pub swap_greed: bool,
    pub ignore_whitespace: bool,
    pub unicode: bool,
    pub octal: bool,
    pub size_limit: usize,
    pub dfa_size_limit: usize,
    pub nest_limit: u32,
}

impl Default for RegexConfig {
    fn default() -> RegexConfig {
        RegexConfig {
            case_insensitive: false,
            case_smart: false,
            multi_line: false,
            dot_matches_new_line: false,
            swap_greed: false,
            ignore_whitespace: false,
            unicode: true,
            octal: false,
            // These size limits are much bigger than what's in the regex
            // crate.
            size_limit: 100 * (1 << 20),
            dfa_size_limit: 1000 * (1 << 20),
            nest_limit: 250,
        }
    }
}

struct RegexMatcher {
    config: RegexConfig,
}

pub struct Searcher<'a> {
    //当前搜索位置
    pos: usize,
    buf: &'a [u8],
}

impl<'a> Searcher<'a> {
    pub fn new(buf: &'a [u8]) -> Searcher {
        Searcher { pos: 0, buf }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }
}

/// 日志文件抽象,主要api入口
pub struct LogMap {
    file: File,
    map: MemoryMap,
    lines: BTreeMap<usize, usize>,
}

impl LogMap {
    pub fn new(file: &File) -> Result<LogMap, Error> {
        let mmap = MemoryMap::new(file, 0, file.metadata()?.len() as usize)?;
        let mut lines = BTreeMap::new();
        let mut line_num = 0;
        lines.insert(0, 0);
        mmap.iter().enumerate().for_each(|(index, c)| {
            if *c == b'\n' {
                lines.insert(line_num, index);
                line_num = line_num + 1;
            }
        });
        let result = LogMap {
            file: file.try_clone()?,
            map: mmap,
            lines,
        };
        Ok(result)
    }
    pub fn get_lines_count(self: &Self) -> usize {
        self.lines.len()
    }

    pub fn find_in_iter(
        &self,
        ranges: &Vec<Range>,
        pattern: &str,
        config: &RegexConfig,
    ) -> Vec<Range> {
        let regex = RegexBuilder::new(pattern)
            .nest_limit(config.nest_limit)
            .octal(config.octal)
            .multi_line(config.multi_line)
            .dot_matches_new_line(config.dot_matches_new_line)
            .unicode(config.unicode)
            .size_limit(config.size_limit)
            .dfa_size_limit(config.dfa_size_limit)
            .build()
            .unwrap();
        ranges
            .iter()
            .map(|range| {
                regex
                    .find_iter(&self.map[*range])
                    .map(|m| Range::new(m.start(), m.end()))
                    .collect()
            })
            .fold(Vec::new(), |p, next| [p, next].concat())
    }

    pub fn get_line(&mut self, line_start: usize, count: usize) -> Vec<String> {
        if line_start >= self.map.len() {
            Vec::new()
        } else {
            let mut result: Vec<String> = Vec::with_capacity(count);
            for i in line_start..min(line_start + count, self.lines.len()) {
                let line = unsafe {
                    String::from_raw_parts(
                        self.map.ptr().clone() as *mut u8,
                        self.lines[&(i + 1)] - 1 - self.lines[&i],
                        10000,
                    )
                };
                result.push(line);
            }
            result
        }
    }
}
