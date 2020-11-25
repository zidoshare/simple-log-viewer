use crate::mmap::MemoryMap;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Error;
use std::str;

/// 日志文件抽象,主要api入口
pub struct LogMap {
    file: File,
    map: MemoryMap,
    lines: BTreeMap<usize, &'static str>,
}

impl LogMap {
    pub fn new(file: &File) -> Result<LogMap, Error> {
        let result = LogMap {
            file: file.try_clone()?,
            map: MemoryMap::new(file, 0, file.metadata()?.len() as usize)?,
            lines: BTreeMap::new(),
        };
        result.resolve_lines();
        Ok(result)
    }
    fn resolve_lines(self: &Self) {
        // 1G以上文件开启多线程
        if self.file.metadata().unwrap().len() > 1024 * 1024 * 1024 {}
    }
    pub fn get_lines_count(self: &Self) -> usize {
        self.lines.len()
    }
    fn get_line(self: &Self, line_start: usize, line_end: usize) -> &'static str {
        str::from_utf8(self.lines[line_start..line_end])
    }
}
