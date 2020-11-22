use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader, Error};
use std::os::raw::c_void;
use std::{env, fs, io, ptr, slice, time};

#[cfg(windows)]
use std::mem;
use std::ops::Deref;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use winapi::um::memoryapi::{CreateFileMappingW, MapViewOfFile};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 0 {
        panic!("please provide file path");
    }
    println!("The file path is {}", args[1]);
    let start_time = time::SystemTime::now();
    let filename = &args[1];
    let file = fs::OpenOptions::new().read(true).open(filename)?;
    println!(
        "文件大小为：{}M",
        file.metadata().unwrap().len() / 1024 / 1024
    );
    let chunk = MemoryMap::new(&file, 0, file.metadata()?.len() as usize).unwrap();
    let mut lines: Vec<String> = Vec::new();
    let mut index = 0;
    for i in index..chunk.len {
        if chunk[i] == ('\n' as u8) {
            lines.push(
                std::str::from_utf8(&chunk[index..i])
                    .unwrap()
                    .parse()
                    .unwrap(),
            );
            index = i;
        }
    }

    println!(
        "使用 mmap耗时:{}s",
        time::SystemTime::now()
            .duration_since(start_time)
            .expect("error times")
            .as_secs()
    );
    lines.clear();
    let start_time = time::SystemTime::now();
    let mut reader: BufReader<File> = BufReader::new(file);
    assert!(reader.buffer().is_empty());
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
        "使用 read_line耗时:{}s",
        time::SystemTime::now()
            .duration_since(start_time)
            .expect("error times")
            .as_secs()
    );

    //一系列and规则 对应相应的下标指针数组
    //预设规则，log level,time
    let patterns = get_patterns();
    let start_time = time::SystemTime::now();
    let mut parent_pattern = Pattern {
        regex: "",
        label: "parent",
        sub: patterns,
        index: lines
            .iter()
            .enumerate()
            .map(|(index, _)| Cursor {
                line: index,
                index: 0,
            })
            .collect(),
    };
    resolve_pattern(&lines, &mut parent_pattern);
    println!(
        "resolve duration since:{}s",
        time::SystemTime::now()
            .duration_since(start_time)
            .expect("error times")
            .as_secs()
    );
    Ok(())
}

fn resolve_pattern(lines: &Vec<String>, pattern: &mut Pattern) {
    if !pattern.index.is_empty() {
        for ind in &pattern.index {
            for sub_pattern in &mut pattern.sub {
                let line = &lines[ind.line];
                let regex = Regex::new(&sub_pattern.regex).unwrap();
                match regex.shortest_match(&line) {
                    Some(index) => {
                        sub_pattern.index.push(Cursor {
                            line: ind.line,
                            index,
                        });
                    }
                    _ => {}
                }
                resolve_pattern(lines, sub_pattern);
            }
        }
    }
}

fn get_patterns<'a>() -> Vec<Pattern<'a>> {
    let mut patterns: Vec<Pattern> = Vec::new();
    patterns.push(Pattern::new("xxx", "xxx"));
    patterns
}

struct MemoryMap {
    #[warn(dead_code)]
    file: Option<File>,
    ptr: *mut c_void,
    len: usize,
}
impl Deref for MemoryMap {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr as *mut u8, self.len) }
    }
}

impl AsRef<[u8]> for MemoryMap {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.deref()
    }
}

#[cfg(unix)]
fn get_fd(file: &fs::File) -> libc::c_int {
    file.as_raw_fd()
}

#[cfg(windows)]
impl MemoryMap {
    fn new(file: &File, offset: u64, len: usize) -> io::Result<MemoryMap> {
        use winapi::shared::basetsd::SIZE_T;
        use winapi::shared::minwindef::DWORD;
        use winapi::um::handleapi::CloseHandle;
        let alignment = offset % allocation_granularity() as u64;
        let aligned_offset = offset - alignment as u64;
        let aligned_len = len + alignment as usize;
        unsafe {
            let handle = CreateFileMappingW(
                file.as_raw_handle(),
                ptr::null_mut(),
                winapi::um::winnt::PAGE_READONLY,
                0,
                0,
                ptr::null(),
            );
            if handle == ptr::null_mut() {
                return Err(io::Error::last_os_error());
            }
            let ptr = MapViewOfFile(
                handle,
                winapi::um::memoryapi::FILE_MAP_READ,
                (aligned_offset >> 16 >> 16) as DWORD,
                (aligned_offset & 0xffffffff) as DWORD,
                aligned_len as SIZE_T,
            );
            CloseHandle(handle);
            if ptr == ptr::null_mut() {
                Err(io::Error::last_os_error())
            } else {
                Ok(MemoryMap {
                    file: Some(file.try_clone()?),
                    ptr: ptr.offset(alignment as isize),
                    len: len as usize,
                })
            }
        }
    }
}

#[cfg(windows)]
fn allocation_granularity() -> usize {
    use winapi::um::sysinfoapi::GetSystemInfo;
    unsafe {
        let mut info = mem::zeroed();
        GetSystemInfo(&mut info);
        return info.dwAllocationGranularity as usize;
    }
}

#[cfg(windows)]
impl Drop for MemoryMap {
    fn drop(&mut self) {
        use winapi::um::memoryapi::UnmapViewOfFile;
        let alignment = self.ptr as usize % allocation_granularity();
        unsafe {
            let ptr = self.ptr.offset(-(alignment as isize));
            assert_ne!(
                UnmapViewOfFile(ptr),
                0,
                "unable to unmap mmap: {}",
                io::Error::last_os_error()
            );
        }
    }
}

#[derive(Debug)]
struct Pattern<'a> {
    // 匹配表达式
    regex: &'a str,
    //显示标签名
    label: &'a str,
    //对应index
    index: Vec<Cursor>,
    //子表达式
    sub: Vec<Pattern<'a>>,
}

impl Pattern<'_> {
    fn new<'a>(regex: &'static str, label: &'static str) -> Pattern<'a> {
        Pattern {
            regex,
            label,
            sub: Vec::new(),
            index: Vec::new(),
        }
    }
}
// 搜索游标，对应行号和行内位置
#[derive(Debug)]
struct Cursor {
    //对应行号
    line: usize,
    //行中匹配的index
    index: usize,
}
