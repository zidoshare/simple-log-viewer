use std::ops;
/// 表示范围
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Range {
    start: usize,
    end: usize,
}

impl Range {
    #[inline]
    pub fn new(start: usize, end: usize) -> Range {
        assert!(start <= end);
        Range { start, end }
    }

    #[inline]
    pub fn zero(offset: usize) -> Range {
        Range {
            start: offset,
            end: offset,
        }
    }

    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }

    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }

    #[inline]
    pub fn offset(&self, amount: usize) -> Range {
        Range {
            start: self.start.checked_add(amount).unwrap(),
            end: self.end.checked_add(amount).unwrap(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl ops::Index<Range> for [u8] {
    type Output = [u8];

    #[inline]
    fn index(&self, index: Range) -> &[u8] {
        &self[index.start..index.end]
    }
}

impl ops::IndexMut<Range> for [u8] {
    #[inline]
    fn index_mut(&mut self, index: Range) -> &mut [u8] {
        &mut self[index.start..index.end]
    }
}

impl ops::Index<Range> for str {
    type Output = str;

    #[inline]
    fn index(&self, index: Range) -> &str {
        &self[index.start..index.end]
    }
}
