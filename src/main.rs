use chrono::Local;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal;
use rand::Rng;
use std::io::{self, Write};
use std::path::Path;
use umya_spreadsheet::Spreadsheet;

fn main() {
    // Инициализация терминала для работы с crossterm
    let _ = terminal::enable_raw_mode();
    defer_cleanup();

    // Приветствие и подсказка
    println!("╔════════════════════════════════════════╗");
    println!("║   Генератор Экзаменационных Билетов    ║");
    println!("╚════════════════════════════════════════╝");
    println!("\nДля выхода нажмите ESC.\n");

    // Бесконечный цикл обработки студентов
    loop {
        // Перехват нажатия клавиши ESC перед запросом фамилии
        if !wait_for_input() {
            println!("\nДо свидания!");
            break;
        }

        // Ввод фамилии с валидацией
        let last_name = input_field("Last name: ");

        // Ввод имени с валидацией
        let first_name = input_field("First name: ");

        // Генерация случайного номера билета от 1 до 20
        let ticket_number = generate_ticket_number();
        println!("Билет № {}\n", ticket_number);

        // Сохранение данных в Excel с обработкой ошибок
        save_to_excel(&last_name, &first_name, ticket_number);
    }

    // Очистка ресурсов при завершении
    let _ = terminal::disable_raw_mode();
}

/// Очищает raw mode, если программа завершилась из-за panic.
fn defer_cleanup() {
    std::panic::set_hook(Box::new(|_| {
        let _ = terminal::disable_raw_mode();
    }));
}

/// Ожидает нажатие любой клавиши
/// Возвращает true если нажата не-ESC клавиша, false если нажата ESC
fn wait_for_input() -> bool {
    loop {
        // Проверяем наличие события с коротким таймаутом
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                match key_event.code {
                    KeyCode::Esc => return false, // ESC нажата - выход
                    _ => return true,              // Другая клавиша - продолжить
                }
            }
        }
    }
}

/// Запрос ввода данных с валидацией
/// Возвращает непустую строку с обрезанными пробелами
/// Поддерживает ESC для выхода из программы
fn input_field(prompt: &str) -> String {
    loop {
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        
        // Включаем raw mode для обнаружения ESC и управления вводом
        let _ = terminal::enable_raw_mode();

        // Читаем символы используя crossterm и печатаем их
        loop {
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    // Обрабатываем только нажатия клавиш (Press), пропускаем Release и Repeat
                    if key_event.kind != event::KeyEventKind::Press {
                        continue;
                    }
                    
                    match key_event.code {
                        KeyCode::Char(c) => {
                            input.push(c);
                            // Печатаем символ
                            print!("{}", c);
                            io::stdout().flush().unwrap();
                        }
                        KeyCode::Backspace => {
                            if !input.is_empty() {
                                input.pop();
                                // Печатаем Backspace последовательность (удаление символа)
                                print!("\x08 \x08");
                                io::stdout().flush().unwrap();
                            }
                        }
                        KeyCode::Enter => {
                            println!(); // Переводим строку после ввода
                            break;
                        }
                        KeyCode::Esc => {
                            // Выход из программы при нажатии ESC
                            let _ = terminal::disable_raw_mode();
                            println!("\n\nДо свидания!");
                            std::process::exit(0);
                        }
                        _ => {} // Игнорируем другие клавиши
                    }
                }
            }
        }

        // Отключаем raw mode после ввода
        let _ = terminal::disable_raw_mode();

        // Обрезаем пробелы
        let trimmed = input.trim().to_string();

        // Валидация: не принимаем пустые строки
        if trimmed.is_empty() {
            println!("❌ Ошибка: поле не может быть пустым. Попробуйте снова.");
            continue;
        }

        return trimmed;
    }
}

/// Генерирует случайный номер билета от 1 до 20 включительно
fn generate_ticket_number() -> u32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(1..=20)
}

