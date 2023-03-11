use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ics2000_rs::{Device, Ics, Room, Scene};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    path::Path as StdPath,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct AppState {
    ics: Arc<Mutex<Option<Ics>>>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    email: String,
    password: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState {
        ics: Arc::new(Mutex::new(None)),
    };
    let config_file = StdPath::new("settings.json");
    let ics_clone = Arc::clone(&state.ics);
    if config_file.exists() {
        let config_json =
            fs::read_to_string("settings.json").expect("Unable to read settings file");
        match serde_json::from_str::<Config>(&config_json) {
            Ok(config) => {
                ics_login(config.email, config.password, ics_clone).await;
            }
            Err(_) => {}
        };
    }
    let app = Router::new()
        .route("/login", post(login))
        .route("/devices", get(devices))
        .route("/devices/:device_id", post(device_action))
        .route("/rooms", get(rooms))
        .route("/scenes", get(scenes))
        .route("/scenes/:scene_id", post(scene_action))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ics_login(email: String, password: String, ics: Arc<Mutex<Option<Ics>>>) -> bool {
    let email_clone = email.clone();
    let password_clone = password.clone();
    let ics_clone = Arc::clone(&ics);
    let resp = tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().expect("Mutex was poisoned");
        *ics = Some(Ics::new(&email_clone, &password_clone, true));
        ics.as_mut().unwrap().login()
    })
    .await
    .expect("Error logging in");
    let config = Config { email, password };
    fs::write("settings.json", serde_json::to_string(&config).unwrap())
        .expect("Could not save settings file");
    resp
}

async fn login(State(state): State<AppState>, Json(payload): Json<Login>) -> StatusCode {
    match ics_login(payload.email, payload.password, state.ics).await {
        true => StatusCode::OK,
        false => StatusCode::FORBIDDEN,
    }
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

async fn device_action(
    State(state): State<AppState>,
    Path(device_id): Path<usize>,
    Json(payload): Json<DeviceAction>,
) -> StatusCode {
    let ics_clone = Arc::clone(&state.ics);
    tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().unwrap();
        match payload.state {
            DeviceState::On => ics.as_mut().unwrap().turn_on(device_id),
            DeviceState::Off => ics.as_mut().unwrap().turn_off(device_id),
            DeviceState::Dim(value) => ics.as_mut().unwrap().dim(device_id, value),
        }
    })
    .await
    .expect("Ics error");

    StatusCode::OK
}

async fn scene_action(
    State(state): State<AppState>,
    Path(scene_id): Path<usize>,
    Json(payload): Json<SceneAction>,
) -> StatusCode {
    let ics_clone = Arc::clone(&state.ics);
    tokio::task::spawn_blocking(move || {
        let mut ics = ics_clone.lock().unwrap();
        match payload.state {
            SceneState::Play => ics.as_mut().unwrap().start_scene(scene_id),
            SceneState::Stop => ics.as_mut().unwrap().stop_scene(scene_id),
        }
    })
    .await
    .expect("Ics error");

    StatusCode::OK
}
#[derive(Deserialize)]
struct Login {
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct DeviceAction {
    state: DeviceState,
}

#[derive(Deserialize)]
enum DeviceState {
    Off,
    On,
    Dim(usize),
}

#[derive(Deserialize)]
struct SceneAction {
    state: SceneState,
}

#[derive(Deserialize)]
enum SceneState {
    Play,
    Stop,
}
