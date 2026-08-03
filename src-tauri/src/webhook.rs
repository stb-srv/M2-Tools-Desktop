use serde_json::json;

/// Sends a plain-text message to a webhook URL using the `{"content": "..."}`
/// JSON shape Discord webhooks expect (other services compatible with that
/// shape work too). Kept as a single generic command rather than wiring
/// webhook calls into every individual backend flow that might want to
/// notify - callers (Server Control failures, DB backup failures, the
/// frontend crash-watch) all just call this with their own message.
pub async fn send_webhook_message(url: &str, content: &str) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .json(&json!({ "content": content }))
        .send()
        .await
        .map_err(|e| format!("Webhook-Anfrage fehlgeschlagen: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("Webhook antwortete mit Status {}", response.status()));
    }
    Ok(())
}
