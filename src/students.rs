use crate::{error::AppError, models::Student};
use calamine::{open_workbook_auto, Reader};
use log::warn;
use std::path::Path;

pub fn load_groups(path: &Path) -> Result<Vec<String>, AppError> {
    if !path.exists() {
        return Err(AppError::MissingFile(path.display().to_string()));
    }
    let workbook =
        open_workbook_auto(path).map_err(|error| AppError::InvalidWorkbook(error.to_string()))?;
    Ok(workbook.sheet_names().to_vec())
}

pub fn load_students(path: &Path, group: &str) -> Result<Vec<Student>, AppError> {
    if !path.exists() {
        return Err(AppError::MissingFile(path.display().to_string()));
    }
    let mut workbook =
        open_workbook_auto(path).map_err(|error| AppError::InvalidWorkbook(error.to_string()))?;
    let range = workbook
        .worksheet_range(group)
        .map_err(|error| AppError::InvalidWorkbook(error.to_string()))?;

    let mut students = Vec::new();
    for row in range.rows().skip(1) {
        let last_name = row
            .first()
            .map(ToString::to_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        let first_name = row
            .get(1)
            .map(ToString::to_string)
            .unwrap_or_default()
            .trim()
            .to_string();
        if last_name.is_empty() && first_name.is_empty() {
            continue;
        }
        if last_name.is_empty() || first_name.is_empty() {
            warn!("Ignoring unreadable student row in worksheet {group}");
            continue;
        }
        students.push(Student {
            last_name,
            first_name,
        });
    }
    Ok(students)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn loads_students_from_selected_group_and_ignores_empty_rows() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("students.xlsx");
        let mut workbook = umya_spreadsheet::new_file();
        workbook.get_sheet_mut(&0).unwrap().set_name("Group A");
        let sheet = workbook.get_sheet_mut(&0).unwrap();
        sheet.get_cell_mut("A1").set_value("Last name");
        sheet.get_cell_mut("B1").set_value("First name");
        sheet.get_cell_mut("A2").set_value("Doe");
        sheet.get_cell_mut("B2").set_value("Jane");
        sheet.get_cell_mut("A3").set_value("");
        sheet.get_cell_mut("B3").set_value("");
        umya_spreadsheet::writer::xlsx::write(&workbook, &path).unwrap();

        assert_eq!(load_groups(&path).unwrap(), vec!["Group A"]);
        assert_eq!(
            load_students(&path, "Group A").unwrap(),
            vec![Student {
                last_name: "Doe".to_string(),
                first_name: "Jane".to_string()
            }]
        );
    }

    #[test]
    fn empty_group_returns_no_students() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("students.xlsx");
        let mut workbook = umya_spreadsheet::new_file();
        workbook.get_sheet_mut(&0).unwrap().set_name("Empty");
        let sheet = workbook.get_sheet_mut(&0).unwrap();
        sheet.get_cell_mut("A1").set_value("Last name");
        sheet.get_cell_mut("B1").set_value("First name");
        umya_spreadsheet::writer::xlsx::write(&workbook, &path).unwrap();

        assert!(load_students(&path, "Empty").unwrap().is_empty());
    }
}
