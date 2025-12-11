/// Представляет позицию в исходном файле
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub fn new(line: usize, column: usize) -> Self {
        Location { line, column }
    }
}

/// Представляет диапазон символов в исходном коде
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    /// Объединяет два span'а в один, охватывающий оба
    pub fn merge(self, other: Span) -> Self {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Преобразует байтовые позиции в номера строк и столбцов
    pub fn to_location(&self, source: &str) -> (Location, Location) {
        let start_loc = byte_offset_to_location(source, self.start);
        let end_loc = byte_offset_to_location(source, self.end);
        (start_loc, end_loc)
    }

    /// Получает текст, соответствующий этому span'у
    pub fn text<'a>(&self, source: &'a str) -> &'a str {
        &source[self.start..self.end]
    }
}

impl Default for Span {
    fn default() -> Self {
        Span { start: 0, end: 0 }
    }
}

/// Преобразует байтовый offset в Location (строка и столбец)
fn byte_offset_to_location(source: &str, offset: usize) -> Location {
    let mut line = 0;
    let mut column = 0;

    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    Location { line, column }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_merge() {
        let span1 = Span::new(5, 10);
        let span2 = Span::new(8, 15);
        let merged = span1.merge(span2);

        assert_eq!(merged.start, 5);
        assert_eq!(merged.end, 15);
    }

    #[test]
    fn test_byte_offset_to_location() {
        let source = "line 1\nline 2\nline 3";

        let loc = byte_offset_to_location(source, 0);
        assert_eq!(loc.line, 0);
        assert_eq!(loc.column, 0);

        let loc = byte_offset_to_location(source, 7); // начало "line 2"
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 0);

        let loc = byte_offset_to_location(source, 10); // 'n' в "line 2"
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 3);
    }

    #[test]
    fn test_span_text() {
        let source = "hello world";
        let span = Span::new(0, 5);
        assert_eq!(span.text(source), "hello");

        let span = Span::new(6, 11);
        assert_eq!(span.text(source), "world");
    }
}