/// Сохраняет запись студента в файл journal.xlsx
/// Если файл не существует - создаёт его с заголовками
/// Если файл существует - добавляет строку в конец
/// Обрабатывает ошибку блокировки файла и предлагает повторную попытку
fn save_to_excel(last_name: &str, first_name: &str, ticket_number: u32) {
    let file_path = "journal.xlsx";
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let path = Path::new(file_path);

    loop {
        // Проверяем существование файла
        let file_exists = path.exists();

        let mut workbook = if file_exists {
            // Открываем существующий файл
            match umya_spreadsheet::reader::xlsx::read(path) {
                Ok(wb) => wb,
                Err(e) => {
                    println!("❌ Ошибка при открытии файла: {:?}", e);
                    println!("   Возможно, журнал открыт в MS Excel.");
                    prompt_retry();
                    continue;
                }
            }
        } else {
            // Создаём новый файл с заголовками
            let mut wb = umya_spreadsheet::new_file();
            
            let sheet = wb.get_sheet_mut(0);
            // Добавляем заголовки: Last name | First name | Номер билета | Дата и время
            sheet.get_cell_mut("A1").set_value("Last name");
            sheet.get_cell_mut("B1").set_value("First name");
            sheet.get_cell_mut("C1").set_value("Номер билета");
            sheet.get_cell_mut("D1").set_value("Дата и время");

            wb
        };

        // Находим последнюю заполненную строку (+ 1 для новой записи)
        let next_row = find_next_empty_row(&workbook);

        // Добавляем новую запись
        let sheet = workbook.get_sheet_mut(0);
        sheet.get_cell_mut(&format!("A{}", next_row)).set_value(last_name);
        sheet.get_cell_mut(&format!("B{}", next_row)).set_value(first_name);
        sheet.get_cell_mut(&format!("C{}", next_row)).set_value(ticket_number.to_string());
        sheet.get_cell_mut(&format!("D{}", next_row)).set_value(&timestamp);

        // Попытка сохранить файл
        match umya_spreadsheet::writer::xlsx::write(&workbook, path) {
            Ok(_) => {
                println!("✓ Запись сохранена в журнал.\n");
                break; // Успешное сохранение - выход из цикла
            }
            Err(e) => {
                // Ошибка при сохранении (возможно, файл заблокирован)
                println!("❌ Ошибка при сохранении файла: {:?}", e);
                println!("   Проверьте, что файл journal.xlsx не открыт в MS Excel");
                prompt_retry();
                // Цикл повторится, попытка записи будет повторена
            }
        }
    }
}

/// Находит номер следующей пустой строки в листе
/// Начинает со строки 2 (так как строка 1 - заголовки)
fn find_next_empty_row(workbook: &Spreadsheet) -> u32 {
    let mut row = 2u32;
    
    // Получаем первый лист
    if let Ok(sheet) = workbook.get_sheet(0) {
        // Ищем первую пустую ячейку в столбце A (используем координату "A{row}")
        loop {
            if let Some(cell) = sheet.get_cell(&format!("A{}", row)) {
                if cell.get_value().is_empty() {
                    break;
                }
            } else {
                // Если ячейка не существует, строка пуста
                break;
            }
            row += 1;
        }
    }
    row
}

/// Выводит сообщение и ждёт нажатия Enter для повторной попытки
/// Поддерживает ESC для выхода
fn prompt_retry() {
    print!("Нажмите Enter для повторной попытки или ESC для выхода...");
    io::stdout().flush().unwrap();
    
    // Читаем клавишу в raw mode
    loop {
        if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
            if let Ok(Event::Key(key_event)) = event::read() {
                match key_event.code {
                    KeyCode::Enter => {
                        println!(); // Новая строка после ввода
                        return;
                    }
                    KeyCode::Esc => {
                        // Выход из программы при нажатии ESC
                        let _ = terminal::disable_raw_mode();
                        println!("\n\nДо свидания!");
                        std::process::exit(0);
                    }
                    _ => {} // Игнорируем другие клавиши
                }
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ticket_is_within_valid_range() {
        for _ in 0..1_000 {
            let ticket = generate_ticket_number();
            assert!((1..=20).contains(&ticket));
        }
    }


    #[test]
    fn generated_ticket_is_always_between_one_and_twenty() {
        for _ in 0..10_000 {
            assert!((1..=20).contains(&generate_ticket_number()));
        }
    }

    #[test]
    fn next_empty_row_skips_contiguous_records() {
        let mut workbook = umya_spreadsheet::new_file();
        let sheet = workbook.get_sheet_mut(0);

        for row in 2..=5 {
            sheet
                .get_cell_mut(&format!("A{row}"))
                .set_value(format!("Student {row}"));
        }

        assert_eq!(find_next_empty_row(&workbook), 6);
    }

    #[test]
    fn next_empty_row_handles_unicode_values() {
        let mut workbook = umya_spreadsheet::new_file();
        let sheet = workbook.get_sheet_mut(0);

        sheet.get_cell_mut("A2").set_value("Иванов");
        sheet.get_cell_mut("A3").set_value("学生");

        assert_eq!(find_next_empty_row(&workbook), 4);
    }


    
        #[test]
        fn ticket_number_never_exceeds_twenty() {
            for _ in 0..10_000 {
                assert!(generate_ticket_number() <= 20);
            }
        }
    
        #[test]
        fn ticket_number_is_never_zero() {
            for _ in 0..10_000 {
                assert!(generate_ticket_number() >= 1);
            }
        }
    
        #[test]
        fn next_row_is_after_one_record() {
            let mut workbook = umya_spreadsheet::new_file();
            workbook
                .get_sheet_mut(0)
                .get_cell_mut("A2")
                .set_value("Student");
    
            assert_eq!(find_next_empty_row(&workbook), 3);
        }

   #[test]
    fn data_in_other_columns_does_not_affect_result() {
        let mut workbook = umya_spreadsheet::new_file();
        let sheet = workbook.get_sheet_mut(0);

        sheet.get_cell_mut("B2").set_value("Ivan");
        sheet.get_cell_mut("C2").set_value("15");
        sheet.get_cell_mut("D2").set_value("2025-01-01");

        assert_eq!(find_next_empty_row(&workbook), 2);
    }

}