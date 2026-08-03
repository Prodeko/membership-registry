use csv::ReaderBuilder;

use crate::application::ports::tabular_parse_port::{
    ParsedCsv, TabularParseError, TabularParsePort,
};

pub struct CsvParseAdapter;

impl TabularParsePort for CsvParseAdapter {
    fn parse(&self, bytes: &[u8]) -> Result<ParsedCsv, TabularParseError> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .flexible(false)
            .from_reader(bytes);

        let headers = rdr
            .headers()
            .map_err(|e| TabularParseError::Malformed(e.to_string()))?
            .iter()
            .map(|h| h.trim().to_string())
            .collect();

        let mut records = Vec::new();
        for result in rdr.records() {
            let rec = result.map_err(|e| TabularParseError::Malformed(e.to_string()))?;
            records.push(rec.iter().map(str::to_string).collect());
        }

        Ok(ParsedCsv { headers, records })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headers_and_rows() {
        let csv = b"email,first_name\na@b.com,Alice\nc@d.com,Bob\n";
        let parsed = CsvParseAdapter.parse(csv).expect("valid csv");
        assert_eq!(parsed.headers, vec!["email", "first_name"]);
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0], vec!["a@b.com", "Alice"]);
    }

    #[test]
    fn rejects_ragged_rows() {
        // second data row has too few columns
        let csv = b"email,first_name\na@b.com,Alice\nc@d.com\n";
        assert!(matches!(
            CsvParseAdapter.parse(csv),
            Err(TabularParseError::Malformed(_))
        ));
    }

    #[test]
    fn preserves_utf8_values() {
        let csv = "email,home_municipality\na@b.com,Jyväskylä\n".as_bytes();
        let parsed = CsvParseAdapter.parse(csv).expect("valid csv");
        assert_eq!(parsed.records[0][1], "Jyväskylä");
    }
}
