use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::application::ports::data_export_port::{DataExportError, Sheet, WorkbookExportPort};

/// Writes `.xlsx` workbooks, which open the same in Excel in every locale
/// (unlike CSV, whose delimiter Finnish Excel reads differently).
pub struct XlsxAdapter;

fn to_export_error(e: XlsxError) -> DataExportError {
    DataExportError::Serialization(e.to_string())
}

impl WorkbookExportPort for XlsxAdapter {
    fn write(&self, sheets: &[Sheet]) -> Result<Vec<u8>, DataExportError> {
        let mut workbook = Workbook::new();
        let bold = Format::new().set_bold();
        for sheet in sheets {
            let worksheet = workbook.add_worksheet();
            worksheet.set_name(&sheet.name).map_err(to_export_error)?;
            for (col, header) in sheet.data.headers.iter().enumerate() {
                worksheet
                    .write_string_with_format(0, col as u16, header, &bold)
                    .map_err(to_export_error)?;
            }
            for (row, values) in sheet.data.rows.iter().enumerate() {
                for (col, value) in values.iter().enumerate() {
                    worksheet
                        .write_string(row as u32 + 1, col as u16, value)
                        .map_err(to_export_error)?;
                }
            }
            worksheet.autofit();
        }
        workbook.save_to_buffer().map_err(to_export_error)
    }

    fn content_type(&self) -> &str {
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    }

    fn file_extension(&self) -> &str {
        "xlsx"
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::application::ports::data_export_port::TabularData;

    #[test]
    fn writes_a_zip_based_workbook_with_every_sheet() {
        let sheets = vec![
            Sheet {
                name: "Summary".to_string(),
                data: TabularData {
                    headers: vec!["Name".to_string()],
                    rows: vec![vec!["Äänestäjä Öljynen".to_string()]],
                },
            },
            Sheet {
                name: "Log".to_string(),
                data: TabularData {
                    headers: vec!["Time".to_string()],
                    rows: vec![],
                },
            },
        ];
        let bytes = XlsxAdapter.write(&sheets).unwrap();
        // .xlsx files are zip archives.
        assert_eq!(&bytes[..2], b"PK");
    }
}
