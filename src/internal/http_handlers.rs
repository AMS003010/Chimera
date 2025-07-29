use crate::internal::chimera::{AppState, CHIMERA_LATEST_VERSION};
use crate::internal::helpers::{compare_values, server_busy_response};
use axum::{
    extract::{Path, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    Form, Json,
};
use chrono::Local;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep, timeout, Duration};
use tracing::{info, warn};

#[derive(Deserialize)]
pub struct FormData {
    #[serde(flatten)]
    fields: HashMap<String, String>,
}

pub async fn ping_pong() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain")],
        format!(
            "status: ONLINE\nversion: {}\n🐲 All systems fused and breathing fire.",
            CHIMERA_LATEST_VERSION
        )
        .to_string(),
    )
}

pub async fn get_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    // Clone only the needed data immediately after acquiring lock
    let route_data = {
        let json_data = match timeout(Duration::from_millis(100), state.json_value.read()).await {
            Ok(lock) => {
                if let Some(path_id) = route.split("/").last() {
                    if let Ok(_id) = path_id.parse::<usize>() {
                        let mut route_parts: Vec<&str> = route.split('/').collect();
                        route_parts.pop();
                        let base_path = route_parts.join("/");
                        lock.get(&base_path).cloned()
                    } else {
                        lock.get(&route).cloned()
                    }
                } else {
                    lock.get(&route).cloned()
                }
            }
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis();
                if !state.logs_disabled {
                    warn!(
                        date_time = date_time,
                        status = "500",
                        method = "GET",
                        path = requested_path,
                        error = "Server busy !!",
                        elapsed_ms = elapsed,
                        records = 0,
                        "HTTP request"
                    );
                }
                return server_busy_response();
            }
        };
        json_data
    };

    match route_data {
        Some(mut value) => {
            if let Some(path_id) = route.split("/").last() {
                if let Ok(_id) = path_id.parse::<usize>() {
                    if let Value::Array(ref mut arr) = value {
                        arr.retain(|obj| {
                            obj.get("id")
                                .and_then(|id| id.as_u64())
                                .map_or(false, |id_num| id_num == _id as u64)
                        });
                    }
                    let elapsed = start_time.elapsed().as_millis();
                    if !state.logs_disabled {
                        info!(
                            date_time = date_time,
                            status = "200",
                            method = "GET",
                            path = requested_path,
                            elapsed_ms = elapsed,
                            records = value.as_array().map_or(0, |arr| arr.len()),
                            "HTTP request"
                        );
                    }
                    return (StatusCode::OK, axum::Json(value)).into_response();
                }
            }

            // Rest of processing happens WITHOUT holding the lock
            if let Some((order, key)) = state.sort_rules.get(&route) {
                if let Value::Array(arr) = &mut value {
                    if arr.len() > 1 {
                        use rayon::slice::ParallelSliceMut;
                        arr.par_sort_by(|a, b| compare_values(a, b, key, order));
                    }
                }
            }

            if state.paginate > 0 {
                if let Value::Array(arr) = &value {
                    if arr.len() > state.paginate as usize {
                        value = Value::Array(arr[..state.paginate as usize].to_vec());
                    }
                }
            }

            let elapsed = start_time.elapsed().as_millis();
            if !state.logs_disabled {
                info!(
                    date_time = date_time,
                    status = "200",
                    method = "GET",
                    path = requested_path,
                    elapsed_ms = elapsed,
                    records = value.as_array().map_or(0, |arr| arr.len()),
                    "HTTP request"
                );
            }
            (StatusCode::OK, axum::Json(value)).into_response()
        }
        None => {
            let elapsed = start_time.elapsed().as_millis();
            if !state.logs_disabled {
                warn!(
                    date_time = date_time,
                    status = "404",
                    method = "GET",
                    path = requested_path,
                    error = "Route not registered !!",
                    elapsed_ms = elapsed,
                    records = 0,
                    "HTTP request"
                );
            }
            (StatusCode::NOT_FOUND, "Route not registered !!").into_response()
        }
    }
}

