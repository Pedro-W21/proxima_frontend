#![feature(mpmc_channel)]
#![feature(string_from_utf8_lossy_owned)]

use std::{
    collections::HashSet, fs::File, io::Read, path::PathBuf, sync::{
        Arc, RwLock, atomic::{AtomicBool, Ordering}, mpmc::{Receiver, Sender}
    }
};

use chrono::Utc;
use futures_util::{StreamExt, TryFutureExt};
use openai::Credentials;
use proxima_backend::{
    ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant},
    database::{
        ClientUpdate, DatabaseError, DatabaseInfoRequest, DatabaseItem, DatabaseItemID, DatabaseReplyVariant, DatabaseRequestVariant, chats::ChatID, context::{ContextPart, ContextPosition}, media::{Media, MediaType}
    },
    web_payloads::{AIPayload, AIResponse, AuthPayload, AuthResponse, DBPayload, DBResponse},
};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tauri::{DragDropEvent, Emitter, Manager, PhysicalPosition, async_runtime::spawn};
use tauri_plugin_notification::NotificationExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
#[derive(Serialize, Deserialize)]
pub struct InitializeInnerArgs {
    pseudonym: String,
    password: String,
    local_ai_url: String,
    proxima_path: PathBuf,
}

#[tauri::command]
fn initialize(state: tauri::State<ProximaState>, inner: InitializeInnerArgs) -> bool {
    state.initialized.store(true, Ordering::Relaxed);
    true
}

#[derive(Serialize, Deserialize)]
pub struct HttpDBPostRequest {
    request: DBPayload,
    url: String,
}

