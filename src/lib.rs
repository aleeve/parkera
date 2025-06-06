use std::collections::HashSet;

use select::{document::Document, predicate::Attr};
use serde_json::{json, Value};
use worker::*;

#[event(scheduled)]
async fn cron(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    console_error_panic_hook::set_once();
    match check(env).await {
        Ok(_) => console_log!("Success!"),
        Err(e) => console_log!("Error: {}", e),
    }
}

async fn check(env: Env) -> Result<()> {
    let kv = env.kv("parkera_kv")?;

    let url = "https://www.wallenstam.se/sv/bostader/parkering/?Region=G%C3%B6teborg&Sort=1";

    let mut response = Fetch::Url(url.parse()?).send().await?;
    let html = response.text().await?;
    let document = Document::from(html.as_str());

    let links: HashSet<String> = document
        .find(Attr("href", ()))
        .filter_map(|node| {
            let href = node.attr("href")?;
            if href.starts_with("/sv/bostader/parkering/goteborg/") {
                Some(href.to_string())
            } else {
                None
            }
        })
        .collect();

    match kv.get("links").text().await? {
        Some(value) => {
            // We need to check if there are new links
            let prev_links: HashSet<_> = serde_json::from_str::<HashSet<String>>(value.as_str())?;
            let added_links: Vec<String> = links.difference(&prev_links).cloned().collect();

            if !added_links.is_empty() {
                let content = format!("Nya parkeringar:\n\n {}", added_links.join("\n"));
                send_email(&env, &content).await?;
            }
        }
        None => {
            console_log!("No previous links");
            let content = format!(
                "Nya parkeringar:\n\n {}",
                links.iter().cloned().collect::<Vec<_>>().join("\n")
            );
            send_email(&env, &content).await?;
        }
    }
    kv.put("links", serde_json::to_string(&links)?)?
        .execute()
        .await?;

    Ok(())
}

async fn send_email(env: &Env, content: &str) -> Result<()> {
    // Get SendGrid API key from Cloudflare secret
    let api_key = env.secret("sendgrid")?.to_string();
    let emails: Vec<Value> =
        serde_json::from_str::<Vec<String>>(&env.secret("emails")?.to_string())?
            .into_iter()
            .map(|email| json!({"email": email}))
            .collect();

    // SendGrid API payload
    let payload = json!({
        "personalizations": [{
            "to": emails
        }],
        "from": {
            "email": "parkera@aleeve.dev",
        },
        "subject": "Nya parkeringar",
        "content": [{
            "type": "text/plain",
            "value": content
        }]
    });

    let mut headers = Headers::new();
    headers.set("Authorization", &format!("Bearer {}", api_key))?;
    headers.set("Content-Type", "application/json")?;

    let request = Request::new_with_init(
        "https://api.sendgrid.com/v3/mail/send",
        RequestInit::new()
            .with_method(Method::Post)
            .with_headers(headers)
            .with_body(Some(payload.to_string().into())),
    )?;

    let mut resp = Fetch::Request(request).send().await?;
    let status_code = resp.status_code();
    if !(200..300).contains(&status_code) {
        console_log!("{:?}", resp.text().await?);
        return Err(Error::from("Failed to send email: {status_code}"));
    };
    Ok(())
}