pub async fn delete_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    // Handle the DELETE operation
    let delete_result = {
        let mut json_data =
            match timeout(Duration::from_millis(100), state.json_value.write()).await {
                Ok(lock) => lock,
                Err(_) => {
                    let elapsed = start_time.elapsed().as_millis();
                    if !state.logs_disabled {
                        warn!(
                            date_time = date_time,
                            status = "500",
                            method = "GET",
                            path = requested_path,
                            error = "Server busy !!",
                            elapsed_ms = elapsed,
                            records = 0,
                            "HTTP request"
                        );
                    }
                    return server_busy_response();
                }
            };

        // Check if we're deleting a specific ID
        if let Some(path_id) = route.split("/").last() {
            if let Ok(id) = path_id.parse::<usize>() {
                // Extract base path (remove the ID part)
                let mut route_parts: Vec<&str> = route.split('/').collect();
                route_parts.pop();
                let base_path = route_parts.join("/");

                match json_data.get_mut(&base_path) {
                    Some(Value::Array(arr)) => {
                        let original_len = arr.len();
                        arr.retain(|obj| {
                            obj.get("id")
                                .and_then(|id_val| id_val.as_u64())
                                .map_or(true, |id_num| id_num != id as u64)
                        });
                        let deleted_count = original_len - arr.len();

                        if deleted_count > 0 {
                            (
                                "200",
                                format!("Deleted {} record(s) with id {}", deleted_count, id),
                                deleted_count,
                            )
                        } else {
                            ("404", format!("No record found with id {}", id), 0)
                        }
                    }
                    Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                    None => ("404", "Route not registered !!".to_string(), 0),
                }
            } else {
                // Delete entire collection
                match json_data.get_mut(&route) {
                    Some(Value::Array(arr)) => {
                        let deleted_count = arr.len();
                        arr.clear();
                        (
                            "200",
                            "All records deleted successfully".to_string(),
                            deleted_count,
                        )
                    }
                    Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                    None => ("404", "Route not registered !!".to_string(), 0),
                }
            }
        } else {
            // Delete entire collection
            match json_data.get_mut(&route) {
                Some(Value::Array(arr)) => {
                    let deleted_count = arr.len();
                    arr.clear();
                    (
                        "200",
                        "All records deleted successfully".to_string(),
                        deleted_count,
                    )
                }
                Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                None => ("404", "Route not registered !!".to_string(), 0),
            }
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = delete_result;

    if !state.logs_disabled {
        match status_code {
            "200" | "201" => info!(
                date_time = date_time,
                status = status_code,
                method = "DELETE",
                path = requested_path,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
            _ => warn!(
                date_time = date_time,
                status = status_code,
                method = "DELETE",
                path = requested_path,
                error = message,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
        }
    }

    // Return appropriate response
    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

pub async fn post_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    // Handle the POST operation
    let post_result = {
        let mut json_data =
            match timeout(Duration::from_millis(100), state.json_value.write()).await {
                Ok(lock) => lock,
                Err(_) => {
                    let elapsed = start_time.elapsed().as_millis();
                    if !state.logs_disabled {
                        warn!(
                            date_time = date_time,
                            status = "500",
                            method = "GET",
                            path = requested_path,
                            error = "Server busy !!",
                            elapsed_ms = elapsed,
                            records = 0,
                            "HTTP request"
                        );
                    }
                    return server_busy_response();
                }
            };

        if let Value::Object(ref mut obj) = *json_data {
            match obj.get_mut(&route) {
                Some(Value::Array(arr)) => match payload {
                    Value::Array(new_items) => {
                        let added_count = new_items.len();
                        arr.extend(new_items);
                        (
                            "201",
                            format!("Added {} record(s) successfully", added_count),
                            added_count,
                        )
                    }
                    single_item => {
                        arr.push(single_item);
                        ("201", "Record added successfully".to_string(), 1)
                    }
                },
                Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                None => {
                    // Create new route with the payload
                    match payload {
                        Value::Array(items) => {
                            let added_count = items.len();
                            obj.insert(route.clone(), Value::Array(items));
                            (
                                "201",
                                format!("Created route and added {} record(s)", added_count),
                                added_count,
                            )
                        }
                        single_item => {
                            obj.insert(route.clone(), Value::Array(vec![single_item]));
                            ("201", "Created route and added record".to_string(), 1)
                        }
                    }
                }
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = post_result;

    if !state.logs_disabled {
        match status_code {
            "201" => info!(
                date_time = date_time,
                status = status_code,
                method = "POST",
                path = requested_path,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
            _ => warn!(
                date_time = date_time,
                status = status_code,
                method = "POST",
                path = requested_path,
                error = message,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
        }
    }

    match status_code {
        "201" => (StatusCode::CREATED, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

pub async fn put_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    // Handle the PUT operation
    let put_result = {
        let mut json_data =
            match timeout(Duration::from_millis(100), state.json_value.write()).await {
                Ok(lock) => lock,
                Err(_) => {
                    let elapsed = start_time.elapsed().as_millis();
                    if !state.logs_disabled {
                        warn!(
                            date_time = date_time,
                            status = "500",
                            method = "GET",
                            path = requested_path,
                            error = "Server busy !!",
                            elapsed_ms = elapsed,
                            records = 0,
                            "HTTP request"
                        );
                    }
                    return server_busy_response();
                }
            };

        if let Value::Object(ref mut obj) = *json_data {
            // Check if we're updating a specific ID
            if let Some(path_id) = route.split("/").last() {
                if let Ok(id) = path_id.parse::<usize>() {
                    // Extract base path (remove the ID part)
                    let mut route_parts: Vec<&str> = route.split('/').collect();
                    route_parts.pop();
                    let base_path = route_parts.join("/");

                    match obj.get_mut(&base_path) {
                        Some(Value::Array(arr)) => {
                            let mut found = false;
                            for item in arr.iter_mut() {
                                if let Some(item_id) = item.get("id").and_then(|id| id.as_u64()) {
                                    if item_id == id as u64 {
                                        *item = payload.clone();
                                        // Ensure the ID is preserved
                                        if let Value::Object(ref mut item_obj) = item {
                                            item_obj.insert(
                                                "id".to_string(),
                                                Value::Number(serde_json::Number::from(id)),
                                            );
                                        }
                                        found = true;
                                        break;
                                    }
                                }
                            }

                            if found {
                                ("200", format!("Updated record with id {}", id), 1)
                            } else {
                                // Create new record with the specified ID
                                let mut new_item = payload.clone();
                                if let Value::Object(ref mut item_obj) = new_item {
                                    item_obj.insert(
                                        "id".to_string(),
                                        Value::Number(serde_json::Number::from(id)),
                                    );
                                }
                                arr.push(new_item);
                                ("201", format!("Created record with id {}", id), 1)
                            }
                        }
                        Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                        None => ("404", "Route not registered !!".to_string(), 0),
                    }
                } else {
                    // Replace entire collection
                    let record_count = match &payload {
                        Value::Array(arr) => arr.len(),
                        _ => 1,
                    };
                    let was_existing = obj.contains_key(&route);
                    obj.insert(route.clone(), payload);

                    if was_existing {
                        (
                            "200",
                            format!("Replaced entire collection with {} record(s)", record_count),
                            record_count,
                        )
                    } else {
                        (
                            "201",
                            format!("Created collection with {} record(s)", record_count),
                            record_count,
                        )
                    }
                }
            } else {
                // Replace entire collection
                let record_count = match &payload {
                    Value::Array(arr) => arr.len(),
                    _ => 1,
                };
                let was_existing = obj.contains_key(&route);
                obj.insert(route.clone(), payload);

                if was_existing {
                    (
                        "200",
                        format!("Replaced entire collection with {} record(s)", record_count),
                        record_count,
                    )
                } else {
                    (
                        "201",
                        format!("Created collection with {} record(s)", record_count),
                        record_count,
                    )
                }
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = put_result;

    if !state.logs_disabled {
        match status_code {
            "200" | "201" => info!(
                date_time = date_time,
                status = status_code,
                method = "PUT",
                path = requested_path,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
            _ => warn!(
                date_time = date_time,
                status = status_code,
                method = "PUT",
                path = requested_path,
                error = message,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
        }
    }

    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "201" => (StatusCode::CREATED, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

pub async fn patch_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    // Handle the PATCH operation
    let patch_result = {
        let mut json_data =
            match timeout(Duration::from_millis(100), state.json_value.write()).await {
                Ok(lock) => lock,
                Err(_) => {
                    let elapsed = start_time.elapsed().as_millis();
                    if !state.logs_disabled {
                        warn!(
                            date_time = date_time,
                            status = "500",
                            method = "GET",
                            path = requested_path,
                            error = "Server busy !!",
                            elapsed_ms = elapsed,
                            records = 0,
                            "HTTP request"
                        );
                    }
                    return server_busy_response();
                }
            };

        if let Value::Object(ref mut obj) = *json_data {
            // Check if we're updating a specific ID
            if let Some(path_id) = route.split("/").last() {
                if let Ok(id) = path_id.parse::<usize>() {
                    // Extract base path (remove the ID part)
                    let mut route_parts: Vec<&str> = route.split('/').collect();
                    route_parts.pop();
                    let base_path = route_parts.join("/");

                    match obj.get_mut(&base_path) {
                        Some(Value::Array(arr)) => {
                            let mut found = false;
                            for item in arr.iter_mut() {
                                if let Some(item_id) = item.get("id").and_then(|id| id.as_u64()) {
                                    if item_id == id as u64 {
                                        // Merge the payload into the existing item
                                        if let (Value::Object(existing), Value::Object(updates)) =
                                            (item, &payload)
                                        {
                                            for (key, value) in updates {
                                                existing.insert(key.clone(), value.clone());
                                            }
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }

                            if found {
                                ("200", format!("Partially updated record with id {}", id), 1)
                            } else {
                                ("404", format!("No record found with id {}", id), 0)
                            }
                        }
                        Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                        None => ("404", "Route not registered !!".to_string(), 0),
                    }
                } else {
                    (
                        "400",
                        "PATCH requires a specific resource ID".to_string(),
                        0,
                    )
                }
            } else {
                (
                    "400",
                    "PATCH requires a specific resource ID".to_string(),
                    0,
                )
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = patch_result;

    if !state.logs_disabled {
        match status_code {
            "200" => info!(
                date_time = date_time,
                status = status_code,
                method = "PATCH",
                path = requested_path,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
            _ => warn!(
                date_time = date_time,
                status = status_code,
                method = "PATCH",
                path = requested_path,
                error = message,
                elapsed_ms = elapsed,
                records = affected_records,
                "HTTP request"
            ),
        }
    }

    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

pub async fn handle_form_submission(
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Form(form_data): Form<FormData>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    if state.latency > 0 {
        sleep(Duration::from_millis(state.latency)).await;
    }

    if form_data.fields.is_empty() {
        let elapsed = start_time.elapsed().as_millis();
        if !state.logs_disabled {
            warn!(
                date_time = date_time,
                status = "422",
                method = "POST",
                path = requested_path,
                error = "Fields are empty!!",
                elapsed_ms = elapsed,
                records = 0,
                "HTTP request"
            );
        }
        return (
            StatusCode::OK,
            axum::Json(json!(
                {
                    "success": false,
                    "received": form_data.fields,
                }
            )),
        )
            .into_response();
    }

    let elapsed = start_time.elapsed().as_millis();
    if !state.logs_disabled {
        info!(
            date_time = date_time,
            status = "200",
            method = "POST",
            path = requested_path,
            elapsed_ms = elapsed,
            records = form_data.fields.len(),
            "HTTP request"
        );
    }
    return (
        StatusCode::OK,
        axum::Json(json!(
            {
                "success": true,
                "received": form_data.fields,
            }
        )),
    )
        .into_response();
}
