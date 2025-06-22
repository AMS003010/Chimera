use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use colored::*;
use rayon::prelude::*;
use serde_json::Value;

pub async fn shutdown_signal() {
    // Create a future that resolves when Ctrl+C is pressed
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    // On Windows, we need to handle ctrl_break differently
    #[cfg(windows)]
    let terminate = async {
        let mut ctrl_break =
            tokio::signal::windows::ctrl_break().expect("Failed to install Ctrl+Break handler");
        ctrl_break.recv().await;
        println!("\nReceived Ctrl+Break signal, starting graceful shutdown...");
    };

    // On Unix, handle SIGTERM
    #[cfg(unix)]
    let terminate = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        sigterm.recv().await;
        println!("\nReceived SIGTERM signal, starting graceful shutdown...");
    };

    // Wait for either signal
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// Helper function for value comparison
pub fn compare_values(a: &Value, b: &Value, key: &str, order: &str) -> std::cmp::Ordering {
    let a_val = a.get(key).and_then(Value::as_i64).unwrap_or(0);
    let b_val = b.get(key).and_then(Value::as_i64).unwrap_or(0);
    if order == "asc" {
        a_val.cmp(&b_val)
    } else {
        b_val.cmp(&a_val)
    }
}

// Helper function for logging
pub fn log_request(
    date_time: &str,
    status: &str,
    method: &str,
    path: &str,
    _is_error: bool,
    elapsed: u128,
    key_len: usize,
    id_len: usize,
    obj: usize,
) {
    let status_display = match status {
        "200" => " 200 ".bold().white().on_blue(),
        "404" => " 404 ".bold().white().on_red(),
        "500" => " 500 ".bold().white().on_green(),
        _ => " ??? ".bold().white().on_yellow(),
    };

    let method_display = match method {
        "GET" => " GET    ".bright_white().on_green(),
        "DELETE" => " DELETE ".bright_white().on_red(),
        _ => method.to_string().bright_white().on_green(),
    };
    let space_padding = key_len + id_len + 2 - path.len();
    let spaces = " ".repeat(space_padding);

    println!(
        "|{}| {} |{}| {}{}  | {}{}, {} {}",
        status_display,
        date_time.italic().dimmed(),
        method_display,
        path.italic(),
        spaces,
        elapsed.to_string().italic().dimmed(),
        "ms".italic().dimmed(),
        obj.to_string().italic().dimmed(),
        "entries affected".italic().dimmed(),
    );
}

// Helper function for busy response
pub fn server_busy_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Server is busy, try again later.",
    )
        .into_response()
}

pub fn find_key_and_id_lengths(parsed_content: &Value) -> Option<(usize, usize)> {
    let object = parsed_content.as_object()?;

    object
        .iter()
        .collect::<Vec<_>>() // Required for Rayon parallelism
        .par_iter()
        .filter_map(|(key, value)| {
            let arr = value.as_array()?;

            // All items must be objects with numeric "id"
            if arr
                .iter()
                .all(|v| v.get("id").and_then(Value::as_i64).is_some())
            {
                let max_id_len = arr
                    .par_iter()
                    .filter_map(|v| v.get("id")?.as_i64())
                    .map(|id| id.abs().to_string().len())
                    .max()?;

                let key_len = key.len();
                Some((key_len, max_id_len))
            } else {
                None
            }
        })
        .max_by_key(|(key_len, id_len)| key_len + id_len)
}
