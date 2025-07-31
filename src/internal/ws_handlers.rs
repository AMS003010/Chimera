use crate::internal::chimera::AppStateWs;
use crate::internal::helpers::compare_values;
use axum::body::Body;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{ConnectInfo, Path, State},
    response::Response,
};
use chrono::Utc;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConnectionState {
    id: String,
    ip: String,
    route: String,
    connected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct WsCommand {
    action: String,
}

pub async fn handle_websocket(
    Path(route): Path<String>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State((state, json_data, connections)): State<(
        Arc<AppStateWs>,
        Arc<RwLock<Value>>,
        Arc<RwLock<HashMap<String, ConnectionState>>>,
    )>,
) -> Response {
    let connection_id = Uuid::new_v4().to_string();
    let ip = addr.ip().to_string();

    if !state.logs_disabled {
        info!(
            timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
            connection_id = %connection_id,
            action = "CONNECT",
            route = %route,
            ip = %ip,
            "WebSocket connection established"
        );
    }

    // Validate route exists
    let data = json_data.read().await;
    if data.get(&route).is_none() {
        if !state.logs_disabled {
            info!(
                timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                connection_id = %connection_id,
                action = "REJECT",
                route = %route,
                "Route not found"
            );
        }
        return Response::builder()
            .status(404)
            .body(Body::from(format!("Route '{}' not found", route)))
            .unwrap();
    }
    drop(data); // Release the read lock

    // Store connection
    connections.write().await.insert(
        connection_id.clone(),
        ConnectionState {
            id: connection_id.clone(),
            ip: ip.clone(),
            route: route.clone(),
            connected_at: Utc::now(),
        },
    );

    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            state,
            json_data,
            connections,
            connection_id,
            route,
            ip,
        )
    })
}

// WebSocket connection handler
pub async fn handle_socket(
    mut socket: WebSocket,
    state: Arc<AppStateWs>,
    json_data: Arc<RwLock<Value>>,
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    connection_id: String,
    route: String,
    ip: String,
) {
    // Send initial data
    if let Err(e) = send_route_data(
        &mut socket,
        &json_data,
        state.clone(),
        &route,
        &connection_id,
        &ip,
    )
    .await
    {
        error!(
            connection_id = %connection_id,
            error = %e,
            "Initial data send failed"
        );
        cleanup_connection(&connections, &connection_id, state).await;
        return;
    }

    // Message handling loop
    while let Some(msg_result) = socket.next().await {
        match msg_result {
            Ok(msg) => {
                match msg {
                    Message::Text(text) => {
                        if let Err(e) = handle_text_message(
                            &mut socket,
                            text,
                            &json_data,
                            state.clone(),
                            &route,
                            connections.clone(),
                            &connection_id,
                            &ip,
                        )
                        .await
                        {
                            error!(
                                connection_id = %connection_id,
                                error = %e,
                                "Error handling text message"
                            );
                            break;
                        }
                    }
                    Message::Close(_reason) => {
                        if !state.logs_disabled {
                            log_close(&connection_id);
                        }
                        break;
                    }
                    Message::Ping(data) => {
                        if let Err(e) = socket.send(Message::Pong(data)).await {
                            error!(
                                connection_id = %connection_id,
                                error = %e,
                                "Failed to send pong"
                            );
                            break;
                        }
                    }
                    _ => {} // Ignore other message types
                }
            }
            Err(e) => {
                error!(
                    connection_id = %connection_id,
                    error = %e,
                    "WebSocket error"
                );
                break;
            }
        }
    }

    // Clean up on disconnect
    cleanup_connection(&connections, &connection_id, state).await;
}

