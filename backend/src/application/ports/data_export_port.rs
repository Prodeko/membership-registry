use std::fmt;

pub struct TabularData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl TabularData {
    pub fn from_exportable<T: Exportable>(items: &[T]) -> Self {
        Self {
            headers: T::headers().into_iter().map(String::from).collect(),
            rows: items.iter().map(|item| item.to_row()).collect(),
        }
    }
}

pub trait Exportable {
    fn headers() -> Vec<&'static str>;
    fn to_row(&self) -> Vec<String>;
}

#[derive(Debug)]
pub enum DataExportError {
    Serialization(String),
}

impl fmt::Display for DataExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(msg) => write!(f, "Export serialization error: {msg}"),
        }
    }
}

pub trait DataExportPort: Send + Sync {
    fn serialize(&self, data: &TabularData) -> Result<Vec<u8>, DataExportError>;
    fn content_type(&self) -> &str;
    fn file_extension(&self) -> &str;
}

pub struct ExportedData {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub file_extension: String,
}
