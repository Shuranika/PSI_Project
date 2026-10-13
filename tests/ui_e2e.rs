use std::{error::Error, time::Duration};
use thirtyfour::components::SelectElement;
use thirtyfour::prelude::*;
use tokio::time::sleep;

const APP_URL: &str = "http://127.0.0.1:8080";
const WEBDRIVER_URL: &str = "http://localhost:9515";

#[tokio::test]
#[ignore = "Requires ChromeDriver on port 9515 and the web app on port 8080"]
async fn test_ui_e2e() -> Result<(), Box<dyn Error>> {
    let capabilities = DesiredCapabilities::chrome();
    let driver = WebDriver::new(WEBDRIVER_URL, capabilities).await?;

    let test_result: Result<(), Box<dyn Error>> = async {
        driver.goto(APP_URL).await?;

        let group_select = driver.find(By::Id("group")).await?;
        wait_for_options(&group_select, 2).await?;
        let groups = group_select.find_all(By::Css("option")).await?;
        let group_value = groups
            .get(1)
            .ok_or("No student groups are available in the running application")?
            .attr("value")
            .await?
            .ok_or("The first group option has no value")?;
        SelectElement::new(&group_select)
            .await?
            .select_by_value(&group_value)
            .await?;

        let student_select = driver.find(By::Id("student")).await?;
        wait_for_options(&student_select, 2).await?;
        let students = student_select.find_all(By::Css("option")).await?;
        let student_value = students
            .get(1)
            .ok_or("The selected group has no students")?
            .attr("value")
            .await?
            .ok_or("The first student option has no value")?;
        SelectElement::new(&student_select)
            .await?
            .select_by_value(&student_value)
            .await?;

        let generate_button = driver.find(By::Id("generate")).await?;
        wait_for_enabled(&generate_button).await?;
        assert!(generate_button.is_displayed().await?);
        generate_button.click().await?;

        let result = driver.find(By::Id("result")).await?;
        wait_for_displayed(&result).await?;
        let result_text = result.text().await?;
        assert!(result_text.contains("Ваш билет №"));

        let ticket_number = driver.find(By::Id("ticket-number")).await?.text().await?;
        assert!(
            ticket_number.parse::<u32>().is_ok(),
            "Expected a numeric ticket number, got {ticket_number:?}"
        );

        let questions = driver.find_all(By::Css("#questions .question")).await?;
        assert_eq!(
            questions.len(),
            3,
            "Expected exactly three ticket questions"
        );

        driver
            .find(By::Css("body"))
            .await?
            .send_keys(Key::Escape)
            .await?;
        wait_for_hidden(&result).await?;
        assert_eq!(
            driver.current_url().await?.as_str().trim_end_matches('/'),
            APP_URL
        );

        Ok(())
    }
    .await;

    let quit_result = driver.quit().await;
    test_result?;
    quit_result?;
    Ok(())
}

async fn wait_for_options(select: &WebElement, minimum: usize) -> Result<(), Box<dyn Error>> {
    for _ in 0..50 {
        if select.find_all(By::Css("option")).await?.len() >= minimum {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err(format!("Select did not receive {minimum} options in time").into())
}

async fn wait_for_enabled(element: &WebElement) -> Result<(), Box<dyn Error>> {
    for _ in 0..50 {
        if element.is_enabled().await? {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err("Generate button did not become enabled".into())
}

async fn wait_for_displayed(element: &WebElement) -> Result<(), Box<dyn Error>> {
    for _ in 0..50 {
        if element.is_displayed().await? {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err("Ticket result did not become visible".into())
}

async fn wait_for_hidden(element: &WebElement) -> Result<(), Box<dyn Error>> {
    for _ in 0..50 {
        if !element.is_displayed().await? {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }
    Err("Ticket result did not close after Escape".into())
}
