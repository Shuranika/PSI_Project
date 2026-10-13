use crate::{
    models::{ResultRecord, Student, Ticket},
    results, students, tickets,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf, sync::Arc};

const STUDENTS_PATH: &str = "students.xlsx";
const TICKETS_PATH: &str = "tickets.docx";
const RESULTS_PATH: &str = "results.xlsx";

#[derive(Clone)]
struct AppState {
    students_path: PathBuf,
    tickets_path: PathBuf,
    results_path: PathBuf,
}

#[derive(Serialize)]
struct Bootstrap {
    groups: Vec<String>,
    ticket_count: usize,
}

#[derive(Deserialize)]
struct GenerateRequest {
    group: String,
    last_name: String,
    first_name: String,
}

#[derive(Serialize)]
struct GenerateResponse {
    result: ResultRecord,
    ticket: Ticket,
}

pub fn run() -> Result<(), io::Error> {
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async_run())
}

async fn async_run() -> Result<(), io::Error> {
    let state = AppState {
        students_path: PathBuf::from(STUDENTS_PATH),
        tickets_path: PathBuf::from(TICKETS_PATH),
        results_path: PathBuf::from(RESULTS_PATH),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    println!("Ticket Generator web UI: http://127.0.0.1:8080");
    axum::serve(listener, router(state)).await
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/groups/:group/students", get(group_students))
        .route("/api/generate", post(generate))
        .with_state(Arc::new(state))
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

async fn bootstrap(State(state): State<Arc<AppState>>) -> Result<Json<Bootstrap>, ApiError> {
    results::ensure_results_file(&state.results_path).map_err(ApiError::from)?;
    let groups = students::load_groups(&state.students_path).map_err(ApiError::from)?;
    let ticket_count = tickets::load_tickets(&state.tickets_path)
        .map_err(ApiError::from)?
        .len();
    Ok(Json(Bootstrap {
        groups,
        ticket_count,
    }))
}

async fn group_students(
    State(state): State<Arc<AppState>>,
    Path(group): Path<String>,
) -> Result<Json<Vec<Student>>, ApiError> {
    students::load_students(&state.students_path, &group)
        .map(Json)
        .map_err(ApiError::from)
}

async fn generate(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateRequest>,
) -> Result<Json<GenerateResponse>, ApiError> {
    let group = request.group.trim();
    let last_name = request.last_name.trim();
    let first_name = request.first_name.trim();
    if group.is_empty() || last_name.is_empty() || first_name.is_empty() {
        return Err(ApiError::bad_request(
            "Group, last name, and first name are required.",
        ));
    }
    let available_students =
        students::load_students(&state.students_path, group).map_err(ApiError::from)?;
    if !available_students
        .iter()
        .any(|student| student.last_name == last_name && student.first_name == first_name)
    {
        return Err(ApiError::bad_request(
            "The selected student does not belong to this group.",
        ));
    }
    let ticket_list = tickets::load_tickets(&state.tickets_path).map_err(ApiError::from)?;
    let historical =
        results::find_first_ticket_for_student(&state.results_path, group, last_name, first_name)
            .map_err(ApiError::from)?;
    let (ticket_number, is_repeat) = if let Some(number) = historical {
        (number, true)
    } else {
        ticket_list
            .choose(&mut rand::thread_rng())
            .map(|ticket| (ticket.number, false))
            .ok_or_else(|| ApiError::bad_request("No valid tickets are available."))?
    };
    let student = Student {
        last_name: last_name.to_string(),
        first_name: first_name.to_string(),
    };
    let pending = crate::models::PendingResult {
        group: group.to_string(),
        student,
        ticket_number,
        is_repeat,
    };
    let result = results::save_result(&state.results_path, &pending).map_err(ApiError::from)?;
    let ticket = ticket_list
        .into_iter()
        .find(|ticket| ticket.number == ticket_number)
        .ok_or_else(|| ApiError::bad_request("The assigned ticket could not be loaded."))?;
    Ok(Json(GenerateResponse { result, ticket }))
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl From<crate::error::AppError> for ApiError {
    fn from(error: crate::error::AppError) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use docx_rs::{Docx, Paragraph, Run};
    use tempfile::tempdir;
    use tower::ServiceExt;
    use umya_spreadsheet::{reader::xlsx::read, writer::xlsx::write};

    fn fixture() -> (tempfile::TempDir, AppState) {
        let directory = tempdir().expect("temporary directory");
        let students_path = directory.path().join("students.xlsx");
        let tickets_path = directory.path().join("tickets.docx");
        let results_path = directory.path().join("results.xlsx");

        let mut workbook = umya_spreadsheet::new_file();
        workbook.get_sheet_mut(&0).unwrap().set_name("Group A");
        let sheet = workbook.get_sheet_mut(&0).unwrap();
        sheet.get_cell_mut("A1").set_value("Фамилия");
        sheet.get_cell_mut("B1").set_value("Имя");
        sheet.get_cell_mut("A2").set_value("Иванов");
        sheet.get_cell_mut("B2").set_value("Иван");
        sheet.get_cell_mut("A3").set_value("Петрова");
        sheet.get_cell_mut("B3").set_value("Анна");
        write(&workbook, &students_path).expect("write students workbook");

        let mut document = Docx::new();
        for (number, questions) in [
            (1, ["Вопрос 1A", "Вопрос 1B", "Вопрос 1C"]),
            (2, ["Вопрос 2A", "Вопрос 2B", "Вопрос 2C"]),
        ] {
            document = document.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(format!("Билет {number}"))),
            );
            for (index, question) in questions.iter().enumerate() {
                document = document.add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text(format!("{}. {question}", index + 1))),
                );
            }
        }
        let mut file = std::fs::File::create(&tickets_path).expect("create tickets document");
        document
            .build()
            .pack(&mut file)
            .expect("write tickets document");

        (
            directory,
            AppState {
                students_path,
                tickets_path,
                results_path,
            },
        )
    }

    async fn response_body(response: axum::response::Response) -> String {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        String::from_utf8(bytes.to_vec()).expect("response is UTF-8")
    }

    #[tokio::test]
    async fn web_ui_serves_selection_controls_and_escape_return() {
        let (_directory, state) = fixture();
        let response = router(state)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let html = response_body(response).await;

        assert!(html.contains("<select id=\"group\">"));
        assert!(html.contains("<select id=\"student\" disabled>"));
        assert!(html.contains("<button id=\"generate\" disabled>"));
        assert!(html.contains("Ваш билет №"));
        assert!(html.contains("event.key === 'Escape'"));
        assert!(html.contains("closeResult()"));
    }

    #[tokio::test]
    async fn group_selection_returns_students_from_the_selected_worksheet() {
        let (_directory, state) = fixture();
        let app = router(state);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/bootstrap")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response_body(response).await).unwrap()
                ["groups"],
            serde_json::json!(["Group A"])
        );

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/groups/Group%20A/students")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&response_body(response).await).unwrap(),
            serde_json::json!([
                {"last_name": "Иванов", "first_name": "Иван"},
                {"last_name": "Петрова", "first_name": "Анна"}
            ])
        );
    }

    #[tokio::test]
    async fn repeated_generation_appends_the_original_ticket_and_marks_repeat() {
        let (_directory, state) = fixture();
        let results_path = state.results_path.clone();
        let app = router(state);
        let request_body = serde_json::json!({
            "group": "Group A",
            "last_name": "Иванов",
            "first_name": "Иван"
        });

        let first = generate_for(&app, request_body.clone()).await;
        assert!((1..=2).contains(&first["ticket"]["number"].as_u64().unwrap()));
        assert_eq!(first["result"]["is_repeat"], false);
        assert_eq!(first["ticket"]["questions"].as_array().unwrap().len(), 3);

        let second = generate_for(&app, request_body).await;
        assert_eq!(second["ticket"]["number"], first["ticket"]["number"]);
        assert_eq!(second["result"]["is_repeat"], true);

        let workbook = read(&results_path).expect("read saved results");
        let sheet = workbook.get_sheet(&0).unwrap();
        assert_eq!(sheet.get_highest_row(), 3);
        assert_eq!(
            sheet.get_cell("D2").unwrap().get_value(),
            first["ticket"]["number"].as_u64().unwrap().to_string()
        );
        assert_eq!(
            sheet.get_cell("D3").unwrap().get_value(),
            first["ticket"]["number"].as_u64().unwrap().to_string()
        );
        assert_eq!(sheet.get_cell("F2").unwrap().get_value(), "нет");
        assert_eq!(sheet.get_cell("F3").unwrap().get_value(), "да");
    }

    async fn generate_for(app: &Router, payload: serde_json::Value) -> serde_json::Value {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        serde_json::from_str(&response_body(response).await).unwrap()
    }

    #[tokio::test]
    async fn missing_source_file_returns_a_readable_error_instead_of_panicking() {
        let directory = tempdir().unwrap();
        let response = router(AppState {
            students_path: directory.path().join("missing-students.xlsx"),
            tickets_path: directory.path().join("missing-tickets.docx"),
            results_path: directory.path().join("results.xlsx"),
        })
        .oneshot(
            Request::builder()
                .uri("/api/bootstrap")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(response_body(response)
            .await
            .contains("missing-students.xlsx"));
    }

    #[tokio::test]
    async fn generation_rejects_students_not_in_the_selected_group() {
        let (_directory, state) = fixture();
        let response = router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/generate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "group": "Group A",
                            "last_name": "Not listed",
                            "first_name": "Student"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response_body(response)
            .await
            .contains("does not belong to this group"));
    }
}
