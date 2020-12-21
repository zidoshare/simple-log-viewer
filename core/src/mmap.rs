use std::fs::File;
use std::io;
use std::ops::Deref;
use std::os::raw::c_void;
use std::ptr;
use std::slice;

#[cfg(windows)]
use std::mem;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;
#[cfg(windows)]
use winapi::um::memoryapi::{CreateFileMappingW, MapViewOfFile};

#[cfg(unix)]
use std::os::unix::io::AsRawFd;

/// 跨平台mmap实现（只读）
pub struct MemoryMap {
    ptr: *mut c_void,
    len: usize,
}
impl MemoryMap {
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn ptr(&self) -> *const u8 {
        self.ptr as *mut u8
    }
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

#[cfg(windows)]
impl MemoryMap {
    pub fn new(file: &File, offset: u64, len: usize) -> io::Result<Self> {
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
            // windows下必须先关闭文件句柄，否则后续无法关闭mmap这一点与unix不同
            CloseHandle(handle);
            if ptr == ptr::null_mut() {
                Err(io::Error::last_os_error())
            } else {
                Ok(MemoryMap {
                    ptr: ptr.offset(alignment as isize),
                    len: len as usize,
                })
            }
        }
    }
}

#[cfg(unix)]
fn page_size() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}

#[cfg(unix)]
impl MemoryMap {
    pub fn new(file: &File, offset: u64, len: usize) -> io::Result<Self> {
        let alignment = offset % page_size() as u64;
        let aligned_offset = offset - alignment;
        let aligned_len = len + alignment as usize;
        if aligned_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "memory map must have a non-zero length",
            ));
        }
        unsafe {
            let ptr = libc::mmap(
                ptr::null_mut(),
                aligned_len as libc::size_t,
                libc::PROT_READ,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                aligned_offset as libc::off_t,
            );
            if ptr == libc::MAP_FAILED {
                Err(io::Error::last_os_error())
            } else {
                Ok(MemoryMap {
                    ptr: ptr.offset(alignment as isize),
                    len,
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

#[cfg(unix)]
impl Drop for MemoryMap {
    fn drop(&mut self) {
        let alignment = self.ptr as usize % page_size();
        unsafe {
            assert!(
                libc::munmap(
                    self.ptr.offset(-(alignment as isize)),
                    (self.len + alignment) as libc::size_t,
                ) == 0,
                "unable to unmap map:{}",
                io::Error::last_os_error()
            )
        }
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
#[cfg(test)]
mod tests {
    use super::MemoryMap;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    fn do_with_random_file<F, R>(f: F) -> std::io::Result<R>
    where
        F: Fn(&File) -> std::io::Result<R>,
    {
        let file_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(12)
            .collect();
        let mut dir = env::temp_dir();
        dir.push(file_name + ".txt");
        println!("temp file path:{:?}", dir);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .read(true)
            .open(&dir)?;
        let r = f(&file).map_err(|r| {
            if let Some(err) = std::fs::remove_file(&dir).err() {
                eprintln!("{}", err);
            }
            r
        })?;
        Ok(r)
    }

    fn fill_content(f: &File, content: &str) -> std::io::Result<()> {
        let mut bw = std::io::BufWriter::new(f);
        bw.write(content.as_bytes())?;
        bw.flush()?;
        Ok(())
    }

    fn fill_random_content(f: &File, bytes_count: usize) -> std::io::Result<String> {
        let mut bw = std::io::BufWriter::new(f);
        let mut result: String = String::from("");
        for i in 0..100 {
            let content: String = rand::thread_rng()
                .sample_iter(Alphanumeric)
                .take(bytes_count / 100)
                .collect();
            bw.write(content.as_bytes())?;
            result.push_str(&content);
            println!("writing: {}%", i);
        }
        bw.flush()?;
        Ok(result)
    }

    #[test]
    fn test_create_memory_map() {
        assert!(do_with_random_file(|file| {
            fill_content(file, "123")?;
            let mmap = MemoryMap::new(file, 0, file.metadata()?.len() as usize)?;
            let content = String::from_utf8_lossy(&mmap);
            if content == "123" {
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .unwrap());
    }
    #[test]
    fn test_big_memory_map() {
        assert!(do_with_random_file(|file| {
            let target = fill_random_content(file, 1024 * 1024 * 100)?;
            let mmap = MemoryMap::new(file, 0, file.metadata()?.len() as usize)?;
            let content = String::from_utf8_lossy(&mmap);
            if content == target {
                Ok(true)
            } else {
                Ok(false)
            }
        })
        .unwrap());
    }
}