// Helper: Send route data
pub async fn send_route_data(
    socket: &mut WebSocket,
    json_data: &Arc<RwLock<Value>>,
    state: Arc<AppStateWs>,
    route: &str,
    connection_id: &str,
    ip: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = json_data.read().await;
    let response = match data.get(route) {
        Some(route_data) => {
            let mut value = route_data.clone();

            // Apply sorting if specified for this route
            if let Some((order, key)) = state.sort_rules.get(&route.to_string()) {
                if let Value::Array(arr) = &mut value {
                    if arr.len() > 1 {
                        use rayon::slice::ParallelSliceMut;
                        arr.par_sort_by(|a, b| compare_values(a, b, key, order));
                    }
                }
            }

            // Apply pagination if enabled
            if state.paginate > 0 {
                if let Value::Array(arr) = &value {
                    if arr.len() > state.paginate as usize {
                        value = Value::Array(arr[..state.paginate as usize].to_vec());
                    }
                }
            }

            json!({
                "status": "success",
                "route": route,
                "data": value
            })
        }
        None => json!({
            "status": "error",
            "message": format!("Route '{}' not found", route)
        }),
    };
    drop(data); // Release the read lock

    let msg = response.to_string();
    if !state.logs_disabled {
        info!(
            timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
            connection_id = %connection_id,
            action = "SEND",
            bytes = msg.len(),
            ip = %ip,
            "Sending route data"
        );
    }

    socket
        .send(Message::Text(msg))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

// Helper: Send route command
pub async fn send_route_command(
    socket: &mut WebSocket,
    json_data: &Arc<RwLock<Value>>,
    connection_id: &str,
    ip: &str,
    state: Arc<AppStateWs>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data = json_data.read().await;
    let msg = data.to_string();
    if !state.logs_disabled {
        info!(
            timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
            connection_id = %connection_id,
            action = "SEND",
            bytes = msg.len(),
            ip = %ip,
            "Sending command response"
        );
    }

    socket
        .send(Message::Text(msg))
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

// Helper: Handle text messages
pub async fn handle_text_message(
    socket: &mut WebSocket,
    text: String,
    json_data: &Arc<RwLock<Value>>,
    state: Arc<AppStateWs>,
    route: &str,
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    connection_id: &str,
    ip: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Process commands
    if let Ok(cmd) = serde_json::from_str::<WsCommand>(&text) {
        match cmd.action.as_str() {
            "refresh" => {
                if !state.logs_disabled {
                    info!(
                        timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        connection_id = %connection_id,
                        action = "RECV",
                        bytes = text.len(),
                        command = "refresh",
                        "Received command"
                    );
                }
                send_route_data(socket, json_data, state, route, connection_id, ip).await?;
            }
            "connections" => {
                if !state.logs_disabled {
                    info!(
                        timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        connection_id = %connection_id,
                        action = "RECV",
                        bytes = text.len(),
                        command = "connections",
                        "Received command"
                    );
                }

                let connections_data = connections.read().await;

                // Create the response Value
                let response_value = json!({
                    "status": "success",
                    "connections": connections_data.iter().map(|(id, conn)| {
                        json!({
                            "id": id,
                            "ip": conn.ip,
                            "route": conn.route,
                            "connected_at": conn.connected_at.to_rfc3339(),
                            "duration_seconds": Utc::now().signed_duration_since(conn.connected_at).num_seconds()
                        })
                    }).collect::<Vec<_>>()
                });

                // Wrap in Arc<RwLock<Value>> as expected by send_route_command
                let wrapped_response = Arc::new(RwLock::new(response_value));

                send_route_command(socket, &wrapped_response, connection_id, ip, state).await?;
            }
            _ => {
                if !state.logs_disabled {
                    info!(
                        timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        connection_id = %connection_id,
                        action = "RECV",
                        bytes = text.len(),
                        command = "unknown",
                        "Received unknown command, echoing"
                    );
                }
                socket.send(Message::Text(text)).await?;
            }
        }
    } else {
        if !state.logs_disabled {
            info!(
                timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                connection_id = %connection_id,
                action = "RECV",
                bytes = text.len(),
                message = %text,
                "Received text message"
            );
        }
        socket.send(Message::Text(text)).await?;
    }

    Ok(())
}

// Helper: Log close reason
pub fn log_close(connection_id: &str) {
    info!(
        timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
        connection_id = %connection_id,
        action = "CLOSE",
        "WebSocket connection closed"
    );
}

// Helper: Clean up connection
pub async fn cleanup_connection(
    connections: &Arc<RwLock<HashMap<String, ConnectionState>>>,
    connection_id: &str,
    state: Arc<AppStateWs>,
) {
    if let Some(conn) = connections.write().await.remove(connection_id) {
        if !state.logs_disabled {
            info!(
                timestamp = %Utc::now().format("%Y-%m-%d %H:%M:%S"),
                connection_id = %connection_id,
                action = "DISCONNECT",
                duration_seconds = Utc::now().signed_duration_since(conn.connected_at).num_seconds(),
                "WebSocket connection disconnected"
            );
        }
    }
}

// WebSocket fallback handler
pub async fn ws_fallback_handler() -> Response {
    Response::builder()
        .status(404)
        .body(Body::from(
            "WebSocket endpoint requires a route: /ws/{route}",
        ))
        .unwrap()
}