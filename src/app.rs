use std::{collections::HashSet, path::PathBuf, thread, time::Duration};

use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::{html::ChildrenProps, prelude::*, virtual_dom::VNode};
use gloo_utils::format::JsValueSerdeExt;
use proxima_backend::{ai_interaction::{endpoint_api::{EndpointRequestVariant, EndpointResponseVariant}, tools::{ProximaTool, Tools}}, database::{access_modes::AccessMode, chats::{Chat, SessionType}, configuration::{ChatConfiguration, ChatSetting}, context::{ContextData, ContextPart, ContextPosition, WholeContext}, description::Description, devices::DeviceID, tags::{NewTag, Tag, TagID}, DatabaseItem, DatabaseItemID, DatabaseReplyVariant, DatabaseRequestVariant, ProxDatabase}, web_payloads::{AIPayload, AIResponse, AuthPayload, AuthResponse, DBPayload, DBResponse}};
use yew::prelude::*;
use selectrs::yew::{Select, Group};
use markdown::to_html;

use crate::db_sync::{handle_add, UserCursors};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}
#[derive(Serialize, Deserialize)]
struct EmptyArgs {
    
}


#[derive(Serialize, Deserialize)]
pub struct HttpAuthPostRequest {
    request:AuthPayload,
    url:String,
}


#[derive(Serialize, Deserialize)]
pub struct HttpDBPostRequest {
    request:DBPayload,
    url:String,
}

#[derive(Serialize, Deserialize)]
pub struct HttpAIPostRequest {
    request:AIPayload,
    url:String,
}

#[derive(Serialize, Deserialize)]
struct PrintArgs {
    value:String,
}

#[derive(Serialize, Deserialize)]
struct InitializeArgs {
    inner:InitializeInnerArgs
}
#[derive(Serialize, Deserialize)]
struct InitializeInnerArgs {
    pseudonym:String,
    local_ai_url:String,
    proxima_path:PathBuf,
}

#[function_component(Initialize)]
pub fn initialize_page() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let pseudonym_input = use_node_ref();
    let prox_folder_input = use_node_ref();
    let local_ai_url_input = use_node_ref();
    let initialize = {
        let pseudonym_input_clone = pseudonym_input.clone();
        let prox_folder_input_clone = prox_folder_input.clone();
        let local_ai_url_input_clone = local_ai_url_input.clone();
        let first_clone = proxima_state.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let test = pseudonym_input_clone.clone();
            let test2 = prox_folder_input_clone.clone();
            let test3 = local_ai_url_input_clone.clone();
            let second_clone = first_clone.clone();
            spawn_local(async move {
                let pseudonym = test
                .cast::<web_sys::HtmlInputElement>()
                .unwrap()
                .value();

                let local_ai_url = test3
                .cast::<web_sys::HtmlInputElement>()
                .unwrap()
                .value();

                let password = test2
                .cast::<web_sys::HtmlInputElement>()
                .unwrap()
                .value();

                let json_request = proxima_backend::web_payloads::AuthPayload::new(password.trim().to_string(),pseudonym.clone().trim().to_string());
                let args = serde_wasm_bindgen::to_value(&HttpAuthPostRequest {request:json_request, url:local_ai_url.clone() + "/auth"}).unwrap();

                let return_val = invoke("auth_post_request", args).await;

                let value =
                 return_val
                 .into_serde::<AuthResponse>();
    
                match &value {
                    Ok(_) => (),
                    Err(error) => {
                        let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("{:?}", error)}).unwrap();
                        invoke("print_to_console", args).await;
                    }
                }

                let value = value;

                match value {
                    Ok(response) => {
                        second_clone.dispatch(ProximaStateAction::ChangeAuthToken(response.session_token.clone()));
                        second_clone.dispatch(ProximaStateAction::ChangeDeviceID(response.device_id));
                        second_clone.dispatch(ProximaStateAction::ChangeUsername(pseudonym.clone().trim().to_string()));
                        second_clone.dispatch(ProximaStateAction::ChangeChatURL(local_ai_url.clone()));
                        second_clone.dispatch(ProximaStateAction::ChangeInit(true));
                        second_clone.dispatch(ProximaStateAction::ChangeLoaded(true));

                        
                        let json_request = proxima_backend::web_payloads::DBPayload::new(response.session_token.clone(), DatabaseRequestVariant::GetAll);
                        let value = make_db_request(json_request, local_ai_url).await;
                        match value {
                            Ok(response) => match response.reply {
                                DatabaseReplyVariant::ReplyAll(value) => {

                                    second_clone.dispatch(ProximaStateAction::ChangeStartDB(Some(value)));
                                },
                                _ => {
                                }
                            },
                            Err(_) => ()
                        }

                    },
                    Err(_) => ()
                }

                let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("AAAAAAAAAAA")}).unwrap();
                invoke("print_to_console", args).await;

            });
        })
    };

    html! {
        <main class="container">
            <div class="first-level standard-padding-margin-corners">
                <h1>{"Welcome to Proxima !"}</h1>

                <p>{"We need a bit of info to get you started :"}</p>
                
                <form onsubmit={initialize} class="second-level standard-padding-margin-corners">
                    <input class="standard-padding-margin-corners most-horizontal-space-no-flex" id="pseudo-input" ref={pseudonym_input} placeholder="Enter your pseudonym..." />
                    <br/>
                    <input class="standard-padding-margin-corners most-horizontal-space-no-flex" id="prox-input" ref={prox_folder_input} placeholder="Enter your password"/>
                    <br/>
                    <input class="standard-padding-margin-corners most-horizontal-space-no-flex" id="local-input" ref={local_ai_url_input} placeholder="Enter a URL for your Proxima endpoint..." />
                    <br/>
                    <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" type="submit">{"Start"}</button>
                </form>
            </div>
        </main>
    }
}

async fn make_db_request(payload:DBPayload, backend_url:String) -> Result<DBResponse, ()> {
    let args = serde_wasm_bindgen::to_value(&HttpDBPostRequest {request:payload, url:backend_url + "/db"}).unwrap();

    let return_val = invoke("database_post_request", args).await;
    
    let value =
    return_val
    .into_serde::<DBResponse>();
    value.map_err(|error| {})
}

async fn make_ai_request(payload:AIPayload, backend_url:String) -> Result<AIResponse, ()> {
    let args = serde_wasm_bindgen::to_value(&HttpAIPostRequest {request:payload, url:backend_url + "/ai"}).unwrap();

    let return_val = invoke("ai_endpoint_post_request", args).await;
    
    let value =
    return_val
    .into_serde::<AIResponse>();
    value.map_err(|error| {})
}

#[function_component(Loading)]
pub fn loading_page() -> Html {
    html! {
        <main class="container">
            <h1>{"Setting up Proxima..."}</h1>

            <p>{"Please wait"}</p>
            
            
        </main>
    }
}

