use crate::{error::AppError, models::Ticket};
use docx_rs::{read_docx, DocumentChild, ParagraphChild, RunChild};
use log::warn;
use std::path::Path;

pub fn load_tickets(path: &Path) -> Result<Vec<Ticket>, AppError> {
    if !path.exists() {
        return Err(AppError::MissingFile(path.display().to_string()));
    }
    let bytes = std::fs::read(path)?;
    let document =
        read_docx(&bytes).map_err(|error| AppError::InvalidDocument(error.to_string()))?;
    let paragraphs = document
        .document
        .children
        .iter()
        .filter_map(|child| match child {
            DocumentChild::Paragraph(paragraph) => Some(paragraph_text(paragraph)),
            _ => None,
        });
    parse_ticket_paragraphs(paragraphs)
}

fn paragraph_text(paragraph: &docx_rs::Paragraph) -> String {
    paragraph
        .children
        .iter()
        .filter_map(|child| match child {
            ParagraphChild::Run(run) => Some(
                run.children
                    .iter()
                    .filter_map(|child| match child {
                        RunChild::Text(text) => Some(text.text.clone()),
                        _ => None,
                    })
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn parse_ticket_paragraphs<I>(paragraphs: I) -> Result<Vec<Ticket>, AppError>
where
    I: IntoIterator<Item = String>,
{
    let mut tickets = Vec::new();
    let mut current_number: Option<u32> = None;
    let mut questions = Vec::new();

    let finish = |number: Option<u32>, questions: &mut Vec<String>, tickets: &mut Vec<Ticket>| {
        if let Some(number) = number {
            if questions.len() >= 3 {
                tickets.push(Ticket {
                    number,
                    questions: std::mem::take(questions),
                });
            } else {
                warn!("Skipping ticket {number}: fewer than three questions");
                questions.clear();
            }
        }
    };

    for paragraph in paragraphs {
        let text = paragraph.trim();
        if let Some(number_text) = text.strip_prefix("Билет ") {
            finish(current_number, &mut questions, &mut tickets);
            match number_text.trim().parse::<u32>() {
                Ok(number) if number > 0 => current_number = Some(number),
                _ => {
                    warn!("Skipping malformed ticket header: {text}");
                    current_number = None;
                }
            }
            continue;
        }
        if let Some((prefix, question)) = text.split_once('.') {
            if prefix.trim().parse::<u32>().is_ok()
                && !question.trim().is_empty()
                && current_number.is_some()
            {
                questions.push(question.trim().to_string());
            }
        }
    }
    finish(current_number, &mut questions, &mut tickets);
    Ok(tickets)
}

#[cfg(test)]
mod tests {
    use super::parse_ticket_paragraphs;

    fn parse(text: &str) -> Vec<crate::models::Ticket> {
        parse_ticket_paragraphs(text.lines().map(str::to_string)).unwrap_or_default()
    }

    #[test]
    fn parses_valid_ticket() {
        assert_eq!(parse("Билет 1\n1. One\n2. Two\n3. Three")[0].number, 1);
    }

    #[test]
    fn skips_short_ticket() {
        assert!(parse("Билет 1\n1. One\n2. Two").is_empty());
    }

    #[test]
    fn skips_malformed_header_and_continues() {
        let tickets = parse("Билет nope\n1. Bad\nБилет 2\n1. One\n2. Two\n3. Three");
        assert_eq!(tickets.len(), 1);
        assert_eq!(tickets[0].number, 2);
    }

    #[test]
    fn parses_multiple_tickets() {
        let tickets = parse("Билет 1\n1. A\n2. B\n3. C\nБилет 2\n1. D\n2. E\n3. F");
        assert_eq!(
            tickets
                .iter()
                .map(|ticket| ticket.number)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}
