use crate::{
    models::{PendingResult, ResultRecord, Student, Ticket},
    results, students, tickets,
};
use eframe::egui::{self, Color32, RichText};
use log::error;
use rand::seq::SliceRandom;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewState {
    Main,
    Result,
}

pub struct TicketGeneratorApp {
    pub groups: Vec<String>,
    pub students: Vec<Student>,
    pub tickets: Vec<Ticket>,
    pub selected_group: Option<String>,
    pub selected_student: Option<Student>,
    pub current_result: Option<ResultRecord>,
    pub view: ViewState,
    pub status_message: String,
    pub error_message: Option<String>,
    pub retry_save: Option<PendingResult>,
    students_path: PathBuf,
    tickets_path: PathBuf,
    results_path: PathBuf,
    saving: bool,
}

impl TicketGeneratorApp {
    pub fn new(students_path: PathBuf, tickets_path: PathBuf, results_path: PathBuf) -> Self {
        let mut app = Self {
            groups: Vec::new(),
            students: Vec::new(),
            tickets: Vec::new(),
            selected_group: None,
            selected_student: None,
            current_result: None,
            view: ViewState::Main,
            status_message: String::new(),
            error_message: None,
            retry_save: None,
            students_path,
            tickets_path,
            results_path,
            saving: false,
        };
        if let Err(error) = results::ensure_results_file(&app.results_path) {
            app.set_error(error);
        }
        match students::load_groups(&app.students_path) {
            Ok(groups) => app.groups = groups,
            Err(error) => app.set_error(error),
        }
        match tickets::load_tickets(&app.tickets_path) {
            Ok(tickets) => app.tickets = tickets,
            Err(error) => app.set_error(error),
        }
        if app.error_message.is_none() {
            app.status_message = "Select a group and student to assign a ticket.".to_string();
        }
        app
    }

    fn set_error(&mut self, error: impl std::fmt::Display) {
        error!("{error}");
        self.error_message = Some(error.to_string());
    }

    fn can_generate(&self) -> bool {
        self.selected_group.is_some()
            && self.selected_student.is_some()
            && !self.tickets.is_empty()
            && !self.saving
            && self.retry_save.is_none()
    }

    fn select_group(&mut self, group: String) {
        self.selected_group = Some(group.clone());
        self.selected_student = None;
        self.students.clear();
        match students::load_students(&self.students_path, &group) {
            Ok(students) => {
                self.students = students;
                self.error_message = None;
                self.status_message = "Choose a student.".to_string();
            }
            Err(error) => self.set_error(error),
        }
    }

    fn save_pending(&mut self, pending: PendingResult) {
        self.saving = true;
        match results::save_result(&self.results_path, &pending) {
            Ok(result) => {
                self.current_result = Some(result);
                self.retry_save = None;
                self.error_message = None;
                self.view = ViewState::Result;
            }
            Err(error) => {
                self.retry_save = Some(pending);
                self.set_error(error);
            }
        }
        self.saving = false;
    }

