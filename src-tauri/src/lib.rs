#![feature(mpmc_channel)]
#![feature(string_from_utf8_lossy_owned)]

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpmc::{Receiver, Sender},
        Arc, RwLock,
    },
};

use openai::Credentials;
use proxima_backend::{ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant}, database::{chats::ChatID, context::{ContextPart, ContextPosition}, DatabaseReplyVariant, DatabaseRequestVariant}, web_payloads::{AIPayload, AIResponse, AuthPayload, AuthResponse, DBPayload, DBResponse}};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use futures_util::StreamExt;

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
    request:DBPayload,
    url:String,
}

#[tauri::command(async)]
async fn database_post_request(state: tauri::State<'_,ProximaState>, request:DBPayload, url:String) -> Result<DBResponse, ()> {
    let response = reqwest::Client::new()
        .post(url.clone())
        .json(&request)
        .send()
        .await;
    match response {
        Ok(data) => {
            match data.json().await {
                Ok(data) => Ok(data),
                Err(error) => Err(())
            }
        },
        Err(error) => Err(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct HttpAIPostRequest {
    request:AIPayload,
    url:String,
    chat_id:ChatID
}
#[derive(Serialize, Deserialize)]
pub struct SecondArgument {
    url:String,
    chat_id:ChatID
}

#[tauri::command(async)]
async fn ai_endpoint_post_request(state: tauri::State<'_,ProximaState>, app_state:tauri::AppHandle, request:AIPayload, second:SecondArgument) -> Result<AIResponse, ()> {
    match request.request.clone() {
        EndpointRequestVariant::RespondToFullPrompt { whole_context, streaming, session_type, chat_settings } => if streaming {
            let response = reqwest::Client::new()
                .post(second.url)
                .json(&request)
                .send()
                .await;

            match response {
                Ok(data) => {
                    let mut stream = data.bytes_stream();
                    let mut total = ContextPart::new(vec![], ContextPosition::AI);
                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(bytes) => {
                                let u8s:Vec<u8> = bytes.to_vec();
                                let string = String::from_utf8_lossy_owned(u8s);
                                if let Ok(request_variant) = serde_json::from_str::<EndpointResponseVariant>(&string) {
                                    app_state.emit("chat-token", (request_variant.clone(), second.chat_id));
                                    match request_variant {
                                        EndpointResponseVariant::StartStream(data, _) | EndpointResponseVariant::ContinueStream(data, _) => {
                                            total.add_data(data);
                                        },
                                        _ => ()
                                    }
                                }
                                
                            },
                            Err(error) => {

                            },
                        }
                    }
                    total.concatenate_text();
                    dbg!(total.get_data()[0].get_text());
                    Ok(AIResponse { reply: EndpointResponseVariant::Block(total) })
                },
                Err(error) => Err(())
            }
        }
        else {
            let response = reqwest::Client::new()
                .post(second.url)
                .json(&request)
                .send()
                .await;

            match response {
                Ok(data) => {
                    match data.json().await {
                        Ok(data) => {Ok(data)},
                        Err(error) => Err(())
                    }
                },
                Err(error) => Err(())
            }
        },
        EndpointRequestVariant::Continue => Err(()),
    }
    
}

#[derive(Serialize, Deserialize)]
pub struct HttpAuthPostRequest {
    request:AuthPayload,
    url:String,
}

#[tauri::command(async)]
async fn auth_post_request(state: tauri::State<'_,ProximaState>, request:AuthPayload, url:String) -> Result<AuthResponse, ()> {
    let response = reqwest::Client::new()
        .post(url)
        .json(&request)
        .send()
        .await;
    //println!("Received response");
    match response {
        Ok(data) => {
            //println!("Response is okayyy");
            if data.status().is_success() {
                //println!("Response got JSON");
                match data.json().await {
                    Ok(data) => {
                        //println!("Rededed");
                        Ok(data)},
                    Err(error) => Err(())
                }
            }
            else {
                Err(())
            }
            
        },
        Err(error) => Err(())
    }
}

#[tauri::command]
fn print_to_console(state: tauri::State<ProximaState>, value: String) {
    println!("PRINTING {}", value);
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
        .invoke_handler(tauri::generate_handler![
            greet,
            initialize,
            print_to_console,
            ai_endpoint_post_request,
            database_post_request,
            auth_post_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
