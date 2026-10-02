use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: u32, end: u32) -> Span {
        debug_assert!(start <= end, "span start must not exceed end");
        Span { file, start, end }
    }

    pub fn point(file: FileId, offset: u32) -> Span {
        Span { file, start: offset, end: offset }
    }

    pub fn to(self, other: Span) -> Span {
        debug_assert_eq!(self.file, other.file, "cannot merge spans across files");
        Span {
            file: self.file,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn len(self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn range(self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Span(#{}, {}..{})", self.file.0, self.start, self.end)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

pub struct SourceFile {
    pub name: String,
    pub src: String,

    line_starts: Vec<u32>,
}

impl SourceFile {
    fn new(name: String, src: String) -> SourceFile {
        let mut line_starts = vec![0u32];
        for (i, b) in src.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        SourceFile { name, src, line_starts }
    }

    fn location(&self, offset: u32) -> Location {

        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let line_start = self.line_starts[line_idx];

        let col = self.src[line_start as usize..offset as usize].chars().count() as u32;
        Location { line: line_idx as u32 + 1, column: col + 1 }
    }

    fn line_text(&self, line: u32) -> &str {
        let idx = (line - 1) as usize;
        let start = self.line_starts[idx] as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&s| s as usize)
            .unwrap_or(self.src.len());
        self.src[start..end].trim_end_matches(['\n', '\r'])
    }
}

#[derive(Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> SourceMap {
        SourceMap { files: Vec::new() }
    }

    pub fn add(&mut self, name: impl Into<String>, src: impl Into<String>) -> FileId {
        let id = FileId(self.files.len() as u32);
        self.files.push(SourceFile::new(name.into(), src.into()));
        id
    }

    pub fn file(&self, id: FileId) -> &SourceFile {
        &self.files[id.0 as usize]
    }

    pub fn name(&self, id: FileId) -> &str {
        &self.file(id).name
    }

    pub fn source(&self, id: FileId) -> &str {
        &self.file(id).src
    }

    pub fn snippet(&self, span: Span) -> &str {
        &self.file(span.file).src[span.range()]
    }

    pub fn location(&self, span: Span) -> Location {
        self.file(span.file).location(span.start)
    }

    pub fn location_at(&self, file: FileId, offset: u32) -> Location {
        self.file(file).location(offset)
    }

    pub fn line_text(&self, file: FileId, line: u32) -> &str {
        self.file(file).line_text(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_merge() {
        let a = Span::new(FileId(0), 2, 5);
        let b = Span::new(FileId(0), 10, 12);
        let m = a.to(b);
        assert_eq!((m.start, m.end), (2, 12));
        assert_eq!(m.len(), 10);
    }

    #[test]
    fn line_column_ascii() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.exo", "abc\ndef\nghi");

        assert_eq!(sm.location_at(f, 0), Location { line: 1, column: 1 });

        assert_eq!(sm.location_at(f, 4), Location { line: 2, column: 1 });

        assert_eq!(sm.location_at(f, 6), Location { line: 2, column: 3 });
        assert_eq!(sm.line_text(f, 2), "def");
    }

    #[test]
    fn column_counts_unicode_scalars() {
        let mut sm = SourceMap::new();

        let f = sm.add("t.exo", "🍋x");
        assert_eq!(sm.location_at(f, 4), Location { line: 1, column: 2 });
    }

    #[test]
    fn snippet_extracts_range() {
        let mut sm = SourceMap::new();
        let f = sm.add("t.exo", "nilai x = 10");
        let sp = Span::new(f, 6, 7);
        assert_eq!(sm.snippet(sp), "x");
    }
}
