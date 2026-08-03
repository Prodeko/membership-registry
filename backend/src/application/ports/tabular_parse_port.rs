#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCsv {
    pub headers: Vec<String>,
    pub records: Vec<Vec<String>>,
}

#[derive(Debug)]
pub enum TabularParseError {
    Malformed(String),
}

pub trait TabularParsePort: Send + Sync {
    /// Parse CSV bytes into a header row plus equal-width data records.
    /// Header cells are trimmed; data cells are preserved verbatim so
    /// downstream domain constructors (`Email`, `AttributeValue`) enforce
    /// their own whitespace rules.
    fn parse(&self, bytes: &[u8]) -> Result<ParsedCsv, TabularParseError>;
}
