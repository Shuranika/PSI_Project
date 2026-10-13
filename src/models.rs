use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Student {
    pub last_name: String,
    pub first_name: String,
}

impl Student {
    pub fn display_name(&self) -> String {
        format!("{} {}", self.last_name, self.first_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ticket {
    pub number: u32,
    pub questions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResultRecord {
    pub group: String,
    pub student: Student,
    pub ticket_number: u32,
    pub timestamp: String,
    pub is_repeat: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingResult {
    pub group: String,
    pub student: Student,
    pub ticket_number: u32,
    pub is_repeat: bool,
}