#[function_component(MainPage)]
pub fn app_page() -> Html {
    let chosen_tab = use_state_eq(|| 0_usize);
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let client_database = use_state_eq(|| {ProxDatabase::new_just_data(proxima_state.username.clone(), String::from("Don't care, local"))});
    let got_start = use_state_eq(|| false);
    let user_cursors = use_state_eq(|| UserCursors::zero());
    {
        let proxima_state = proxima_state.clone();
        let client_database = client_database.clone();
        let got_start = got_start.clone();
        use_effect(move || {
            if !*got_start.clone() {
                let db_clone = proxima_state.start_db.clone();
                let db_clone_is_ok = db_clone.is_some();
                if db_clone_is_ok {
                    client_database.set(proxima_state.start_db.clone().unwrap());
                    got_start.set(true);
                }
                
            }
            
        });
    }
    
    /*let database_data = use_state_eq(|| SharedProximaData::new());
    {  
        let database_data_setter = database_data.clone();
        use_effect(move || {
            spawn_local(async move {
                let args = serde_wasm_bindgen::to_value(&EmptyArgs{}).unwrap();
                database_data_setter.set(invoke("get_database_copy", args).await.into_serde().unwrap());
            });
        });
    }*/

    let mut values = Vec::with_capacity(4);
    for i in 0..8 {
        if *chosen_tab == i {
            values.push(String::from("chosen"));
        }
        else {
            values.push(String::from("not-chosen"));
        }
    }
    let tab_picker_callbacks:Vec<Callback<MouseEvent>> = (0..8).into_iter().map(|i| {
        let chosen_tab = chosen_tab.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            let prev_tab = (*chosen_tab).clone();
            chosen_tab.set(i);
            
        })
    }).collect();


    let selected_value = use_state(|| "Option1".to_string());
    let chosen_chat = use_state(|| None);
    let chosen_tag:UseStateHandle<Option<usize>>= use_state(|| None);
    let chosen_parent_tag:UseStateHandle<Option<usize>>= use_state(|| None);
    let chosen_access_mode = use_state_eq(|| {0_usize});
    let chosen_tags_for_am = use_state_eq(|| {HashSet::<TagID>::new()});
    let chosen_am_for_creation:UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let cc_in_modification:UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let cc_in_use:UseStateHandle<Option<usize>> = use_state_eq(|| None);
    let setting_in_modification:UseStateHandle<Option<ChatSetting>> = use_state_eq(|| None);

    let pseudo_node_ref = use_node_ref();
    let prompt_node_ref = use_node_ref();
    let tag_desc_ref = use_node_ref();
    let tag_name_ref = use_node_ref();
    let am_name_ref = use_node_ref();
    let cc_select_ref = use_node_ref();
    let cc_name_ref = use_node_ref();
    let cc_setting_ref = use_node_ref();
    let cc_setting_value_ref = use_node_ref();
    let second_db_here = client_database.clone();
    let current_app = match *(chosen_tab.clone()) {
        /*Home*/ 0 => {
            let onchange = {
                let selected_value = selected_value.clone();
                Callback::from(move |new_value: String| {
                    selected_value.set(new_value);
                })
            };

            let prompt_send_callback = {
                let prompt = prompt_node_ref.clone();
                let proxima_state = proxima_state.clone();

                let chosen_tab = chosen_tab.clone();
                let chosen_chat = chosen_chat.clone();
                let user_cursors = user_cursors.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    chosen_tab.set(1);
                    let prompt_text = prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    let starting_context = WholeContext::new(vec![ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User)]);
                    let mut database_copy = (*client_database).clone();
                    let mut local_id = database_copy.chats.create_chat(starting_context.clone(), None, proxima_state.device_id, None);
                    let start_chat = database_copy.chats.get_chats().get(&local_id).unwrap().clone();
                    client_database.set(database_copy.clone());
                    let proxima_state = proxima_state.clone();
                    let client_database= client_database.clone();
                    chosen_tab.set(1);
                    chosen_chat.set(Some(local_id));
                    let user_cursors = user_cursors.clone();
                    spawn_local(async move {


                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Add(DatabaseItem::Chat(start_chat.clone())) };
                        let (new_cursors, new_id) = match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                            Ok(response) => handle_add(
                                &mut database_copy,
                                DatabaseItemID::Chat(local_id),
                                DatabaseItem::Chat(start_chat.clone()),
                                response.reply,
                                (*user_cursors).clone(),
                                async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                            ).await,
                            Err(()) => ((*user_cursors).clone(), DatabaseItemID::Chat(local_id))
                        };
                        user_cursors.set(new_cursors);
                        if new_id != DatabaseItemID::Chat(local_id) {
                            local_id = match new_id {
                                DatabaseItemID::Chat(id) => id,
                                _ => panic!("Wrong kind of ID after check, impossible")
                            };
                        }

                        let json_request = proxima_backend::web_payloads::AIPayload::new(proxima_state.auth_token.clone(), EndpointRequestVariant::RespondToFullPrompt { whole_context: starting_context, streaming: false, session_type: SessionType::Chat, chat_settings:None });
                        

                        let value = make_ai_request(json_request, proxima_state.chat_url.clone()).await;

                        match value {
                            Ok(response) => {
                                match response.reply {
                                    EndpointResponseVariant::Block(context_part) => {
                                        let mut db = database_copy;
                                        let chat = db.chats.get_chats_mut().get_mut(&local_id).unwrap();
                                        chat.add_to_context(context_part);
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => (),
                                            Err(()) => ()
                                        }
                                        client_database.set(db);
                                    },
                                    _ => ()
                                }
                            },
                            Err(_) => ()
                        }


                    });
                })
            };
            html!{
                <div>
                <div class="multi-input-container standard-padding-margin-corners first-level">
                    <div class="label-input-combo standard-padding-margin-corners second-level">
                        <p>{"Name (Optional) : "}</p>
                        <input class="standard-padding-margin-corners" placeholder="Enter a name here..."/>
                        
                        <button class="mainapp-button standard-padding-margin-corners">{"Set"}</button>
                    </div>
                    <div class="label-input-combo standard-padding-margin-corners second-level">
                        <p>{"Pseudonym (Obligatory) : "}</p>
                        <input class="standard-padding-margin-corners" placeholder="Enter a pseudonym here..."/>
                        <button class="mainapp-button">{"Set"}</button>
                    </div>
                    <div class="label-input-combo standard-padding-margin-corners second-level">
                        <p>
                            {"What do you want to be called by ?"}
                        </p>
                    </div>
                    
                </div>
                <hr/>
                <div class="multi-input-container standard-padding-margin-corners second-level">
                    <div class="label-input-combo standard-padding-margin-corners third-level">
                        <p>{"Local AI URL : "}</p>
                        <input class="standard-padding-margin-corners" placeholder="Enter a valid URL to an Proxima backend here"/>
                        <button class="mainapp-button standard-padding-margin-corners">{"Check URL ?"}</button>
                    </div>

                    <div class="label-input-combo standard-padding-margin-corners third-level">
                        <input class="standard-padding-margin-corners" placeholder="Have a prompt ?" ref={prompt_node_ref}/>

                        <button class="mainapp-button standard-padding-margin-corners" onclick={prompt_send_callback}>{"Send"}</button>
                    </div>
                </div>
                
                
            </div>  
            }
        },
        /*Chat*/ 1 => 
        {
            let prompt_send_callback = {
                let prompt = prompt_node_ref.clone();
                let proxima_state = proxima_state.clone();
                let client_database = client_database.clone();
                let chosen_chat = chosen_chat.clone();
                let chosen_tab = chosen_tab.clone();
                let cc_in_use = cc_in_use.clone();
                let chosen_access_mode = chosen_access_mode.clone();
                let user_cursors = user_cursors.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let prompt_text = prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap().set_value("");
                    let mut database_copy = (*client_database).clone();
                    let (mut local_id, starting_context, created, start_chat, config_opt) = match *(chosen_chat.clone()) {
                        Some(chatid) => {
                            let mut chat = database_copy.chats.get_chats_mut().get_mut(&chatid).unwrap();
                            let (context_part, config_opt) = match *cc_in_use {
                                Some(config) => {
                                    chat.config = Some(config);
                                    let config_clone = database_copy.configs.get_configs()[config].clone();
                                    chat.latest_used_config = Some(config_clone.clone());
                                    match &config_clone.tools {
                                        Some(tools) => {
                                            (ContextPart::new_user_prompt_with_tools(vec![ContextData::Text(prompt_text)]), Some(config_clone))
                                        },
                                        None => (ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User), Some(config_clone))
                                    }
                                },
                                None => {
                                    chat.config = None;
                                    chat.latest_used_config = None;
                                    (ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User), None)
                                }
                            };
                            chat.add_to_context(context_part);
                            (chatid, chat.get_context().clone(), false, chat.clone(), config_opt)
                        },
                        None => {
                            let (starting_context, config_opt) = match *cc_in_use {
                                Some(config) => {
                                    let config_clone = database_copy.configs.get_configs()[config].clone();
                                    match config_clone.tools.clone() {
                                        Some(tools) => {
                                            (WholeContext::new_with_all_settings(vec![ContextPart::new_user_prompt_with_tools(vec![ContextData::Text(prompt_text)])], &config_clone), Some(config_clone))
                                        },
                                        None => (WholeContext::new_with_all_settings(vec![ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User)], &config_clone), Some(config_clone))
                                    }
                                },
                                None => {
                                    (WholeContext::new(vec![ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User)]), None)
                                }
                            };
                            let new_chatid = database_copy.chats.create_chat(starting_context.clone(), None, proxima_state.device_id, config_opt.clone());

                            let mut chats = database_copy.chats.get_chats_mut().get_mut(&new_chatid).unwrap();
                            chats.access_modes.insert(*chosen_access_mode);
                            chosen_chat.set(Some(new_chatid));
                            (new_chatid, starting_context, true, database_copy.chats.get_chats_mut().get_mut(&new_chatid).unwrap().clone(), config_opt)
                        }
                    };
                    chosen_tab.set(1);
                    client_database.set(database_copy.clone());
                    let proxima_state = proxima_state.clone();
                    let client_database= client_database.clone();
                    let user_cursors = user_cursors.clone();
                    spawn_local(async move {
                        if created {
                            let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Add(DatabaseItem::Chat(start_chat.clone())) };
                            let (new_cursors, new_id) = match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                Ok(response) => handle_add(
                                    &mut database_copy,
                                    DatabaseItemID::Chat(local_id),
                                    DatabaseItem::Chat(start_chat.clone()),
                                    response.reply,
                                    (*user_cursors).clone(),
                                    async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                ).await,
                                Err(()) => ((*user_cursors).clone(), DatabaseItemID::Chat(local_id))
                            };
                            user_cursors.set(new_cursors);
                            if new_id != DatabaseItemID::Chat(local_id) {
                                local_id = match new_id {
                                    DatabaseItemID::Chat(id) => id,
                                    _ => panic!("Wrong kind of ID after check, impossible")
                                };
                            }
                        }

                        let json_request = proxima_backend::web_payloads::AIPayload::new(proxima_state.auth_token.clone(), EndpointRequestVariant::RespondToFullPrompt { whole_context: starting_context, streaming: false, session_type: SessionType::Chat, chat_settings:config_opt });


                        let value = make_ai_request(json_request, proxima_state.chat_url.clone()).await;

                        match value {
                            Ok(response) => {
                                match response.reply {
                                    EndpointResponseVariant::Block(context_part) => {
                                        let mut db = database_copy;
                                        let chat = db.chats.get_chats_mut().get_mut(&local_id).unwrap();
                                        chat.add_to_context(context_part);
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => (),
                                            Err(()) => ()
                                        }
                                        client_database.set(db);
                                    },
                                    EndpointResponseVariant::MultiTurnBlock(whole_context) => {
                                        let mut db = database_copy;
                                        let chat = db.chats.get_chats_mut().get_mut(&local_id).unwrap();
                                        chat.context = whole_context;
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => (),
                                            Err(()) => ()
                                        }
                                        client_database.set(db);
                                    }
                                    _ => ()
                                }
                            },
                            Err(_) => ()
                        }


                    });
                })
            };

            let new_chat_callback = {
                let chosen_chat = chosen_chat.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    chosen_chat.set(None);
                })
            };
            let chosen_access_mode = chosen_access_mode.clone();
            let chat_htmls = client_database.chats.get_chats().iter().map(|(id, chat)| {
                let chosen_access_mode = chosen_access_mode.clone();
                let callback = {
                    let chosen_chat_clone = chosen_chat.clone();
                    let id_clone = *id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        chosen_chat_clone.set(Some(id_clone));
                    })
                };

                if !chat.access_modes.contains(&*chosen_access_mode) {
                    html!()
                }
                else if chosen_chat.is_some() && *id == chosen_chat.unwrap() {
                    html!(

                        <div><button onclick={callback} class="chat-option chosen-chat">{match chat.get_title() {Some(title) => title.clone(), None => format!("Chat {}", *id)}}</button></div>
                    )
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{match chat.get_title() {Some(title) => title.clone(), None => format!("Chat {}", *id)}}</button></div>
                    )
                }
            }).collect::<Html>();
            let chosen_chat_by_id = client_database.chats.get_chats().get(&(chosen_chat.unwrap_or(1000000)));
            let config_htmls:Vec<Html> = second_db_here.configs.get_configs().iter().enumerate().map(|(id, config)| {
                html!(
                    <option value={config.name.clone()}>{config.name.clone()}</option>
                )
            }).collect();

            let cc_select_callback = {
                let cc_in_use = cc_in_use.clone();
                let select_node = cc_select_ref.clone();
                let db_clone = second_db_here.clone();
                Callback::from(move |mouse_evt:Event| {
                    let mut db_copy = (*db_clone).clone();
                    let cc_name = select_node.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    match db_copy.configs.get_configs().iter().enumerate().find(|(i,config)| {
                        &config.name == &cc_name
                    }) {
                        Some((id, config)) => {
                            let second_db_clone = db_clone.clone();
                            let config_data = format!("{:?} {:?}", config.raw_settings.clone(), config.tools.is_some());
                            cc_in_use.set(Some(id));
                            spawn_local(async move {
                                let args = serde_wasm_bindgen::to_value(&PrintArgs {value:config_data}).unwrap();
                                invoke("print_to_console", args).await;
                            });
                        },
                        None => {
                            cc_in_use.set(None);
                        }
                    }
                    
                })
            };
            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">
                        <h1>{"Past chats"}</h1>
                        <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_chat_callback}>{"New chat"}</button>
                        <hr/>
                        <div class="list-holder">
                            {
                                if client_database.chats.get_chats().len() > 0 {
                                    chat_htmls
                                }
                                else {
                                    html!({"No chats yet !"})
                                }
                            }
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space chat-tab-not-sidebar">
                        <div class="chat-tab-current-chat second-level standard-padding-margin-corners">
                            <h1>{
                            match chosen_chat_by_id {
                                Some(chat) => match &chat.chat_title {
                                    Some(title) => title.clone(),
                                    None => format!("Untitled Chat {}", chat.id),
                                },
                                None => "Please select a chat or start one :)".to_string()
                            }} 
                            </h1>
                            <div class="list-holder all-vertical-space-flex">
                            {
                                match chosen_chat_by_id {
                                    Some(chat) => {
                                        chat.context.get_parts().iter().map(|context_part| {
                                            if context_part.in_visible_position() {
                                                html!(
                                                    <div>
                                                    <h3>{if context_part.is_user() {proxima_state.username.clone() + " : "} else {"Proxima : ".to_string()}}</h3>
                                                    <div> {context_part.data_to_text().iter().map(|string| {VNode::from_html_unchecked(AttrValue::from(to_html(&string)))}).collect::<Vec<Html>>() /*context_part.data_to_text().iter().map(|string| {string.split('\n').collect::<Vec<&str>>()}).flatten().map(|string| {html!(string)}).intersperse(html!(<br/>)).collect::<Vec<Html>>()*/}</div>
                                                    </div>
                                                )
                                            }
                                            else {
                                                html!()
                                            }
                                        }).collect()
                                    },
                                    None => Vec::new()
                                }
                            }
                            {
                                match chosen_chat_by_id{
                                    Some(chat) => if chat.last_response_is_user() {
                                        html!(<h2>{"Waiting on the AI to respond..."}</h2>)
                                    }
                                    else {
                                        html!()
                                    },
                                    None => html!()
                                }
                            }
                            </div>
                        </div>

                        <div class="label-input-combo bottom-bar most-horizontal-space-no-flex third-level standard-padding-margin-corners">
                            <textarea placeholder="Have a prompt ?" ref={prompt_node_ref} class="standard-padding-margin-corners"/>
                            <select class="standard-padding-margin-corners" ref={cc_select_ref} onchange={cc_select_callback}>
                                <option value="NO CHAT CONFIG WHATSOEVER (please do not use this magic name for a real chat config)">{"None"}</option>
                                {config_htmls}
                            </select>
                            <button class="mainapp-button standard-padding-margin-corners" onclick={prompt_send_callback}>{"Send"}</button>
                            
                        </div>
                    </div>
                    
                </div>
            }
        }
        /* Tags */ 2 => {

            let tag_htmls = client_database.tags.get_tags().iter().enumerate().map(|(id, tag)| {
                let callback = {

                    let second_db = client_database.clone();
                    let chosen_tag_clone = chosen_tag.clone();
                    let chosen_parent_tag_clone = chosen_parent_tag.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        chosen_tag_clone.set(Some(id_clone));
                        chosen_parent_tag_clone.set(second_db.tags.get_tags()[id_clone].get_parent())
                    })
                };
                if !client_database.access_modes.get_modes()[*chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else if chosen_tag.is_some() && id == chosen_tag.unwrap() {
                    html!(

                        <div><button onclick={callback} class="chat-option chosen-chat">{tag.get_name().clone()}</button></div>
                    )
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();

            let parent_tag_htmls = client_database.tags.get_tags().iter().enumerate().map(|(id, tag)| {
                let callback = {
                    let chosen_tag_clone = chosen_parent_tag.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        chosen_tag_clone.set(Some(id_clone));
                    })
                };
                if (chosen_tag.is_some() && id == chosen_tag.unwrap()) || !client_database.access_modes.get_modes()[*chosen_access_mode].get_tags().contains(&id)  {
                    html!()
                }
                else if chosen_parent_tag.is_some() && id == chosen_parent_tag.unwrap() {
                    html!(

                        <div><button onclick={callback} class="chat-option chosen-chat">{tag.get_name().clone()}</button></div>
                    )
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();
            
            let new_tag_callback = {
                let chosen_tag = chosen_tag.clone();
                let chosen_parent_tag = chosen_parent_tag.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    chosen_tag.set(None);
                    chosen_parent_tag.set(None);
                })
            };

            let chosen_tag_by_id = client_database.tags.get_tags().get((chosen_tag.unwrap_or(1000000)));

            let tag_update_callback = {
                let chosen_tag = chosen_tag.clone();
                let chosen_parent_tag = chosen_parent_tag.clone();
                let client_db = client_database.clone();
                let tag_name_ref = tag_name_ref.clone();
                let tag_desc_ref = tag_desc_ref.clone();
                let chosen_access_mode = chosen_access_mode.clone();
                let proxima_state = proxima_state.clone();
                let user_cursors = user_cursors.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let mut db_copy = (*client_db).clone();
                    
                    let tag_name = tag_name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    let tag_desc = tag_desc_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    let description = Description::new(tag_desc);
                    
                    match *chosen_tag {
                        Some(tag_id) => {
                            let mut tag = db_copy.tags.get_tags()[tag_id].clone();
                            tag.name = tag_name;
                            tag.desc = description;
                            tag.parent = *chosen_parent_tag;
                            db_copy.tags.update_tag(tag.clone());
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Tag(tag)) };
                                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => (),
                                    Err(()) => ()
                                }
                            });
                            client_db.set(db_copy);
                            
                        },
                        None => {
                            let tag_id = db_copy.tags.add_tag(NewTag::new(tag_name, description, *chosen_parent_tag));


                            let am_id = *chosen_access_mode;
                            db_copy.access_modes.associate_tag_to_mode(am_id, tag_id);
                            let new_access_mode = db_copy.access_modes.get_modes()[am_id].clone();

                            let tag = db_copy.tags.get_tags()[tag_id].clone();

                            let proxima_state = proxima_state.clone();
                            let user_cursors = user_cursors.clone();
                            let client_db = client_db.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Add(DatabaseItem::Tag(tag.clone())) };
                                let (new_cursors, new_id) = match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => handle_add(
                                        &mut db_copy,
                                        DatabaseItemID::Tag(tag_id),
                                        DatabaseItem::Tag(tag),
                                        response.reply,
                                        (*user_cursors).clone(),
                                        async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                    ).await,
                                    Err(()) => ((*user_cursors).clone(), DatabaseItemID::Tag(tag_id))
                                };
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::AccessMode(new_access_mode)) };
                                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => (),
                                    Err(()) => ()
                                }
                                user_cursors.set(new_cursors);
                                client_db.set(db_copy);
                            });
                        },
                    }
                })
            };

            

            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">
                        <h1>{"Tags"}</h1>
                        <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_tag_callback}>{"New tag"}</button>
                        <hr/>

                        <div class="list-holder">
                            {
                                if client_database.tags.get_tags().len() > 0 {
                                    tag_htmls
                                }
                                else {
                                    html!({"No tags yet !"})
                                }
                            }
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space chat-tab-not-sidebar">
                        <h1> {
                            match chosen_tag_by_id {
                                Some(tag) => {format!("Currently modifying : {}", tag.get_name())},
                                None => "Creating a tag".to_string()    
                            }
                        }
                        </h1>
                        <div class="multi-input-container second-level standard-padding-margin-corners">
                            
                            <div class="label-input-combo third-level standard-padding-margin-corners">
                                <p>{"Tag name (obligatory) : "}</p>
                                <input class="standard-padding-margin-corners" placeholder="Enter a tag name here..." ref={tag_name_ref}/>
                                
                            </div>
                            <div class="label-input-combo third-level standard-padding-margin-corners">
                                <p>{"Tag description (optional) : "}</p>
                                <input class="standard-padding-margin-corners" placeholder="Enter tag descirption here... keep it simple !" ref={tag_desc_ref}/>
                            </div>
                            <div class="chat-tab-current-chat third-level standard-padding-margin-corners">
                                <h2>
                                    {
                                        match *chosen_parent_tag {
                                            Some(tag_id) => format!("Currently chosen parent tag : {}", client_database.tags.get_tags()[tag_id].get_name().clone()),
                                            None => "Does this tag have a parent ?".to_string()
                                        }
                                    }
                                </h2>
                                <div class="list-holder">
                                    {
                                        if client_database.tags.get_tags().len() > 0 {
                                            parent_tag_htmls
                                        }
                                        else {
                                            html!({"No other tags yet !"})
                                        }
                                    }
                                </div>
                            </div>
                            
                            
                        </div>

                        <div class="label-input-combo bottom-bar most-horizontal-space-no-flex">

                            <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={tag_update_callback}>
                            {
                                match chosen_tag_by_id {
                                    Some(tag) => {"Save modifications".to_string()},
                                    None => "Create tag".to_string()    
                                }
                            }
                            </button>
                        </div>
                    </div>
                </div>
            }
        },
        /* Access Modes */ 3 => {
            let chosen_am = chosen_am_for_creation.clone();
            let access_mode_htmls = client_database.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
                let callback = {

                    let second_db = client_database.clone();
                    let chosen_am_clone = chosen_am.clone();
                    let chosen_tags = chosen_tags_for_am.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        chosen_am_clone.set(Some(id_clone));
                        chosen_tags.set(second_db.access_modes.get_modes()[id_clone].get_tags().clone());
                    })
                };
                if chosen_am.is_some() && id == chosen_am.unwrap() {
                    html!(

                        <div><button onclick={callback} class="chat-option chosen-chat">{access_mode.get_name().clone()}</button></div>
                    )
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{access_mode.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();

            let tag_htmls = client_database.tags.get_tags().iter().enumerate().map(|(id, tag)| {
                let chosen_access_mode = chosen_access_mode.clone();
                let callback = {

                    let second_db = client_database.clone();
                    let chosen_tags = chosen_tags_for_am.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        let mut list_clone = (*chosen_tags).clone();
                        list_clone.insert(id_clone);
                        chosen_tags.set(list_clone);
                    })
                };
                if chosen_tags_for_am.contains(&id) || !client_database.access_modes.get_modes()[*chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();

            let chosen_tag_htmls = client_database.tags.get_tags().iter().enumerate().map(|(id, tag)| {
                let chosen_access_mode = chosen_access_mode.clone();
                let callback = {
                    let second_db = client_database.clone();
                    let chosen_tags = chosen_tags_for_am.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        let mut list_clone = (*chosen_tags).clone();
                        list_clone.remove(&id);
                        chosen_tags.set(list_clone);
                    })
                };
                if !chosen_tags_for_am.contains(&id) || !client_database.access_modes.get_modes()[*chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();
            
            let new_tag_callback = {
                let chosen_am = chosen_am_for_creation.clone();
                let chosen_tags = chosen_tags_for_am.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    chosen_am.set(None);
                    chosen_tags.set(HashSet::new());
                })
            };

            let chosen_am_by_id = client_database.access_modes.get_modes().get((chosen_am_for_creation.unwrap_or(1000000)));

            let am_update_callback = {
                let chosen_am = chosen_am_for_creation.clone();
                let chosen_tags = chosen_tags_for_am.clone();
                let client_db = client_database.clone();
                let proxima_state = proxima_state.clone();
                let am_name_ref = am_name_ref.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let mut db_copy = (*client_db).clone();
                    
                    let am_name = am_name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    match *chosen_am {
                        Some(am_id) => {
                            let mut am = db_copy.access_modes.get_modes()[am_id].clone();
                            am.name = am_name;
                            am.tags = (*chosen_tags).clone();
                            db_copy.access_modes.update_mode(am.clone());
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::AccessMode(am)) };
                                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => (),
                                    Err(()) => ()
                                }
                            });
                            client_db.set(db_copy);
                        },
                        None => {
                            let am = AccessMode::new(0, (*chosen_tags).clone(), am_name);
                            let id = db_copy.access_modes.add_mode(am.clone());
                            let proxima_state = proxima_state.clone();
                            let user_cursors = user_cursors.clone();
                            let client_db = client_db.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Add(DatabaseItem::AccessMode(am.clone())) };
                                let (new_cursors, new_id) = match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => handle_add(
                                        &mut db_copy,
                                        DatabaseItemID::AccessMode(id),
                                        DatabaseItem::AccessMode(am),
                                        response.reply,
                                        (*user_cursors).clone(),
                                        async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                    ).await,
                                    Err(()) => ((*user_cursors).clone(), DatabaseItemID::AccessMode(id))
                                };
                                user_cursors.set(new_cursors);
                                client_db.set(db_copy);
                            });
                        },
                    }
                    chosen_am.set(None);
                })
            };

            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">
                        <h1>{"Access Modes"}</h1>
                        <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_tag_callback}>{"New Access Mode"}</button>
                        <hr/>

                        <div class="list-holder">
                            {
                                if client_database.access_modes.get_modes().len() > 0 {
                                    access_mode_htmls
                                }
                                else {
                                    html!({"Something is very wrong, you are supposed to have AT LEAST 1 Access mode"})
                                }
                            }
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space chat-tab-not-sidebar">
                        <h1> {
                            match chosen_am_by_id {
                                Some(tag) => {format!("Currently modifying : {}", tag.get_name())},
                                None => "Creating an Access Mode".to_string()    
                            }
                        }
                        </h1>
                        <div class="multi-input-container second-level standard-padding-margin-corners">
                            <div class="label-input-combo standard-padding-margin-corners third-level">
                                <p>{"Access mode name (obligatory) : "}</p>
                                <input class="standard-padding-margin-corners" placeholder="Enter an access mode name here..." ref={am_name_ref}/>
                                
                            </div>
                            <div class="chat-tab-current-chat third-level standard-padding-margin-corners">
                                <h2>
                                    {"What tags are associated with this access mode ?"}
                                </h2>
                                <table>
                                    <tr>
                                        <th>
                                            {"Available tags"}
                                        </th>
                                        <th>

                                            {"Chosen tags"}
                                        </th>
                                    </tr>
                                    <tr>
                                        <td>
                                        <div class="list-holder">
                                            {
                                                if client_database.tags.get_tags().len() > 0 {
                                                    tag_htmls
                                                }
                                                else {
                                                    html!({"No tags left to add"})
                                                }
                                            }
                                        </div>
                                        </td>
                                        <td>
                                        <div class="list-holder">
                                            {
                                                if client_database.tags.get_tags().len() > 0 {
                                                    chosen_tag_htmls
                                                }
                                                else {
                                                    html!({"Add tags to this access mode :)"})
                                                }
                                            }
                                        </div>
                                        </td>
                                    </tr>
                                </table>
                            </div>
                            
                        </div>

                        <div class="label-input-combo bottom-bar most-horizontal-space-no-flex">
                            <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={am_update_callback}>
                            {
                                match chosen_am_by_id {
                                    Some(tag) => {"Save modifications".to_string()},
                                    None => "Create Access Mode".to_string()    
                                }
                            }
                            </button>
                        </div>
                    </div>
                </div>
            }
        },
        /*Files*/ 4 => html!(),
        /*Chat Settings*/ 5 => {
            /* Sidebar with list picker from all possible settings, add button, and then the list of all current settings with removal buttons if necessary */
            /* The main area is for configuration of a single setting */

            let new_cc_callback = {
                let user_cursors = user_cursors.clone();
                let name_ref = cc_name_ref.clone();
                let client_db = client_database.clone();
                let proxima_state = proxima_state.clone();
                let chosen_access_mode = chosen_access_mode.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let mut cursors_clone = (*user_cursors).clone();
                    
                    let mut db_copy = (*client_db).clone();
                    let proxima_state = proxima_state.clone();
                    let name = name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    if !name.trim().is_empty() {
                        let mut config = ChatConfiguration::new(name, vec![]);
                        config.access_modes.insert((*chosen_access_mode).clone());
                        let id = db_copy.configs.add_config(config.clone());
                        let proxima_state = proxima_state.clone();
                        let user_cursors = user_cursors.clone();
                        let client_db = client_db.clone();
                        spawn_local(async move {
                            let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Add(DatabaseItem::ChatConfig(config.clone())) };
                            let (new_cursors, new_id) = match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                Ok(response) => handle_add(
                                    &mut db_copy,
                                    DatabaseItemID::ChatConfiguration(id),
                                    DatabaseItem::ChatConfig(config),
                                    response.reply,
                                    (*user_cursors).clone(),
                                    async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                ).await,
                                Err(()) => ((*user_cursors).clone(), DatabaseItemID::ChatConfiguration(id))
                            };
                            user_cursors.set(new_cursors);
                            client_db.set(db_copy);
                        });
                    }
                    
                })
            };
            let ccs_htmls = client_database.configs.get_configs().iter().enumerate().map(|(id, config)| {
                let user_cursors = user_cursors.clone();
                let chosen_access_mode = chosen_access_mode.clone();
                let callback = {

                    let setting_in_modification = setting_in_modification.clone();
                    let second_db = client_database.clone();
                    let user_cursors = user_cursors.clone();
                    let chosen_tags = chosen_tags_for_am.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        let mut cursors_clone = (*user_cursors).clone();
                        cursors_clone.config_for_modification = Some(id_clone);
                        cursors_clone.chosen_setting = None;
                        setting_in_modification.set(None);
                        
                        user_cursors.set(cursors_clone);
                    })
                };
                if !config.access_modes.contains(&*chosen_access_mode) {
                    html!()
                }
                else if user_cursors.config_for_modification.is_some() && id == user_cursors.config_for_modification.unwrap() {
                    html!(

                        <div><button onclick={callback} class="chat-option chosen-chat">{config.name.clone()}</button></div>
                    )
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{config.name.clone()}</button></div>
                    )
                }
            }).collect::<Html>();

            let current_cursors = (*user_cursors).clone();

                let setting_in_modification = setting_in_modification.clone();
            let chosen_cc_settings_htmls = match current_cursors.config_for_modification {
                Some(config_id) => {
                    let client_db = client_database.clone();
                    client_db.configs.get_configs()[config_id].raw_settings.iter().enumerate().map(|(id, setting)| {

                        let setting_in_modification = setting_in_modification.clone();
                        let user_cursors = user_cursors.clone();
                        let setting_c = setting.clone();
                        let callback = {

                            let second_db = client_database.clone();
                            let user_cursors = user_cursors.clone();
                            let chosen_tags = chosen_tags_for_am.clone();
                            let id_clone = id;
                            let setting_clone = setting_c.clone();
                            Callback::from(move |mouse_evt:MouseEvent| {
                                let mut cursors_clone = (*user_cursors).clone();
                                cursors_clone.chosen_setting = Some(id_clone);
                                setting_in_modification.set(Some(setting_clone.clone()));
                                user_cursors.set(cursors_clone);
                            })
                        };
                        if user_cursors.chosen_setting.is_some() && id == user_cursors.chosen_setting.unwrap() {
                            html!(

                                <div><button onclick={callback} class="chat-option chosen-chat">{setting.get_title()}</button></div>
                            )
                        }
                        else {
                            html!(
                                <div><button onclick={callback} class="chat-option">{setting.get_title()}</button></div>
                            )
                        }
                    }).collect::<Html>()
                },
                None => if client_database.configs.get_configs().len() > 0 {
                    html!({"Please pick a configuration to modify"})
                }
                else {
                    html!({"Please create a configuration to add settings to it"})
                },
            };

            let select_settings_callback = {
                let chosen_access_mode = chosen_access_mode.clone();
                let select_node = cc_setting_ref.clone();
                let db_clone = second_db_here.clone();
                let user_cursors = user_cursors.clone();
                let chosen_chat = chosen_chat.clone();
                let chosen_tag = chosen_tag.clone();
                let chosen_parent_tag = chosen_parent_tag.clone();
                let setting_in_modification = setting_in_modification.clone();
                Callback::from(move |mouse_evt:Event| {
                    let mut db_copy = (*db_clone).clone();
                    let selected_setting_string = select_node.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    setting_in_modification.set(match selected_setting_string.trim() {
                        "Temperature" => Some(ChatSetting::Temperature(700)),
                        "System prompt" => Some(ChatSetting::SystemPrompt(ContextPart::new(vec![], ContextPosition::System))),
                        "Initial Pre-prompt" => Some(ChatSetting::PrePrompt(ContextPart::new(vec![], ContextPosition::User))),
                        "Pre-prompt at chat end" => Some(ChatSetting::PrePromptBeforeLatest(ContextPart::new(vec![], ContextPosition::User))),
                        "Max context length" => Some(ChatSetting::MaxContextLength(10000)),
                        "Max response length" => Some(ChatSetting::ResponseTokenLimit(10000)),
                        "Tool" => Some(ChatSetting::Tool(ProximaTool::Calculator)),
                        _ => None
                    });
                    let mut cursors_copy = (*user_cursors).clone();
                    cursors_copy.chosen_setting = None;
                    user_cursors.set(cursors_copy);
                })
            };

            let add_update_settings_callback = {
                let user_cursors = user_cursors.clone();
                let cc_setting_value_ref = cc_setting_value_ref.clone();
                let client_db = client_database.clone();
                let proxima_state = proxima_state.clone();
                let setting_in_modification = setting_in_modification.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let mut cursors_clone = (*user_cursors).clone();
                    let cc_setting_value_ref = cc_setting_value_ref.clone();
                    let mut db_copy = (*client_db).clone();
                    let proxima_state = proxima_state.clone();
                    let new_setting = match (*setting_in_modification).clone() {
                        Some(setting) => match setting {
                            ChatSetting::PrePrompt(prompt) => ChatSetting::PrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::User)),
                            ChatSetting::PrePromptBeforeLatest(prompt) => ChatSetting::PrePromptBeforeLatest(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::User)),
                            ChatSetting::Temperature(temp) => ChatSetting::Temperature(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::SystemPrompt(prompt) => ChatSetting::SystemPrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::System)),
                            ChatSetting::Tool(tool) => ChatSetting::Tool(match cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                "Calculator" => ProximaTool::Calculator,
                                "Local Memory" => ProximaTool::LocalMemory,
                                _ => panic!("Impossible")
                            }),
                            ChatSetting::MaxContextLength(length) => ChatSetting::MaxContextLength(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::ResponseTokenLimit(limit) => ChatSetting::ResponseTokenLimit(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::AccessMode(access_mode) => {
                                let access_mode_name = cc_setting_value_ref.cast::<web_sys::HtmlInputElement>()
                                .unwrap()
                                .value();
                                match db_copy.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
                                    access_mode.get_name() == &access_mode_name
                                }) {
                                    Some((i, access_mode)) => {
                                        ChatSetting::AccessMode(i)
                                    },
                                    None => panic!("What")
                                }
                            }
                        },
                        None => {
                            panic!("Impossible, the button should only exist if this is Some(...)")
                        }
                    };
                    let setting_in_modification = setting_in_modification.clone();
                    setting_in_modification.set(Some(new_setting.clone()));
                    match cursors_clone.config_for_modification {
                        Some(config_id) => {
                            let proxima_state = proxima_state.clone();
                            let user_cursors = user_cursors.clone();
                            let client_db = client_db.clone();
                            spawn_local(async move {
                                let mut config = db_copy.configs.get_configs()[config_id].clone();
                                let mut cursors_clone = (*user_cursors).clone();

                                match cursors_clone.chosen_setting {
                                    Some(setting) => {
                                        config.raw_settings[setting] = new_setting.clone();
                                    },
                                    None => {
                                        cursors_clone.chosen_setting = Some(config.raw_settings.len());
                                        config.raw_settings.push((new_setting.clone()));
                                    },
                                }
                                config.tools = Tools::try_from_settings(config.raw_settings.clone());
                                config.last_updated = Utc::now();
                                db_copy.configs.update_config(config.clone());
                                let proxima_state = proxima_state.clone();
                                spawn_local(async move {
                                    let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::ChatConfig(config)) };
                                    match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                        Ok(response) => (),
                                        Err(()) => ()
                                    }
                                });
                                client_db.set(db_copy);
                                user_cursors.set(cursors_clone);
                            });
                        },
                        None => ()
                    }
                        
                    
                })
            };
            let setting_config = {
                match (*setting_in_modification).clone() {
                    Some(setting) => match setting {
                        ChatSetting::PrePrompt(prompt) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"Pre-prompt value : "}</p>
                                <textarea class="standard-padding-margin-corners" placeholder="Pre-prompt here..." id="pre_pre_prompt" ref={cc_setting_value_ref}/>
                            </div>
                        ),
                        ChatSetting::PrePromptBeforeLatest(prompt) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"Pre-prompt added after all of your prompts : "}</p>
                                <textarea class="standard-padding-margin-corners" placeholder="Pre-prompt here..." id="pre_prompt" ref={cc_setting_value_ref}/>
                            </div>
                        ),
                        ChatSetting::Temperature(temp) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"Temperature : "}</p>
                                <input class="standard-padding-margin-corners" type="range" id="temp_slider" min="0" max="1000" step="1" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::SystemPrompt(prompt) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"System prompt part : "}</p>
                                <textarea class="standard-padding-margin-corners" placeholder="System prompt here..." id="system_prompt" ref={cc_setting_value_ref}/>
                            </div>
                        ),
                        ChatSetting::Tool(tool) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"System prompt part : "}</p>
                                <select class="standard-padding-margin-corners" id="tool_select" ref={cc_setting_value_ref}>
                                    <option value={"Calculator"}>{"Calculator"}</option>
                                    <option value={"Local Memory"}>{"Local Memory"}</option>
                                </select>
                            </div>
                        ),
                        ChatSetting::MaxContextLength(length) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"Max context length (in tokens) : "}</p>
                                <input class="standard-padding-margin-corners" type="range" id="context_slider" min="512" max="32000" step="256" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::ResponseTokenLimit(limit) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{"Max response length (in tokens) : "}</p>
                                <input class="standard-padding-margin-corners" type="range" id="response_slider" min="512" max="32000" step="256" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::AccessMode(access_mode) => {
                            let access_modes_htmls:Vec<Html> = second_db_here.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
                                html!(
                                    <option value={access_mode.get_name().clone()}>{access_mode.get_name().clone()}</option>
                                )
                            }).collect();

                            html!(
                                <div class="label-input-combo second-level standard-padding-margin-corners">
                                    <p>{"System prompt part : "}</p>
                                    <select class="standard-padding-margin-corners" id="access_select" ref={cc_setting_value_ref}>
                                        {access_modes_htmls}
                                    </select>
                                </div>
                            )
                        }
                    },
                    None => {
                        html!({"Choose a setting to add or modify to configure its attribute(s)"})
                    }
                }
            };
            html!(
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level at-most-a-sixth-width">
                        <h1>{"Chat configurations"}</h1>
                        <input class="standard-padding-margin-corners most-horizontal-space-no-flex" placeholder="Chat config name..." ref={cc_name_ref}/>
                        <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={new_cc_callback}>{"New Chat Configuration"}</button>
                        <hr/>

                        <div class="list-holder most-horizontal-space-no-flex">
                            {
                                if client_database.configs.get_configs().len() > 0 {
                                    ccs_htmls
                                }
                                else {
                                    html!({"To create a chat configuration, please give it a non-empty name and click \"New Configuration\" above"})
                                }
                            }
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level at-most-a-sixth-width">
                        <h1>{"Configuration settings"}</h1>
                        <h2>
                        {
                            match user_cursors.config_for_modification {
                                Some(config) => html!({format!("For : {}", client_database.configs.get_configs()[config].name.clone())}),
                                None => html!()
                            }
                        }
                        </h2>
                        <select class="most-horizontal-space-no-flex standard-padding-margin-corners" ref={cc_setting_ref} onchange={select_settings_callback}>
                            <option value={"Temperature"}>{"Temperature"}</option>
                            <option value={"System prompt"}>{"System prompt"}</option>
                            <option value={"Initial Pre-prompt"}>{"Initial Pre-prompt"}</option>
                            <option value={"Pre-prompt at chat end"}>{"Pre-prompt at chat end"}</option>
                            <option value={"Max context length"}>{"Max context length"}</option>
                            <option value={"Max response length"}>{"Max response length"}</option>
                            <option value={"Tool"}>{"Tool"}</option>
                        </select>
                        <hr/>

                        <div class="list-holder most-horizontal-space-no-flex">
                            {
                                chosen_cc_settings_htmls
                            }
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space chat-tab-not-sidebar">
                        <h1> 
                        {"Modifying settings here"}
                        </h1>
                        <div class="multi-input-container standard-padding-margin-corners">
                            {setting_config}
                            
                        </div>

                        <div class="label-input-combo bottom-bar most-horizontal-space-no-flex">
                            {
                                match (*setting_in_modification).clone() {
                                    Some(setting) => html!(<button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={add_update_settings_callback}>
                                        {
                                            match current_cursors.chosen_setting {
                                                Some(tag) => {"Update setting".to_string()},
                                                None => "Add setting".to_string()    
                                            }
                                        }
                                        </button>
                                    ),
                                    None => html!({"Choose a setting to add or modify for more fun !!!"})
                                }
                            }
                            
                        </div>
                    </div>
                </div>
            )
        },
        _ => html!({"Something is very wrong"})
    };
    let access_mode_select = use_node_ref();
    let access_mode_callback = {
        let chosen_access_mode = chosen_access_mode.clone();
        let select_node = access_mode_select.clone();
        let db_clone = second_db_here.clone();
        let chosen_chat = chosen_chat.clone();
        let chosen_tag = chosen_tag.clone();
        let chosen_parent_tag = chosen_parent_tag.clone();
        Callback::from(move |mouse_evt:Event| {
            let mut db_copy = (*db_clone).clone();
            let access_mode_name = select_node.cast::<web_sys::HtmlInputElement>()
            .unwrap()
            .value();
            match db_copy.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
                access_mode.get_name() == &access_mode_name
            }) {
                Some((id, access_mode)) => {
                    let second_db_clone = db_clone.clone();
                    let access_mode_name_clone = access_mode_name.clone();
                    if *chosen_access_mode != id {
                        chosen_chat.set(None);
                        chosen_tag.set(None);
                        chosen_parent_tag.set(None);
                    }
                    chosen_access_mode.set(id);
                    spawn_local(async move {
                        

                        let args = serde_wasm_bindgen::to_value(&PrintArgs {value:access_mode_name_clone}).unwrap();
                        invoke("print_to_console", args).await;

                    });
                },
                None => ()
            }
            
        })
    };
    let access_modes_htmls:Vec<Html> = second_db_here.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
        html!(
            <option value={access_mode.get_name().clone()}>{access_mode.get_name().clone()}</option>
        )
    }).collect();
    html! {
        <main class="container">
            <div id="top-bar">
                <div id="menu-bar">
                    <button class="menu-item" id={values[0].clone()} onclick={tab_picker_callbacks[0].clone()}>{"LOGO HERE"}</button>
                    <button class="menu-item" id={values[1].clone()} onclick={tab_picker_callbacks[1].clone()}>{"Chat"}</button>
                    <button class="menu-item" id={values[2].clone()} onclick={tab_picker_callbacks[2].clone()}>{"Tags"}</button>
                    <button class="menu-item" id={values[3].clone()} onclick={tab_picker_callbacks[3].clone()}>{"Access Modes"}</button>
                    <button class="menu-item" id={values[4].clone()} onclick={tab_picker_callbacks[4].clone()}>{"Files"}</button>
                    <button class="menu-item" id={values[5].clone()} onclick={tab_picker_callbacks[5].clone()}>{"Settings"}</button>
                    <select class="menu-item" ref={access_mode_select} onchange={access_mode_callback}>
                        {access_modes_htmls}
                    </select>
                </div>
            </div>
            <div class="interactive-part">
                {current_app}
            </div>
        </main>
    }
}