    fn generate(&mut self) {
        let (Some(group), Some(student)) =
            (self.selected_group.clone(), self.selected_student.clone())
        else {
            return;
        };
        let historical = match results::find_first_ticket_for_student(
            &self.results_path,
            &group,
            &student.last_name,
            &student.first_name,
        ) {
            Ok(ticket) => ticket,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        let (ticket_number, is_repeat) = if let Some(number) = historical {
            (number, true)
        } else if let Some(ticket) = self.tickets.choose(&mut rand::thread_rng()) {
            (ticket.number, false)
        } else {
            self.set_error("No valid tickets are available.");
            return;
        };
        self.save_pending(PendingResult {
            group,
            student,
            ticket_number,
            is_repeat,
        });
    }

    fn setup_visuals(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(Color32::from_rgb(231, 238, 242));
        visuals.panel_fill = Color32::from_rgb(13, 24, 35);
        visuals.window_fill = Color32::from_rgb(18, 32, 45);
        visuals.extreme_bg_color = Color32::from_rgb(8, 17, 27);
        visuals.faint_bg_color = Color32::from_rgb(24, 43, 55);
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(18, 32, 45);
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(24, 43, 55);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(35, 91, 93);
        visuals.widgets.active.bg_fill = Color32::from_rgb(43, 125, 122);
        visuals.selection.bg_fill = Color32::from_rgb(43, 125, 122);
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(16.0, 10.0);
        style.visuals.window_rounding = egui::Rounding::same(18.0);
        ctx.set_style(style);
    }

    fn section_title(ui: &mut egui::Ui, eyebrow: &str, title: &str) {
        ui.label(
            RichText::new(eyebrow.to_uppercase())
                .size(11.0)
                .strong()
                .color(Color32::from_rgb(95, 202, 188)),
        );
        ui.label(RichText::new(title).size(25.0).strong());
    }

    fn stat_card(ui: &mut egui::Ui, value: impl Into<String>, label: &str) {
        egui::Frame::none()
            .fill(Color32::from_rgb(18, 34, 47))
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(37, 65, 76)))
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::same(14.0))
            .show(ui, |ui| {
                ui.label(RichText::new(value.into()).size(22.0).strong());
                ui.label(RichText::new(label).size(11.0).color(Color32::from_rgb(145, 166, 173)));
            });
    }

    fn top_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TG").size(18.0).strong().color(Color32::from_rgb(8, 29, 34)));
            ui.add_space(4.0);
            ui.label(RichText::new("TICKET GENERATOR").size(15.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("LOCAL EXAM DESK").size(10.0).color(Color32::from_rgb(145, 166, 173)));
            });
        });
        ui.separator();
    }

    fn main_view(&mut self, ui: &mut egui::Ui) {
        Self::section_title(ui, "Exam session", "Assign an exam ticket");
        ui.label("Choose a group and student. The ticket is saved to the results workbook instantly.");
        ui.add_space(18.0);

        ui.horizontal(|ui| {
            Self::stat_card(ui, self.groups.len().to_string(), "GROUPS LOADED");
            Self::stat_card(ui, self.tickets.len().to_string(), "TICKETS READY");
            Self::stat_card(ui, self.students.len().to_string(), "STUDENTS IN GROUP");
        });
        ui.add_space(18.0);

        egui::Frame::none()
            .fill(Color32::from_rgb(18, 34, 47))
            .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(37, 65, 76)))
            .rounding(egui::Rounding::same(16.0))
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.label(RichText::new("1  SELECT GROUP").size(11.0).strong().color(Color32::from_rgb(95, 202, 188)));
                egui::ComboBox::from_id_salt("group_selector")
                    .width(ui.available_width())
                    .selected_text(self.selected_group.as_deref().unwrap_or("Choose a group"))
                    .show_ui(ui, |ui| {
                        for group in self.groups.clone() {
                            if ui.selectable_label(self.selected_group.as_ref() == Some(&group), &group).clicked() {
                                self.select_group(group);
                                ui.close_menu();
                            }
                        }
                    });
                ui.add_space(12.0);
                ui.label(RichText::new("2  SELECT STUDENT").size(11.0).strong().color(Color32::from_rgb(95, 202, 188)));
                egui::ComboBox::from_id_salt("student_selector")
                    .width(ui.available_width())
                    .selected_text(self.selected_student.as_ref().map(Student::display_name).unwrap_or_else(|| "Choose a student".to_string()))
                    .show_ui(ui, |ui| {
                        for student in self.students.clone() {
                            if ui.selectable_label(self.selected_student.as_ref() == Some(&student), student.display_name()).clicked() {
                                self.selected_student = Some(student);
                                ui.close_menu();
                            }
                        }
                    });
                ui.add_space(18.0);
                let button = egui::Button::new(RichText::new("GENERATE TICKET  >").strong().size(14.0))
                    .min_size(egui::vec2(ui.available_width(), 48.0));
                if ui.add_enabled(self.can_generate(), button).clicked() {
                    self.generate();
                }
            });

        if let Some(error) = &self.error_message {
            ui.add_space(12.0);
            ui.colored_label(Color32::from_rgb(245, 126, 126), error);
        } else if !self.status_message.is_empty() {
            ui.add_space(10.0);
            ui.label(RichText::new(&self.status_message).color(Color32::from_rgb(145, 166, 173)));
        }
        if let Some(pending) = self.retry_save.clone() {
            ui.add_space(8.0);
            ui.label("The result was not saved. Close results.xlsx and retry.");
            if ui.button("Retry save").clicked() {
                self.save_pending(pending);
            }
        }
    }

    fn result_view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.view = ViewState::Main;
            return;
        }
        if let Some(result) = &self.current_result {
            Self::section_title(ui, "Assignment complete", "Your exam ticket");
            ui.label("The assignment has been recorded in results.xlsx.");
            ui.add_space(18.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(30, 91, 87))
                .rounding(egui::Rounding::same(16.0))
                .inner_margin(egui::Margin::same(22.0))
                .show(ui, |ui| {
                    ui.label(RichText::new("TICKET NUMBER").size(11.0).strong().color(Color32::from_rgb(174, 230, 216)));
                    ui.label(RichText::new(result.ticket_number.to_string()).size(58.0).strong().color(Color32::WHITE));
                    ui.label(format!("{}  /  {}", result.group, result.student.display_name()));
                });
            ui.add_space(14.0);
            ui.label(RichText::new(if result.is_repeat { "Previously assigned ticket" } else { "New random assignment" }).color(Color32::from_rgb(145, 166, 173)));
            if let Some(ticket) = self
                .tickets
                .iter()
                .find(|ticket| ticket.number == result.ticket_number)
            {
                ui.add_space(18.0);
                ui.label(RichText::new("QUESTIONS ON THIS TICKET").size(11.0).strong().color(Color32::from_rgb(95, 202, 188)));
                for (index, question) in ticket.questions.iter().take(3).enumerate() {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(18, 34, 47))
                        .rounding(egui::Rounding::same(9.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.label(format!("{:02}  {}", index + 1, question));
                        });
                }
            }
            ui.add_space(12.0);
            if ui.button("<  Assign another student").clicked() {
                self.view = ViewState::Main;
            }
        }
    }
}

impl eframe::App for TicketGeneratorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        Self::setup_visuals(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(10, 20, 30)).inner_margin(egui::Margin::same(28.0)))
            .show(ctx, |ui| {
                self.top_bar(ui);
                ui.add_space(24.0);
                match self.view {
                    ViewState::Main => self.main_view(ui),
                    ViewState::Result => self.result_view(ctx, ui),
                }
            });
    }
}
