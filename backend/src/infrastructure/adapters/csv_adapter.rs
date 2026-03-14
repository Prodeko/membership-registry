use csv::WriterBuilder;

use crate::application::ports::data_export_port::{DataExportError, DataExportPort, TabularData};

pub struct CsvAdapter;

impl DataExportPort for CsvAdapter {
    fn serialize(&self, data: &TabularData) -> Result<Vec<u8>, DataExportError> {
        let mut wtr = WriterBuilder::new().from_writer(Vec::new());

        wtr.write_record(&data.headers)
            .map_err(|e| DataExportError::Serialization(e.to_string()))?;

        for row in &data.rows {
            wtr.write_record(row)
                .map_err(|e| DataExportError::Serialization(e.to_string()))?;
        }

        wtr.into_inner()
            .map_err(|e| DataExportError::Serialization(e.to_string()))
    }

    fn content_type(&self) -> &str {
        "text/csv"
    }

    fn file_extension(&self) -> &str {
        "csv"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_produces_valid_csv_with_headers() {
        let adapter = CsvAdapter;
        let data = TabularData {
            headers: vec!["name".into(), "age".into()],
            rows: vec![
                vec!["Alice".into(), "30".into()],
                vec!["Bob".into(), "25".into()],
            ],
        };

        let result = adapter.serialize(&data).expect("serialization should succeed");
        let output = String::from_utf8(result).expect("valid utf-8");

        assert_eq!(output, "name,age\nAlice,30\nBob,25\n");
    }

    #[test]
    fn serialize_empty_data() {
        let adapter = CsvAdapter;
        let data = TabularData {
            headers: vec!["col1".into(), "col2".into()],
            rows: vec![],
        };

        let result = adapter.serialize(&data).expect("serialization should succeed");
        let output = String::from_utf8(result).expect("valid utf-8");

        assert_eq!(output, "col1,col2\n");
    }

    #[test]
    fn serialize_escapes_special_characters() {
        let adapter = CsvAdapter;
        let data = TabularData {
            headers: vec!["field".into()],
            rows: vec![vec!["value with, comma".into()], vec!["value with \"quotes\"".into()]],
        };

        let result = adapter.serialize(&data).expect("serialization should succeed");
        let output = String::from_utf8(result).expect("valid utf-8");

        assert!(output.contains("\"value with, comma\""));
        assert!(output.contains("\"value with \"\"quotes\"\"\""));
    }
}