impl Default for BoolState {
    fn default() -> Self {
        Self { value: false }
    }
}

pub enum BoolAction {
    On,
    Off,
    Toggle
}

#[derive(PartialEq, Clone, Eq)]
pub struct BoolState {
    value:bool
}

impl Reducible for BoolState {
    type Action = BoolAction;
    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let next_val = match action {
            BoolAction::On => true,
            BoolAction::Off => false,
            BoolAction::Toggle => !self.value,
        };
        Self { value:next_val }.into()
    }
}

#[derive(PartialEq, Clone, Eq)]
pub struct ProximaState {
    initialized:bool,
    loaded:bool,
    username:String,
    auth_token:String,
    chat_url:String,
    device_id:DeviceID,
    start_db:Option<ProxDatabase>
}

pub enum ProximaStateAction {
    ChangeInit(bool),
    ChangeLoaded(bool),
    ChangeUsername(String),
    ChangeAuthToken(String),
    ChangeChatURL(String),
    ChangeDeviceID(DeviceID),
    ChangeStartDB(Option<ProxDatabase>)
}

impl Reducible for ProximaState {
    type Action = ProximaStateAction;
    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let next_val = match action {
            ProximaStateAction::ChangeInit(new_init) => Self {initialized:new_init, loaded:self.loaded, username:self.username.clone(), auth_token:self.auth_token.clone(), chat_url:self.chat_url.clone(), device_id:self.device_id.clone(), start_db:self.start_db.clone()},
            ProximaStateAction::ChangeLoaded(new_load) => Self {initialized:self.initialized, loaded:new_load, username:self.username.clone(), auth_token:self.auth_token.clone(), chat_url:self.chat_url.clone(), device_id:self.device_id.clone(), start_db:self.start_db.clone()},
            ProximaStateAction::ChangeUsername(new_username) => Self {initialized:self.initialized, loaded:self.loaded, username:new_username, auth_token:self.auth_token.clone(), chat_url:self.chat_url.clone(), device_id:self.device_id.clone(), start_db:self.start_db.clone()},
            ProximaStateAction::ChangeAuthToken(new_auth_token) => Self {initialized:self.initialized, loaded:self.loaded, username:self.username.clone(), auth_token:new_auth_token, chat_url:self.chat_url.clone(), device_id:self.device_id.clone(), start_db:self.start_db.clone()},
            ProximaStateAction::ChangeChatURL(new_chat_url) => Self {initialized:self.initialized, loaded:self.loaded, username:self.username.clone(), auth_token:self.auth_token.clone(), chat_url:new_chat_url, device_id:self.device_id.clone(), start_db:self.start_db.clone()},
            ProximaStateAction::ChangeDeviceID(new_device_id) => Self {initialized:self.initialized, loaded:self.loaded, username:self.username.clone(), auth_token:self.auth_token.clone(), chat_url:self.chat_url.clone(), device_id:new_device_id, start_db:self.start_db.clone()},
            ProximaStateAction::ChangeStartDB(new_start_db) => Self {initialized:self.initialized, loaded:self.loaded, username:self.username.clone(), auth_token:self.auth_token.clone(), chat_url:self.chat_url.clone(), device_id:self.device_id, start_db:new_start_db},
        };
        next_val.into()
    }
}

