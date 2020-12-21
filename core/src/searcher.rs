use crate::range::Range;
pub trait Matcher {
    type Error: std::fmt::Display;

    fn find_at(&self, content: &[u8], at: usize) -> Result<Option<Range>, Self::Error>;

    fn find(&self, content: &[u8]) -> Result<Option<Range>, Self::Error> {
        self.find_at(content, 0)
    }

    fn find_iter<F>(&self, content: &[u8], mut matched: F) -> Result<(), Self::Error>
    where
        F: FnMut(Range) -> bool,
    {
        self.try_find_iter(content, |m| Ok(matched(m)))
            .map(|r: Result<(), ()>| r.unwrap())
    }

    fn try_find_iter<F, E>(
        &self,
        content: &[u8],
        mut matched: F,
    ) -> Result<Result<(), E>, Self::Error>
    where
        F: FnMut(Range) -> Result<bool, E>,
    {
        let mut last_end = 0;
        let mut last_match = None;
        loop {
            if last_end > content.len() {
                return Ok(Ok(()));
            }
            let m = match self.find_at(content, last_end)? {
                None => return Ok(Ok(())),
                Some(m) => m,
            };
            if m.start() == m.end() {
                last_end = m.end() + 1;
                if Some(m.end()) == last_match {
                    continue;
                }
            } else {
                last_end = m.end();
            }
            last_match = Some(m.end());
            match matched(m) {
                Ok(true) => continue,
                Ok(false) => return Ok(Ok(())),
                Err(err) => return Ok(Err(err)),
            }
        }
    }
    fn is_match(&self, content: &[u8]) -> Result<bool, Self::Error> {
        self.is_match_at(content, 0)
    }
    fn is_match_at(&self, content: &[u8], at: usize) -> Result<bool, Self::Error> {
        Ok(self.find_at(content, at)?.is_some())
    }
    fn shortest_match(&self, content: &[u8]) -> Result<Option<usize>, Self::Error> {
        self.shortest_match_at(content, 0)
    }
    fn shortest_match_at(&self, content: &[u8], at: usize) -> Result<Option<usize>, Self::Error> {
        Ok(self.find_at(content, at)?.map(|m| m.end()))
    }
}
