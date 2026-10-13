mod app;
mod error;
mod models;
mod results;
mod students;
mod tickets;
mod web;

use app::TicketGeneratorApp;
use log::error;
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::init();
    if std::env::args().any(|argument| argument == "--web") {
        return web::run().map_err(|error| eframe::Error::AppCreation(Box::new(error)));
    }
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Exam Ticket Generator",
        options,
        Box::new(|_creation_context| {
            Ok(Box::new(TicketGeneratorApp::new(
                PathBuf::from("students.xlsx"),
                PathBuf::from("tickets.docx"),
                PathBuf::from("results.xlsx"),
            )))
        }),
    )
    .map_err(|error| {
        error!("GUI failed: {error}");
        error
    })
}