#[tauri::command(async)]
async fn database_post_request(
    state: tauri::State<'_, ProximaState>,
    request: DBPayload,
    url: String,
) -> Result<DBResponse, ()> {
    let response = reqwest::Client::new()
        .post(url.clone())
        .json(&request)
        .send()
        .await;
    match response {
        Ok(data) => match data.json().await {
            Ok(data) => Ok(data),
            Err(error) => Err(()),
        },
        Err(error) => Err(()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpAIPostRequest {
    request: AIPayload,
    url: String,
    chat_id: ChatID,
}
#[derive(Serialize, Deserialize)]
pub struct SecondArgument {
    url: String,
    chat_id: ChatID,
}

#[tauri::command(async)]
async fn streaming_update_task(
    state: tauri::State<'_, ProximaState>,
    app_state: tauri::AppHandle,
    test: String,
    test2: String,
) -> Result<(), ()> {
    println!("[backend] Starting streaming update task");
    if !state.initialized.fetch_or(true, Ordering::Relaxed) {
        println!("[backend] Starting streaming update task");
        spawn(async move {
            let response = reqwest::Client::new()
                .post(test2)
                .json(&DBPayload::new(
                    test.clone(),
                    DatabaseRequestVariant::Info(DatabaseInfoRequest::UnknownUpdates {
                        access_key: test,
                    }),
                ))
                .send()
                .await;

            match response {
                Ok(data) => {
                    let mut stream = data.bytes_stream();
                    let mut token_id: u64 = 0;
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(bytes) => {
                                let u8s: Vec<u8> = bytes.to_vec();
                                let string = String::from_utf8_lossy_owned(u8s);
                                if let Ok(request_variant) =
                                    serde_json::from_str::<ClientUpdate>(&string)
                                {
                                    match request_variant.clone() {
                                        ClientUpdate::ItemUpdate(id, _) => {dbg!(id);},
                                        _ => ()
                                    }
                                    println!("[backend] emitting client update {token_id}");
                                    app_state
                                        .emit("client-update", (request_variant.clone(), token_id))
                                        .unwrap();
                                    token_id += 1;
                                } else {
                                    dbg!("Getting invalid events : ", string);
                                }
                            }
                            Err(error) => {}
                        }
                    }
                }
                Err(error) => (),
            }
        });
        Ok(())
    } else {
        Err(())
    }
}

#[tauri::command(async)]
async fn ai_endpoint_post_request(
    state: tauri::State<'_, ProximaState>,
    app_state: tauri::AppHandle,
    request: AIPayload,
    second: SecondArgument,
) -> Result<AIResponse, ()> {
    match request.request.clone() {
        EndpointRequestVariant::RespondToFullPrompt {
            whole_context,
            streaming,
            session_type,
            chat_settings,
            chat_id,
            access_mode,
        } => {
            if streaming {
                let response = reqwest::Client::new()
                    .post(second.url)
                    .json(&request)
                    .send()
                    .await;

                match response {
                    Ok(data) => {
                        let mut stream = data.bytes_stream();
                        let mut total = whole_context.clone();
                        let mut current_part = ContextPart::new(vec![], ContextPosition::AI);
                        let mut token_id: u64 = 0;
                        while let Some(item) = stream.next().await {
                            match item {
                                Ok(bytes) => {
                                    let u8s: Vec<u8> = bytes.to_vec();
                                    let string = String::from_utf8_lossy_owned(u8s);
                                    if let Ok(request_variant) =
                                        serde_json::from_str::<EndpointResponseVariant>(&string)
                                    {
                                        app_state
                                            .emit(
                                                "chat-token",
                                                (request_variant.clone(), second.chat_id, token_id),
                                            )
                                            .unwrap();
                                        token_id += 1;
                                        match request_variant {
                                            EndpointResponseVariant::StartStream(data, pos)
                                            | EndpointResponseVariant::ContinueStream(data, pos) => {
                                                println!("[backend] emitting chat-token event for chat {} ! event : {}", second.chat_id, data.get_text());
                                                if pos == *current_part.get_position() {
                                                    current_part.add_data(data);
                                                } else {
                                                    current_part.concatenate_text();
                                                    total.add_part(current_part.clone());
                                                    current_part =
                                                        ContextPart::new(vec![data], pos);
                                                }
                                            }
                                            _ => (),
                                        }
                                    } else {
                                        dbg!("Getting invalid events : ", string);
                                    }
                                }
                                Err(error) => {}
                            }
                        }
                        current_part.concatenate_text();
                        total.add_part(current_part);
                        Ok(AIResponse {
                            reply: EndpointResponseVariant::MultiTurnBlock(total),
                        })
                    }
                    Err(error) => Err(()),
                }
            } else {
                let response = reqwest::Client::new()
                    .post(second.url)
                    .json(&request)
                    .send()
                    .await;

                match response {
                    Ok(data) => match data.json().await {
                        Ok(data) => Ok(data),
                        Err(error) => Err(()),
                    },
                    Err(error) => Err(()),
                }
            }
        }
        EndpointRequestVariant::Continue => Err(()),
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpAuthPostRequest {
    request: AuthPayload,
    url: String,
}

#[tauri::command(async)]
async fn auth_post_request(
    state: tauri::State<'_, ProximaState>,
    request: AuthPayload,
    url: String,
) -> Result<AuthResponse, ()> {
    println!("making request");
    let response = reqwest::Client::new().post(url).json(&request).send().await;
    println!("Received response");
    match response {
        Ok(data) => {
            //println!("Response is okayyy");
            if data.status().is_success() {
                //println!("Response got JSON");
                match data.json().await {
                    Ok(data) => {
                        println!("got response");
                        Ok(data)
                    }
                    Err(error) => {
                        dbg!(error);
                        Err(())
                    }
                }
            } else {
                dbg!(data.status());
                Err(())
            }
        }
        Err(error) => {
            dbg!(error);
            Err(())
        }
    }
}

#[tauri::command(async)]
async fn show_notification(
    state: tauri::State<'_, ProximaState>,
    app_state: tauri::AppHandle,
    title: String,
    description: String,
) -> Result<(), ()> {
    println!("[backend] creating notification");
    app_state.notification().builder().title(title).body(description).show().map_err(|err| {
    println!("[backend] notification creation error : {err}");()})?;
    println!("[backend] notification send");
    Ok(())
}

#[tauri::command]
fn print_to_console(state: tauri::State<ProximaState>, value: String) {
    println!("[frontend] {}", value);
}


#[tauri::command(async)]
async fn add_media_from_file_if_exists(state: tauri::State<'_, ProximaState>, test1: PathBuf, test2:String, test3:String) -> Result<([u8 ; 32], String), ()> {
    println!("[backend] in add_media");
    let mut file = File::open(test1.clone()).map_err(|e| {})?;
    println!("[backend] opened file");
    let mut bytes = Vec::with_capacity(4096);
    file.read_to_end(&mut bytes).map_err(|e| {})?;
    println!("[backend] read file");

    let mut hasher = Sha3_256::new();
    hasher.update(&bytes);
    let hash:[u8 ; 32] = hasher.finalize().into();
    println!("[backend] hashed file");

    let request = DBPayload::new(test3.clone(), DatabaseRequestVariant::Get(DatabaseItemID::Media(hash)));
    let response = reqwest::Client::new()
        .post(test2.clone())
        .json(&request)
        .send()
        .await.map_err(|e| {})?;
    let data = response.json::<DBResponse>().await.map_err(|e| {})?;
    println!("[backend] decoded DB response");
    let file_name = match data.reply {
        DatabaseReplyVariant::ReturnedItem(DatabaseItem::Media(med, _)) => {

            println!("[backend] In existing media branch");
            med.file_name
        },
        DatabaseReplyVariant::Error(DatabaseError::ItemNotFound(DatabaseItemID::Media(_))) => {
            println!("[backend] in no media branch");
            let request = DBPayload::new(test3.clone(), DatabaseRequestVariant::Add(DatabaseItem::Media(Media { hash, media_type: MediaType::Image, file_name:test1.file_name().unwrap().to_string_lossy().to_string(), tags: HashSet::new(), access_modes: HashSet::from([0]), added_at: Utc::now() }, bytes)));
            let response = reqwest::Client::new()
                .post(test2.clone())
                .json(&request)
                .send()
                .await.map_err(|e| {})?;
            let data = response.json::<DBResponse>().await.map_err(|e| {})?;
            if let DatabaseReplyVariant::AddedItem(_) = data.reply {
                println!("[backend] added new media");
                let request = DBPayload::new(test3.clone(), DatabaseRequestVariant::Get(DatabaseItemID::Media(hash)));
                let response = reqwest::Client::new()
                    .post(test2.clone())
                    .json(&request)
                    .send()
                    .await.map_err(|e| {})?;
                let data = response.json::<DBResponse>().await.map_err(|e| {})?;
                if let DatabaseReplyVariant::ReturnedItem(DatabaseItem::Media(mem, _)) = data.reply {
                    mem.file_name.clone()
                }
                else {
                    return Err(())
                }
            }
            else {
                return Err(())
            }
        },
        _ => return Err(())
    };
    Ok((hash, file_name))
}

#[derive(Serialize, Clone)]
pub struct SpecialDragDrop {
    paths:Vec<PathBuf>,
    position:PhysicalPosition<f64>
}
pub struct ProximaState {
    initialized: AtomicBool,
    user_loaded: AtomicBool,
    auth_token: Arc<RwLock<String>>,
    username: Arc<RwLock<String>>,
    password: Arc<RwLock<String>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(ProximaState {
                initialized: AtomicBool::new(false),
                user_loaded: AtomicBool::new(false),
                auth_token: Arc::new(RwLock::new(String::new())),
                username: Arc::new(RwLock::new(String::new())),
                password: Arc::new(RwLock::new(String::new())),
            });
            Ok(())
        })
        .on_window_event(|window, event| 
        match event {
            tauri::WindowEvent::DragDrop(tauri::DragDropEvent::Drop { paths, position }) => {
                
                window.emit("special-drag-and-drop", SpecialDragDrop { paths:paths.clone(), position: position.clone() }).unwrap();
                println!("[backend] emitting drag drop event")
            },
            _ => ()
        }
        )
        .invoke_handler(tauri::generate_handler![
            greet,
            initialize,
            print_to_console,
            ai_endpoint_post_request,
            database_post_request,
            auth_post_request,
            streaming_update_task,
            show_notification,
            add_media_from_file_if_exists
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