impl Default for ProximaState {
    fn default() -> Self {
        Self { initialized: false, loaded: false, username: String::from("No username defined"), auth_token: String::from("INVALID AUTH TOKEN"), chat_url:String::from("INVALID ADDRESS"), device_id:0,start_db:None }
    }
}

#[function_component(App)]
pub fn app() -> Html {
    let proxima_state = use_reducer_eq(ProximaState::default);
    let waitfor_value = if !(*proxima_state).loaded && (*proxima_state).initialized {
        String::from("loading")
    }
    else {
        
        String::from("NOPE")
    };
    {
        let loaded_clone = proxima_state.clone();
        use_effect(move || {
            // Make a call to DOM API after component is rendered
            spawn_local(async move {
                
                let args = serde_wasm_bindgen::to_value(&PrintArgs {value:waitfor_value}).unwrap();
                invoke("print_to_console", args).await;
                loaded_clone.dispatch(ProximaStateAction::ChangeLoaded(true));
            });
        });
    }
    

    if !(*proxima_state).initialized {
        
        return html!{
            <ContextProvider<UseReducerHandle<ProximaState>> context={proxima_state.clone()}>
            // Every child here and their children will have access to this context.
                <Initialize />
            </ContextProvider<UseReducerHandle<ProximaState>>>
            
        }
    }
    else if !(*proxima_state).loaded {
        return html!{
            <ContextProvider<UseReducerHandle<ProximaState>> context={proxima_state.clone()}>
            // Every child here and their children will have access to this context.
                <Loading />
            </ContextProvider<UseReducerHandle<ProximaState>>>
        }
    }
    else {
        return html!(
            <ContextProvider<UseReducerHandle<ProximaState>> context={proxima_state.clone()}>
            <MainPage/>
            </ContextProvider<UseReducerHandle<ProximaState>>>)
    }

}
