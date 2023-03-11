use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ics2000_rs::{Device, Ics, Room, Scene};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct AppState {
    ics: Arc<Mutex<Option<Ics>>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        ics: Arc::new(Mutex::new(None)),
    };
    let app = Router::new()
        .route("/login", post(login))
        .route("/devices", get(devices))
        .route("/rooms", get(rooms))
        .route("/scenes", get(scenes))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn login(State(state): State<AppState>, Json(payload): Json<Login>) -> StatusCode {
    let ics_clone = Arc::clone(&state.ics);
    tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().expect("Mutex was poisoned");
        *ics = Some(Ics::new(&payload.email, &payload.password, true));
        ics.as_mut().unwrap().login();
    })
    .await
    .expect("Something went wrong logging in");
    StatusCode::OK
}

async fn devices(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<Device>>), (StatusCode, String)> {
    let ics_clone = Arc::clone(&state.ics);
    let devices = tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().unwrap();
        if ics.is_none() {
            return Err("Not logged in");
        }
        ics.as_mut().unwrap().get_devices()
    })
    .await
    .expect("Could not fetch devices");

    match devices {
        Ok(result) => Ok((StatusCode::OK, Json(result))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn rooms(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<Room>>), (StatusCode, String)> {
    let ics_clone = Arc::clone(&state.ics);
    let rooms = tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().unwrap();
        if ics.is_none() {
            return Err("Not logged in");
        }
        ics.as_mut().unwrap().get_rooms()
    })
    .await
    .expect("Could not fetch rooms");

    match rooms {
        Ok(result) => Ok((StatusCode::OK, Json(result))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn scenes(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<Scene>>), (StatusCode, String)> {
    let ics_clone = Arc::clone(&state.ics);
    let scenes = tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().unwrap();
        if ics.is_none() {
            return Err("Not logged in");
        }
        ics.as_mut().unwrap().get_scenes()
    })
    .await
    .expect("Could not fetch scenes");

    match scenes {
        Ok(result) => Ok((StatusCode::OK, Json(result))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}
