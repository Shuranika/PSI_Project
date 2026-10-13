use crate::{error::AppError, models::ResultRecord};
use chrono::Local;
use log::warn;
use std::path::Path;
use umya_spreadsheet::{reader::xlsx::read, writer::xlsx::write, Spreadsheet};

const HEADERS: [&str; 6] = [
    "Группа",
    "Фамилия",
    "Имя",
    "Номер билета",
    "Дата и время",
    "Повтор",
];

fn workbook(path: &Path) -> Result<Spreadsheet, AppError> {
    if path.exists() {
        read(path).map_err(|error| {
            warn!("Could not read results workbook: {error:?}");
            AppError::Save("results.xlsx is unavailable or locked".to_string())
        })
    } else {
        Ok(umya_spreadsheet::new_file())
    }
}

fn ensure_headers(workbook: &mut Spreadsheet) -> Result<(), AppError> {
    let sheet = workbook
        .get_sheet_mut(&0)
        .ok_or_else(|| AppError::InvalidWorkbook("workbook has no worksheet".to_string()))?;
    for (index, header) in HEADERS.iter().enumerate() {
        let column = (b'A' + index as u8) as char;
        sheet.get_cell_mut(format!("{column}1")).set_value(*header);
    }
    Ok(())
}

pub fn ensure_results_file(path: &Path) -> Result<(), AppError> {
    if path.exists() {
        return Ok(());
    }
    let mut workbook = umya_spreadsheet::new_file();
    ensure_headers(&mut workbook)?;
    write(&workbook, path).map_err(|error| {
        warn!("Could not create results workbook: {error:?}");
        AppError::Save("results.xlsx could not be created.".to_string())
    })
}

fn text(sheet: &umya_spreadsheet::structs::Worksheet, coordinate: &str) -> String {
    sheet
        .get_cell(coordinate)
        .map(|cell| cell.get_value().to_string())
        .unwrap_or_default()
}

pub fn find_first_ticket_for_student(
    path: &Path,
    group: &str,
    last_name: &str,
    first_name: &str,
) -> Result<Option<u32>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    let workbook = read(path)
        .map_err(|error| AppError::Save(format!("cannot inspect results.xlsx: {error:?}")))?;
    let sheet = workbook
        .get_sheet(&0)
        .ok_or_else(|| AppError::InvalidWorkbook("workbook has no worksheet".to_string()))?;
    for row in 2..=sheet.get_highest_row() {
        if text(sheet, &format!("A{row}")) == group
            && text(sheet, &format!("B{row}")) == last_name
            && text(sheet, &format!("C{row}")) == first_name
        {
            if let Ok(number) = text(sheet, &format!("D{row}")).parse::<u32>() {
                return Ok(Some(number));
            }
            warn!("Invalid historical ticket number at row {row}");
        }
    }
    Ok(None)
}

pub fn append_result(
    path: &Path,
    group: &str,
    last_name: &str,
    first_name: &str,
    ticket_number: u32,
    is_repeat: bool,
) -> Result<(), AppError> {
    let mut workbook = workbook(path)?;
    ensure_headers(&mut workbook)?;
    let sheet = workbook
        .get_sheet_mut(&0)
        .ok_or_else(|| AppError::InvalidWorkbook("workbook has no worksheet".to_string()))?;
    let row = sheet.get_highest_row().max(1) + 1;
    let values = [
        group.to_string(),
        last_name.to_string(),
        first_name.to_string(),
        ticket_number.to_string(),
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        if is_repeat {
            "да".to_string()
        } else {
            "нет".to_string()
        },
    ];
    for (index, value) in values.iter().enumerate() {
        let column = (b'A' + index as u8) as char;
        sheet
            .get_cell_mut(format!("{column}{row}"))
            .set_value(value);
    }
    write(&workbook, path).map_err(|error| {
        warn!("Could not save results workbook: {error:?}");
        AppError::Save("results.xlsx could not be written. Close it and retry.".to_string())
    })
}

pub fn save_result(
    path: &Path,
    pending: &crate::models::PendingResult,
) -> Result<ResultRecord, AppError> {
    append_result(
        path,
        &pending.group,
        &pending.student.last_name,
        &pending.student.first_name,
        pending.ticket_number,
        pending.is_repeat,
    )?;
    Ok(ResultRecord {
        group: pending.group.clone(),
        student: pending.student.clone(),
        ticket_number: pending.ticket_number,
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        is_repeat: pending.is_repeat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn appends_and_reuses_first_ticket() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("results.xlsx");
        append_result(&path, "G", "Doe", "Jane", 7, false).unwrap();
        append_result(&path, "G", "Doe", "Jane", 9, true).unwrap();
        assert_eq!(
            find_first_ticket_for_student(&path, "G", "Doe", "Jane").unwrap(),
            Some(7)
        );
        let workbook = read(&path).unwrap();
        let sheet = workbook.get_sheet(&0).unwrap();
        assert_eq!(sheet.get_cell("F3").unwrap().get_value(), "да");
        assert_eq!(sheet.get_cell("A2").unwrap().get_value(), "G");
    }

    #[test]
    fn initializes_results_workbook_with_headers_without_overwriting_existing_rows() {
        let directory = tempdir().expect("temporary directory");
        let path = directory.path().join("results.xlsx");
        ensure_results_file(&path).unwrap();
        append_result(&path, "G", "Doe", "Jane", 7, false).unwrap();

        ensure_results_file(&path).unwrap();
        let workbook = read(&path).unwrap();
        let sheet = workbook.get_sheet(&0).unwrap();
        assert_eq!(sheet.get_cell("A1").unwrap().get_value(), "Группа");
        assert_eq!(sheet.get_cell("F1").unwrap().get_value(), "Повтор");
        assert_eq!(sheet.get_cell("A2").unwrap().get_value(), "G");
    }
}
