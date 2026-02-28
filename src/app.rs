use std::{collections::{HashMap, HashSet}, path::PathBuf, thread, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use gloo_events::EventListener;
use reqwest::header::{HeaderMap, HeaderValue, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::{html::ChildrenProps, platform::pinned::mpsc::UnboundedSender, prelude::*, virtual_dom::VNode};
use gloo_utils::format::JsValueSerdeExt;
use proxima_backend::{ai_interaction::{endpoint_api::{EndpointRequestVariant, EndpointResponseVariant}, tools::{AgentToolData, ProximaTool, ProximaToolData, Tools}}, database::{DatabaseItem, DatabaseItemID, DatabaseReplyVariant, DatabaseRequestVariant, ProxDatabase, access_modes::AccessMode, chats::{Chat, ChatID, SessionType}, configuration::{ChatConfiguration, ChatSetting, RepeatPosition}, context::{ContextData, ContextPart, ContextPosition, WholeContext}, description::Description, devices::DeviceID, tags::{NewTag, Tag, TagID}}, web_payloads::{AIPayload, AIResponse, AuthPayload, AuthResponse, DBPayload, DBResponse}};
use yew::prelude::*;
use selectrs::yew::{Select, Group};
use markdown::to_html;
use web_sys::{EventTarget, HtmlElement};
use futures::StreamExt;

use crate::{db_sync::{UserCursors, apply_server_updates, get_delta_for_add, get_next_id_for_category, handle_add, handle_add_reducible}, notification_tab::NotificationTab};

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
    second:SecondArgument
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

pub async fn make_db_request(payload:DBPayload, backend_url:String) -> Result<DBResponse, ()> {
    let args = serde_wasm_bindgen::to_value(&HttpDBPostRequest {request:payload, url:backend_url + "/db"}).unwrap();

    let return_val = invoke("database_post_request", args).await;
    
    let value =
    return_val
    .into_serde::<DBResponse>();
    value.map_err(|error| {})
}


#[derive(Serialize, Deserialize)]
pub struct SecondArgument {
    url:String,
    chat_id:ChatID
}


pub async fn make_ai_request(payload:AIPayload, backend_url:String, chat_id:ChatID) -> Result<AIResponse, ()> {
    let args = serde_wasm_bindgen::to_value(&HttpAIPostRequest {request:payload, second:SecondArgument { url: backend_url + "/ai", chat_id }}).unwrap();

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

#[derive(PartialEq)]
pub struct DatabaseState {
    pub db:ProxDatabase,
    pub cursors:UserCursors,
    pub update_flipper:bool,
    pub token_streams:HashMap<ChatID, StreamingData>
}

#[derive(Clone, PartialEq)]
struct StreamingData {
    token_ids:Vec<(u64, DateTime<Utc>)>,
    all_ids:HashSet<u64>,
    last_update:DateTime<Utc>,
    last_part_pos:ContextPosition
}

impl Default for DatabaseState {
    fn default() -> Self {
        Self {
            db:ProxDatabase::new_just_data(String::from("a"), String::from("a")),
            cursors:UserCursors::zero(),
            update_flipper:false,
            token_streams:HashMap::with_capacity(16),
        }
    }
}

pub enum DatabaseAction {
    SetDB(ProxDatabase),
    ApplyUpdates(Vec<(DatabaseItemID, DatabaseItem)>),
    AddItem(Vec<(DatabaseItemID, DatabaseItem)>, DatabaseItemID, DatabaseItem),
    RemoveItem(DatabaseItemID),
    SetTab(usize),
    SetChat(Option<usize>),
    SetModifiedAM(Option<usize>),
    SetGlobalAM(usize),
    SetModifiedTag(Option<usize>),
    SetParentTag(Option<usize>),
    ChangeUsedChatConfig(Option<usize>),
    AddToTagsForAM(TagID),
    RemoveFromTagsForAM(TagID),
    SetTagsForAM(HashSet<TagID>),
    SetConfigSettingID(Option<usize>),
    SetCurrentSetting(Option<ChatSetting>),
    SetModifiedConfig(Option<usize>),
    AddPartToChat {
        chat_id:ChatID,
        token_id:u64,
        part:ContextPart
    },
    AddDataToLastPartOfChat {
        chat_id:ChatID,
        token_id:u64,
        data:ContextData
    }
}

impl Reducible for DatabaseState {
    type Action = DatabaseAction;
    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let mut database = self.db.clone();
        let mut cursors = self.cursors.clone();
        let mut update_flipper = self.update_flipper;
        let mut token_streams = self.token_streams.clone();
        let now = Utc::now();
        let mut to_remove = Vec::with_capacity(2);
        for (chat_id, stream) in &mut token_streams {
            if stream.last_update.signed_duration_since(now).abs() > TimeDelta::minutes(3) {
                to_remove.push(*chat_id);
            }
        }
        for rem in to_remove {
            token_streams.remove(&rem);
        }
        match action {
            DatabaseAction::SetDB(db) => database = db,
            DatabaseAction::ApplyUpdates(updates) => {cursors = apply_server_updates(&mut database, updates, cursors);},
            DatabaseAction::AddItem(delta, remote_id, item) => {
                let local_id = get_next_id_for_category(&database, &item);
                // idea : make the add action have 2 parts :
                // make the add request in an async scope, and get everything from the local id to the given id in an array
                // send the array of items between local and remote id as well as the new item as an action
                // rewrite handle_add_reducible to handle that gracefully
                let new_cursors = handle_add_reducible(
                    &mut database,
                    local_id,
                    remote_id,
                    item,
                    cursors.clone(),
                    delta
                );
                cursors = new_cursors;
            },
            DatabaseAction::RemoveItem(item_id) => {
                database.remove_request(item_id);
            }
            DatabaseAction::SetChat(chat) => cursors.chosen_chat = chat,
            DatabaseAction::SetTab(tab) => cursors.chosen_tab = tab,
            DatabaseAction::SetGlobalAM(am) => cursors.chosen_access_mode = am,
            DatabaseAction::SetModifiedAM(am) => cursors.access_mode_for_modification = am,
            DatabaseAction::SetModifiedTag(tag) => cursors.chosen_tag = tag,
            DatabaseAction::SetParentTag(par) => cursors.chosen_parent_tag = par,
            DatabaseAction::ChangeUsedChatConfig(config) => cursors.chosen_config = config,
            DatabaseAction::AddToTagsForAM(tag) => {cursors.chosen_access_mode_tags.insert(tag);},
            DatabaseAction::RemoveFromTagsForAM(tag) => {cursors.chosen_access_mode_tags.remove(&tag);},
            DatabaseAction::SetTagsForAM(tags) => cursors.chosen_access_mode_tags = tags,
            DatabaseAction::SetConfigSettingID(id) => cursors.chosen_setting = id,
            DatabaseAction::SetCurrentSetting(setting) => cursors.setting_for_modification = setting,
            DatabaseAction::SetModifiedConfig(config) => cursors.config_for_modification = config,
            DatabaseAction::AddPartToChat { chat_id, token_id, part } => {
                update_flipper = !update_flipper;
                database.chats.get_chats_mut().get_mut(&chat_id).map(|chat| {
                    match token_streams.get_mut(&chat_id) {
                        Some(stream) => {
                            if now.signed_duration_since(stream.last_update).abs() > TimeDelta::seconds(1) || stream.last_part_pos != part.get_position().clone() {
                                token_streams.insert(chat_id, StreamingData { token_ids: vec![(token_id, Utc::now())], all_ids:HashSet::from([token_id]), last_update:now, last_part_pos:part.get_position().clone() });
                                chat.context.add_part(part);
                            }
                        },
                        None => {
                            token_streams.insert(chat_id, StreamingData { token_ids: vec![(token_id, Utc::now())], all_ids:HashSet::from([token_id]), last_update:Utc::now(), last_part_pos:part.get_position().clone() });
                            chat.context.add_part(part);
                        }
                    }
                });
            },
            DatabaseAction::AddDataToLastPartOfChat {
                chat_id,
                token_id,
                data
            } => {

                update_flipper = !update_flipper;
                database.chats.get_chats_mut().get_mut(&chat_id).map(|chat| {

                    match token_streams.get_mut(&chat_id) {
                        Some(stream) => {
                            if !stream.all_ids.contains(&token_id) {
                                let last_part = chat.context.get_parts_mut().last_mut().unwrap();
                                match last_part.get_data_mut().last_mut().unwrap() {
                                    ContextData::Text(text) => {
                                        match data {
                                            ContextData::Text(new_text) => *text += &new_text,
                                            ContextData::Media(image) => last_part.add_data(ContextData::Media(image)),
                                        }
                                    },
                                    ContextData::Media(image) => {
                                        last_part.add_data(data);
                                    }
                                }
                                stream.all_ids.insert(token_id);
                                stream.last_update = Utc::now();
                                stream.token_ids.push((token_id, stream.last_update.clone()));
                            }
                        },
                        None => ()
                    }
                });
            }

        }
        DatabaseState{db:database, cursors, update_flipper, token_streams}.into()
    }
}



#[function_component(MainPage)]
pub fn app_page() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_reducer_eq(DatabaseState::default);
    let got_start = use_state_eq(|| false);
    {
        let proxima_state = proxima_state.clone();
        let db_state = db_state.clone();
        let got_start = got_start.clone();
        use_effect(move || {
            if !*got_start.clone() {
                let db_clone = proxima_state.start_db.clone();
                let db_clone_is_ok = db_clone.is_some();
                if db_clone_is_ok {
                    db_state.dispatch(DatabaseAction::SetDB(proxima_state.start_db.clone().unwrap()));
                    got_start.set(true);
                }
                
            }
            
        });
    }

    let event_div_node_ref = use_node_ref();

    let second_db = db_state.clone();

    use_effect_with(
        second_db.clone(),
        {
            let div_node_ref = event_div_node_ref.clone();
            let second_db = second_db.clone();
            let proxima_state = proxima_state.clone();
            move |_| {
                let mut custard_listener = None;

                let db_state = second_db.clone();
                let proxima_state = proxima_state.clone();
                if let Some(element) = div_node_ref.cast::<HtmlElement>() {
                    // Create your Callback as you normally would
                    let oncustard = Callback::from(move |e: Event| {
                        
                    });
                    spawn_local(async move {
                        let mut listener = tauri_sys::event::listen::<(EndpointResponseVariant, ChatID, u64)>("chat-token").await.unwrap();

                        let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("STARTED LISTENING")}).unwrap();
                        invoke("print_to_console", args).await;
                        let (mut listener, mut abort_handle) = futures::stream::abortable(listener);
                        if let Some(raw_event) = listener.next().await {

                            
                            let event = raw_event.payload.0;
                            let chat_id = raw_event.payload.1;
                            let token_id = raw_event.payload.2;

                            let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("IT'S FOR CHAT_ID {chat_id} | CHAT LEN {} | EVENT ID {}", db_state.db.chats.get_chats().len(), raw_event.id)}).unwrap();
                            invoke("print_to_console", args).await;

                            //let chat = db_state.db.chats.get_chats().get(&chat_id).unwrap().clone();


                            //let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("CHAT PARTS LEN {} | LAST CHAT PART LEN {:?}", chat.context.get_parts().len(), chat.context.get_parts().last().map(|val| {val.get_data().len()}))}).unwrap();
                            //invoke("print_to_console", args).await;

                            match event {
                                EndpointResponseVariant::StartStream(data, position) => {

                                    let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("IT'S A START STREAM EVENT")}).unwrap();
                                    invoke("print_to_console", args).await;
                                    db_state.dispatch(DatabaseAction::AddPartToChat{
                                        chat_id,
                                        token_id,
                                        part:ContextPart::new(vec![data], position)
                                    });
                                },
                                EndpointResponseVariant::ContinueStream(data, position) => {
                                    let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("IT'S A CONTINUE STREAM EVENT")}).unwrap();
                                    invoke("print_to_console", args).await;
                                    db_state.dispatch(DatabaseAction::AddDataToLastPartOfChat {
                                        chat_id,
                                        token_id,
                                        data
                                    });
                                    
                                     
                                },
                                EndpointResponseVariant::EndStream(data, position) => {
                                    let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("IT'S A END STREAM EVENT")}).unwrap();
                                    invoke("print_to_console", args).await;
                                    panic!("Odd, not supposed to get that yet");
                                },
                                _ => {

                                    let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("IT'S AN IMPOSSIBLE STREAM EVENT")}).unwrap();
                                    invoke("print_to_console", args).await;
                                    panic!("Impossible to get that here")
                                }
                            }

                            let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("FINISHED EVENT YAHOOO")}).unwrap();
                            invoke("print_to_console", args).await;
                        }
                    });
                    // Create a Closure from a Box<dyn Fn> - this has to be 'static
                    let listener = EventListener::new(
                        &element,
                        "chat-token",
                        move |e| {}
                    );

                    custard_listener = Some(listener);
                }

                move || drop(custard_listener)
            }
        }
    );
    
    
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
        if db_state.cursors.chosen_tab == i {
            values.push(String::from("chosen"));
        }
        else {
            values.push(String::from("not-chosen"));
        }
    }
    let tab_picker_callbacks:Vec<Callback<MouseEvent>> = (0..8).into_iter().map(|i| {
        let db_state = db_state.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            db_state.dispatch(DatabaseAction::SetTab(i));
            
        })
    }).collect();

    let allocatable_agent_tools = use_state(|| HashSet::<ProximaTool>::new());
    let selected_value = use_state(|| "Option1".to_string());

    let pseudo_node_ref = use_node_ref();
    let prompt_node_ref = use_node_ref();
    let tag_desc_ref = use_node_ref();
    let tag_name_ref = use_node_ref();
    let am_name_ref = use_node_ref();
    let cc_select_ref = use_node_ref();
    let cc_name_ref = use_node_ref();
    let cc_setting_ref = use_node_ref();
    let cc_setting_value_ref = use_node_ref();
    let cc_second_setting_value_ref = use_node_ref();
    let cc_third_setting_value_ref = use_node_ref();
    let second_db_here = db_state.clone();
    let current_app = match db_state.cursors.chosen_tab {
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

                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    db_state.dispatch(DatabaseAction::SetTab(1));
                    let prompt_text = prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    let starting_context = WholeContext::new(vec![ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User)]);
                    let mut start_chat = db_state.db.chats.create_possible_chat(starting_context.clone(), None, proxima_state.device_id, None);
                    let mut local_id = start_chat.id;
                    let proxima_state = proxima_state.clone();
                    let db_state = db_state.clone();
                    db_state.dispatch(DatabaseAction::SetTab(1));
                    db_state.dispatch(DatabaseAction::SetChat(Some(local_id)));
                    let db_state = db_state.clone();
                    spawn_local(async move {
                        let (delta, new_id, new_item) = get_delta_for_add(
                            DatabaseItemID::Chat(local_id),
                            DatabaseItem::Chat(start_chat.clone()),
                            async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                        ).await;

                        if new_id != DatabaseItemID::Chat(local_id) {
                            local_id = match new_id {
                                DatabaseItemID::Chat(id) => id,
                                _ => panic!("Wrong kind of ID after check, impossible")
                            };
                        }

                        db_state.dispatch(DatabaseAction::AddItem(delta, new_id, new_item));

                        let json_request = proxima_backend::web_payloads::AIPayload::new(proxima_state.auth_token.clone(), EndpointRequestVariant::RespondToFullPrompt { whole_context: starting_context, streaming: true, session_type: SessionType::Chat, chat_settings:None, chat_id:Some(local_id), access_mode:db_state.cursors.chosen_access_mode });
                        
                        let value = make_ai_request(json_request, proxima_state.chat_url.clone(), local_id).await;

                        match value {
                            Ok(response) => {
                                match response.reply {
                                    EndpointResponseVariant::Block(context_part) => {
                                        start_chat.add_to_context(context_part);
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(start_chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Chat(local_id), DatabaseItem::Chat(start_chat))])),
                                            Err(()) => ()
                                        }
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
                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let prompt_text = prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    prompt.cast::<web_sys::HtmlInputElement>()
                    .unwrap().set_value("");
                    let (mut local_id, starting_context, created, start_chat, config_opt) = match (db_state.cursors.chosen_chat.clone()) {
                        Some(chatid) => {
                            let mut chat = db_state.db.chats.get_chats().get(&chatid).unwrap().clone();
                            let (context_part, config_opt) = match db_state.cursors.chosen_config {
                                Some(config) => {
                                    chat.config = Some(config);
                                    let config_clone = db_state.db.configs.get_configs()[config].clone();
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
                            match &config_opt {
                                Some(configuration) => chat.context.add_per_turn_settings(configuration),
                                None => ()
                            }
                            (chatid, chat.get_context().clone(), false, chat.clone(), config_opt)
                        },
                        None => {
                            let (starting_context, config_opt) = match db_state.cursors.chosen_config {
                                Some(config) => {
                                    let config_clone = db_state.db.configs.get_configs()[config].clone();
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
                            let mut new_chat = db_state.db.chats.create_possible_chat(starting_context.clone(), None, proxima_state.device_id, config_opt.clone());
                            let new_chatid = new_chat.id;
                            new_chat.access_modes.insert(db_state.cursors.chosen_access_mode);
                            db_state.dispatch(DatabaseAction::SetChat(Some(new_chatid)));
                            (new_chatid, starting_context, true, new_chat, config_opt)
                        }
                    };
                    db_state.dispatch(DatabaseAction::SetTab(1));
                    let proxima_state = proxima_state.clone();
                    let db_state = db_state.clone();
                    spawn_local(async move {
                        if created {
                            let (delta, new_id, new_item) = get_delta_for_add(
                                DatabaseItemID::Chat(local_id),
                                DatabaseItem::Chat(start_chat.clone()),
                                async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                            ).await;
                            if new_id != DatabaseItemID::Chat(local_id) {
                                local_id = match new_id {
                                    DatabaseItemID::Chat(id) => id,
                                    _ => panic!("Wrong kind of ID after check, impossible")
                                };
                            }

                            db_state.dispatch(DatabaseAction::AddItem(delta, new_id, new_item));
                        }
                        else {
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Chat(local_id), DatabaseItem::Chat(start_chat.clone()))]));
                        }

                        let streaming = true;

                        let json_request = proxima_backend::web_payloads::AIPayload::new(proxima_state.auth_token.clone(), EndpointRequestVariant::RespondToFullPrompt { whole_context: starting_context, streaming, session_type: SessionType::Chat, chat_settings:config_opt, chat_id:Some(local_id), access_mode:db_state.cursors.chosen_access_mode });


                        let value = make_ai_request(json_request, proxima_state.chat_url.clone(), local_id).await;
                        match value {
                            Ok(response) => {
                                match response.reply {
                                    EndpointResponseVariant::Block(context_part) => {
                                        let mut chat = start_chat.clone();
                                        chat.add_to_context(context_part);
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => (),
                                            Err(()) => ()
                                        }
                                        db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Chat(chat.id), DatabaseItem::Chat(chat))]));
                                    },
                                    EndpointResponseVariant::MultiTurnBlock(whole_context) => {
                                        let mut chat = start_chat.clone();
                                        chat.context = whole_context;
                                        let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(chat.clone())) };
                                        match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                            Ok(response) => (),
                                            Err(()) => ()
                                        }
                                        db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Chat(chat.id), DatabaseItem::Chat(chat))]));
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
                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    db_state.dispatch(DatabaseAction::SetChat(None));
                })
            };
            let chat_htmls = db_state.db.chats.get_chats().iter().map(|(id, chat)| {
                let db_state = db_state.clone();
                let callback = {
                    let id_clone = *id;
                    let db_state = db_state.clone();
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::SetChat(Some(id_clone)));
                    })
                };

                if !chat.access_modes.contains(&db_state.cursors.chosen_access_mode) {
                    html!()
                }
                else if db_state.cursors.chosen_chat.is_some() && *id == db_state.cursors.chosen_chat.unwrap() {
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
            let chosen_chat_by_id = db_state.db.chats.get_chats().get(&(db_state.cursors.chosen_chat.unwrap_or(1000000)));
            let config_htmls:Vec<Html> = second_db_here.db.configs.get_configs().iter().enumerate().map(|(id, config)| {
                html!(
                    <option value={config.name.clone()}>{config.name.clone()}</option>
                )
            }).collect();

            let cc_select_callback = {
                let select_node = cc_select_ref.clone();
                let db_state = second_db_here.clone();
                Callback::from(move |mouse_evt:Event| {
                    let cc_name = select_node.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    match db_state.db.configs.get_configs().iter().enumerate().find(|(i,config)| {
                        &config.name == &cc_name
                    }) {
                        Some((id, config)) => {
                            let config_data = format!("{:?} {:?}", config.name.clone(), config.tools.is_some());
                            
                            db_state.dispatch(DatabaseAction::ChangeUsedChatConfig(Some(id)));
                            spawn_local(async move {
                                let args = serde_wasm_bindgen::to_value(&PrintArgs {value:config_data}).unwrap();
                                invoke("print_to_console", args).await;
                            });
                        },
                        None => {
                            db_state.dispatch(DatabaseAction::ChangeUsedChatConfig(None));
                        }
                    }
                    
                })
            };
            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">
                        <div class="list-plus-other-col">
                            <div>
                                <h1>{"Past chats"}</h1>
                                <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_chat_callback}>{"New chat"}</button>
                                <hr/>
                            </div>
                            <div class="list-holder all-vertical-space-flex">
                                {
                                    if db_state.db.chats.get_chats().len() > 0 {
                                        chat_htmls
                                    }
                                    else {
                                        html!({"No chats yet !"})
                                    }
                                }
                            </div>
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space chat-tab-not-sidebar">
                        <div class="list-plus-other-col">
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
                                                    <div class={if context_part.is_user() {"standard-padding-margin-corners"} else {"standard-padding-margin-corners nonuser-turn"}}>
                                                    <div> {context_part.data_to_text().iter().map(|string| {VNode::from_html_unchecked(AttrValue::from(to_html(&string.lines().intersperse("\n\n").collect::<Vec<&str>>().concat())))}).collect::<Vec<Html>>() /*context_part.data_to_text().iter().map(|string| {string.split('\n').collect::<Vec<&str>>()}).flatten().map(|string| {html!(string)}).intersperse(html!(<br/>)).collect::<Vec<Html>>()*/}</div>
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
                    
                </div>
            }
        }
        /* Tags */ 2 => {

            let tag_htmls = db_state.db.tags.get_tags().iter().map(|(id, tag)| {
                let db_state = db_state.clone();
                let callback = {

                    let second_db = db_state.clone();
                    let id_clone = *id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        second_db.dispatch(DatabaseAction::SetModifiedTag(Some(id_clone)));

                        second_db.dispatch(DatabaseAction::SetParentTag(second_db.db.tags.get_tags().get(&id_clone).unwrap().get_parent()));
                    })
                };
                if !db_state.db.access_modes.get_modes()[db_state.cursors.chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else if db_state.cursors.chosen_tag.is_some() && *id == db_state.cursors.chosen_tag.unwrap() {
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

            let parent_tag_htmls = db_state.db.tags.get_tags().iter().map(|(id, tag)| {
                let db_state = db_state.clone();
                let callback = {
                    let id_clone = *id;
                    let second_db = db_state.clone();
                    Callback::from(move |mouse_evt:MouseEvent| {
                        second_db.dispatch(DatabaseAction::SetParentTag(Some(id_clone)));
                    })
                };
                if (db_state.cursors.chosen_tag.is_some() && *id == db_state.cursors.chosen_tag.unwrap()) || !db_state.db.access_modes.get_modes()[db_state.cursors.chosen_access_mode].get_tags().contains(&id)  {
                    html!()
                }
                else if db_state.cursors.chosen_parent_tag.is_some() && *id == db_state.cursors.chosen_parent_tag.unwrap() {
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
                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    second_db.dispatch(DatabaseAction::SetModifiedTag(None));
                    second_db.dispatch(DatabaseAction::SetParentTag(None));
                })
            };

            let chosen_tag_by_id = db_state.db.tags.get_tags().get(&(db_state.cursors.chosen_tag.unwrap_or(1000000)));

            let tag_update_callback = {
                let db_state = db_state.clone();
                let tag_name_ref = tag_name_ref.clone();
                let tag_desc_ref = tag_desc_ref.clone();
                let proxima_state = proxima_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let db_state = db_state.clone();
                    
                    let tag_name = tag_name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    let tag_desc = tag_desc_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    let description = Description::new(tag_desc);
                    
                    match db_state.cursors.chosen_tag {
                        Some(tag_id) => {
                            let mut tag = db_state.db.tags.get_tags().get(&tag_id).unwrap().clone();
                            tag.name = tag_name;
                            tag.desc = description;
                            tag.parent = db_state.cursors.chosen_parent_tag;
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Tag(tag_id), DatabaseItem::Tag(tag.clone()))]));
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Tag(tag)) };
                                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => (),
                                    Err(()) => ()
                                }
                            });
                            
                        },
                        None => {
                            let tag = db_state.db.tags.create_possible_tag(NewTag::new(tag_name, description, db_state.cursors.chosen_parent_tag));
                            let mut tag_id = tag.get_id();

                            let am_id = db_state.cursors.chosen_access_mode;
                            let (new_am_0, new_am_n) = db_state.db.access_modes.get_updated_modes_from_association(am_id, tag_id);
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![
                                (DatabaseItemID::AccessMode(0), DatabaseItem::AccessMode(new_am_0.clone())),
                                (DatabaseItemID::AccessMode(new_am_n.get_id()), DatabaseItem::AccessMode(new_am_n.clone())),
                            ]));
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let (delta, new_id, new_item) = get_delta_for_add(
                                    DatabaseItemID::Tag(tag_id),
                                    DatabaseItem::Tag(tag.clone()),
                                    async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                ).await;
                                make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request:DatabaseRequestVariant::Update(DatabaseItem::AccessMode(new_am_0)) }, proxima_state.chat_url.clone()).await;
                                make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request:DatabaseRequestVariant::Update(DatabaseItem::AccessMode(new_am_n)) }, proxima_state.chat_url.clone()).await;
                                

                                db_state.dispatch(DatabaseAction::AddItem(delta, new_id, new_item));
                            });
                        },
                    }
                })
            };

            

            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">
                        <div class="list-plus-other-col">
                            <div>
                                <h1>{"Tags"}</h1>
                                <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_tag_callback}>{"New tag"}</button>
                                <hr/>
                            </div>

                            <div class="list-holder">
                                {
                                    if db_state.db.tags.get_tags().len() > 0 {
                                        tag_htmls
                                    }
                                    else {
                                        html!({"No tags yet !"})
                                    }
                                }
                            </div>
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
                            <div class="list-plus-other-col third-level standard-padding-margin-corners">
                                <h2>
                                    {
                                        match db_state.cursors.chosen_parent_tag {
                                            Some(tag_id) => format!("Currently chosen parent tag : {}", db_state.db.tags.get_tags().get(&tag_id).unwrap().get_name().clone()),
                                            None => "Does this tag have a parent ?".to_string()
                                        }
                                    }
                                </h2>
                                <div class="list-holder">
                                    {
                                        if db_state.db.tags.get_tags().len() > 0 {
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
            let access_mode_htmls = db_state.db.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
                let callback = {

                    let db_state = db_state.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::SetModifiedAM(None));
                        db_state.dispatch(DatabaseAction::SetTagsForAM(db_state.db.access_modes.get_modes()[id_clone].get_tags().clone()));
                    })
                };
                if db_state.cursors.access_mode_for_modification.is_some() && id == db_state.cursors.access_mode_for_modification.unwrap() {
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

            let tag_htmls = db_state.db.tags.get_tags().iter().map(|(id, tag)| {
                let callback = {

                    let db_state = db_state.clone();
                    let id_clone = *id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::AddToTagsForAM(id_clone));
                    })
                };
                if db_state.cursors.chosen_access_mode_tags.contains(&id) || !db_state.db.access_modes.get_modes()[db_state.cursors.chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();

            let chosen_tag_htmls = db_state.db.tags.get_tags().iter().map(|(id, tag)| {
                let callback = {
                    let db_state = db_state.clone();
                    let id_clone = *id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::RemoveFromTagsForAM(id_clone));
                    })
                };
                if !db_state.cursors.chosen_access_mode_tags.contains(&id) || !db_state.db.access_modes.get_modes()[db_state.cursors.chosen_access_mode].get_tags().contains(&id) {
                    html!()
                }
                else {
                    html!(
                        <div><button onclick={callback} class="chat-option">{tag.get_name().clone()}</button></div>
                    )
                }
            }).collect::<Html>();
            
            let new_tag_callback = {

                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    db_state.dispatch(DatabaseAction::SetModifiedAM(None));
                    db_state.dispatch(DatabaseAction::SetTagsForAM(HashSet::new()));
                })
            };

            let chosen_am_by_id = db_state.db.access_modes.get_modes().get((db_state.cursors.access_mode_for_modification.unwrap_or(1000000)));

            let am_update_callback = {
                let db_state = db_state.clone();
                let proxima_state = proxima_state.clone();
                let am_name_ref = am_name_ref.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let mut db_state = db_state.clone();
                    
                    let am_name = am_name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();

                    match db_state.cursors.access_mode_for_modification {
                        Some(am_id) => {
                            let mut am = db_state.db.access_modes.get_modes()[am_id].clone();
                            am.name = am_name;
                            am.tags = db_state.cursors.chosen_access_mode_tags.clone();
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::AccessMode(am_id), DatabaseItem::AccessMode(am.clone()))]));
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::AccessMode(am)) };
                                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                    Ok(response) => (),
                                    Err(()) => ()
                                }
                            });
                        },
                        None => {
                            let am = AccessMode::new(0, db_state.cursors.chosen_access_mode_tags.clone(), am_name);
                            let id = get_next_id_for_category(&db_state.db, &DatabaseItem::AccessMode(am.clone()));
                            let proxima_state = proxima_state.clone();

                            let db_state = db_state.clone();
                            spawn_local(async move {
                                let (delta, new_id, new_item) = get_delta_for_add(
                                    id,
                                    DatabaseItem::AccessMode(am.clone()),
                                    async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                                ).await;

                                db_state.dispatch(DatabaseAction::AddItem(delta, new_id, new_item));
                            });
                        },
                    }
                    db_state.dispatch(DatabaseAction::SetModifiedAM(None));
                })
            };

            html!{
                <div class="chat-part">
                    <div class="all-vertical-space standard-padding-margin-corners first-level">

                        <div class="list-plus-other-col">
                            <div>
                                <h1>{"Access Modes"}</h1>
                                <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_tag_callback}>{"New Access Mode"}</button>
                                <hr/>
                            </div>

                            <div class="list-holder">
                                {
                                    if db_state.db.access_modes.get_modes().len() > 0 {
                                        access_mode_htmls
                                    }
                                    else {
                                        html!({"Something is very wrong, you are supposed to have AT LEAST 1 Access mode"})
                                    }
                                }
                            </div>
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
                            <div class="list-plus-other-col third-level standard-padding-margin-corners">
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
                                                if db_state.db.tags.get_tags().len() > 0 {
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
                                                if db_state.db.tags.get_tags().len() > 0 {
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
                let name_ref = cc_name_ref.clone();
                let db_state = db_state.clone();
                let proxima_state = proxima_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    
                    let mut db_state = db_state.clone();
                    let proxima_state = proxima_state.clone();
                    let name = name_ref.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    if !name.trim().is_empty() {
                        let mut config = ChatConfiguration::new(name, vec![]);
                        config.access_modes.insert(db_state.cursors.chosen_access_mode.clone());
                        let id = get_next_id_for_category(&db_state.db, &DatabaseItem::ChatConfig(config.clone()));
                        let proxima_state = proxima_state.clone();
                        spawn_local(async move {
                            let (delta, new_id, new_item) = get_delta_for_add(
                                id,
                                DatabaseItem::ChatConfig(config.clone()),
                                async |request| {make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request }, proxima_state.chat_url.clone()).await.map(|response| {response.reply})}
                            ).await;

                            db_state.dispatch(DatabaseAction::AddItem(delta, new_id, new_item));
                        });
                    }
                    
                })
            };
            let ccs_htmls = db_state.db.configs.get_configs().iter().enumerate().map(|(id, config)| {
                let callback = {

                    let db_state = db_state.clone();
                    let id_clone = id;
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::SetModifiedConfig(Some(id_clone)));
                        db_state.dispatch(DatabaseAction::SetConfigSettingID(None));
                        db_state.dispatch(DatabaseAction::SetCurrentSetting(None));
                    })
                };
                if !config.access_modes.contains(&db_state.cursors.chosen_access_mode) {
                    html!()
                }
                else if db_state.cursors.config_for_modification.is_some() && id == db_state.cursors.config_for_modification.unwrap() {
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

            let chosen_cc_settings_htmls = match db_state.cursors.config_for_modification {
                Some(config_id) => {
                    let client_db = db_state.db.clone();
                    client_db.configs.get_configs()[config_id].raw_settings.iter().enumerate().map(|(id, setting)| {

                        let setting_c = setting.clone();
                        let callback = {

                            let db_state = db_state.clone();
                            let id_clone = id;
                            let setting_clone = setting_c.clone();
                            Callback::from(move |mouse_evt:MouseEvent| {
                                db_state.dispatch(DatabaseAction::SetConfigSettingID(Some(id_clone)));
                                db_state.dispatch(DatabaseAction::SetCurrentSetting(Some(setting_clone.clone())));
                            })
                        };
                        if db_state.cursors.chosen_setting.is_some() && id == db_state.cursors.chosen_setting.unwrap() {
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
                None => if db_state.db.configs.get_configs().len() > 0 {
                    html!({"Please pick a configuration to modify"})
                }
                else {
                    html!({"Please create a configuration to add settings to it"})
                },
            };

            let select_settings_callback = {
                let select_node = cc_setting_ref.clone();
                let db_state = second_db_here.clone();
                Callback::from(move |mouse_evt:Event| {
                    let selected_setting_string = select_node.cast::<web_sys::HtmlInputElement>()
                    .unwrap()
                    .value();
                    db_state.dispatch(DatabaseAction::SetConfigSettingID(None));
                    db_state.dispatch(DatabaseAction::SetCurrentSetting(match selected_setting_string.trim() {
                        "Temperature" => Some(ChatSetting::Temperature(70)),
                        "System prompt" => Some(ChatSetting::SystemPrompt(ContextPart::new(vec![], ContextPosition::System))),
                        "Initial Pre-prompt" => Some(ChatSetting::PrePrompt(ContextPart::new(vec![], ContextPosition::User))),
                        "Repeated Pre-prompt" => Some(ChatSetting::RepeatedPrePrompt(ContextPart::new(vec![], ContextPosition::User), RepeatPosition::AfterLatest)),
                        "Max context length" => Some(ChatSetting::MaxContextLength(10000)),
                        "Max response length" => Some(ChatSetting::ResponseTokenLimit(10000)),
                        "Tool" => Some(ChatSetting::Tool(ProximaTool::Calculator, None)),
                        _ => None
                    }));
                })
            };

            let on_click_callback = {
                let allocatable = allocatable_agent_tools.clone();
                let cc_setting_value_ref = cc_setting_value_ref.clone();
                let cc_second_setting_value_ref = cc_second_setting_value_ref.clone();
                let cc_third_setting_value_ref = cc_third_setting_value_ref.clone();
                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let cc_setting_value_ref = cc_setting_value_ref.clone();
                    let new_setting = match (db_state.cursors.setting_for_modification).clone() {
                        Some(setting) => match setting {
                            ChatSetting::PrePrompt(prompt) => ChatSetting::PrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::User)),
                            ChatSetting::RepeatedPrePrompt(prompt, position) => ChatSetting::RepeatedPrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], 
                                match cc_second_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "User" => ContextPosition::User,
                                    "AI" => ContextPosition::AI,
                                    val => {
                                        let value = val.to_string();
                                        spawn_local(async move {
                                            let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("got {}", value)}).unwrap();
                                            invoke("print_to_console", args).await;
                                        });
                                        ContextPosition::User
                                },
                                }
                                ),
                                match cc_third_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "Before latest" => RepeatPosition::BeforeLatest,
                                    "After latest" => RepeatPosition::AfterLatest,
                                    _ => RepeatPosition::AfterLatest
                                }
                            ),
                            ChatSetting::Temperature(temp) => ChatSetting::Temperature(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::SystemPrompt(prompt) => ChatSetting::SystemPrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::System)),
                            ChatSetting::Tool(tool, _) => match cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                "Calculator" => ChatSetting::Tool(ProximaTool::Calculator, None),
                                "Local Memory" => ChatSetting::Tool(ProximaTool::LocalMemory, None),
                                "Web" => ChatSetting::Tool(ProximaTool::Web, None),
                                "Python" => ChatSetting::Tool(ProximaTool::Python, None),
                                "Agent" => ChatSetting::Tool(ProximaTool::Agent, Some(ProximaToolData::Agent(
                                        AgentToolData::new(allocatable.iter().map(|tool| {tool.clone()}).collect())
                                    ))),
                                "RNG" => ChatSetting::Tool(ProximaTool::Rng, None),
                                "Memory" => ChatSetting::Tool(ProximaTool::Memory, Some(ProximaToolData::Memory { access_mode_id: db_state.cursors.chosen_access_mode })),
                                _ => panic!("Impossible")
                            },
                            ChatSetting::MaxContextLength(length) => ChatSetting::MaxContextLength(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::ResponseTokenLimit(limit) => ChatSetting::ResponseTokenLimit(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::AccessMode(access_mode) => {
                                let access_mode_name = cc_setting_value_ref.cast::<web_sys::HtmlInputElement>()
                                .unwrap()
                                .value();
                                match db_state.db.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
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

                    db_state.dispatch(DatabaseAction::SetCurrentSetting(Some(new_setting)));
                }
                )
            };

            let on_change_callback = {
                let allocatable = allocatable_agent_tools.clone();
                let cc_setting_value_ref = cc_setting_value_ref.clone();
                let cc_second_setting_value_ref = cc_second_setting_value_ref.clone();
                let cc_third_setting_value_ref = cc_third_setting_value_ref.clone();
                let db_state = db_state.clone();
                Callback::from(move |mouse_evt:Event| {
                    let cc_setting_value_ref = cc_setting_value_ref.clone();
                    let new_setting = match (db_state.cursors.setting_for_modification).clone() {
                        Some(setting) => match setting {
                            ChatSetting::PrePrompt(prompt) => ChatSetting::PrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::User)),
                            ChatSetting::RepeatedPrePrompt(prompt, position) => ChatSetting::RepeatedPrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], 
                                match cc_second_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "User" => ContextPosition::User,
                                    "AI" => ContextPosition::AI,
                                    _ => ContextPosition::User
                                }
                                ),
                                match cc_third_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "Before latest" => RepeatPosition::BeforeLatest,
                                    "After latest" => RepeatPosition::AfterLatest,
                                    _ => RepeatPosition::AfterLatest
                                }
                            ),
                            ChatSetting::Temperature(temp) => ChatSetting::Temperature(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::SystemPrompt(prompt) => ChatSetting::SystemPrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::System)),
                            ChatSetting::Tool(tool, _) => match cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                "Calculator" => ChatSetting::Tool(ProximaTool::Calculator, None),
                                "Local Memory" => ChatSetting::Tool(ProximaTool::LocalMemory, None),
                                "Web" => ChatSetting::Tool(ProximaTool::Web, None),
                                "Python" => ChatSetting::Tool(ProximaTool::Python, None),
                                "Agent" => {
                                    ChatSetting::Tool(ProximaTool::Agent, Some(ProximaToolData::Agent(
                                        AgentToolData::new(allocatable.iter().map(|tool| {tool.clone()}).collect())
                                    )))
                                },
                                "RNG" => ChatSetting::Tool(ProximaTool::Rng, None),
                                "Memory" => ChatSetting::Tool(ProximaTool::Memory, Some(ProximaToolData::Memory { access_mode_id: db_state.cursors.chosen_access_mode })),
                                _ => panic!("Impossible")
                            },
                            ChatSetting::MaxContextLength(length) => ChatSetting::MaxContextLength(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::ResponseTokenLimit(limit) => ChatSetting::ResponseTokenLimit(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::AccessMode(access_mode) => {
                                let access_mode_name = cc_setting_value_ref.cast::<web_sys::HtmlInputElement>()
                                .unwrap()
                                .value();
                                match db_state.db.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
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
                    db_state.dispatch(DatabaseAction::SetCurrentSetting(Some(new_setting)));
                }
                )
            };

            let add_update_settings_callback = {
                let allocatable = allocatable_agent_tools.clone();
                let cc_setting_value_ref = cc_setting_value_ref.clone();
                let cc_second_setting_value_ref = cc_second_setting_value_ref.clone();
                let cc_third_setting_value_ref = cc_third_setting_value_ref.clone();
                let client_db = db_state.clone();
                let proxima_state = proxima_state.clone();
                Callback::from(move |mouse_evt:MouseEvent| {
                    let cc_setting_value_ref = cc_setting_value_ref.clone();
                    let proxima_state = proxima_state.clone();
                    let db_state = client_db.clone();
                    let new_setting = match (db_state.cursors.setting_for_modification).clone() {
                        Some(setting) => match setting {
                            ChatSetting::PrePrompt(prompt) => ChatSetting::PrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::User)),
                            ChatSetting::RepeatedPrePrompt(prompt, position) => ChatSetting::RepeatedPrePrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], 
                                match cc_second_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "User" => ContextPosition::User,
                                    "AI" => ContextPosition::AI,
                                    _ => ContextPosition::User
                                }
                                ),
                                match cc_third_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                    "Before latest" => RepeatPosition::BeforeLatest,
                                    "After latest" => RepeatPosition::AfterLatest,
                                    _ => RepeatPosition::AfterLatest
                                }
                            ),
                            ChatSetting::Temperature(temp) => ChatSetting::Temperature(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::SystemPrompt(prompt) => ChatSetting::SystemPrompt(ContextPart::new(vec![
                                ContextData::Text(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value())
                                ], ContextPosition::System)),
                            ChatSetting::Tool(tool, data) => match cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                "Calculator" => ChatSetting::Tool(ProximaTool::Calculator, None),
                                "Local Memory" => ChatSetting::Tool(ProximaTool::LocalMemory, None),
                                "Web" => ChatSetting::Tool(ProximaTool::Web, None),
                                "Python" => ChatSetting::Tool(ProximaTool::Python, None),
                                "Agent" => ChatSetting::Tool(ProximaTool::Agent, Some(ProximaToolData::Agent(
                                        AgentToolData::new(allocatable.iter().map(|tool| {tool.clone()}).collect())
                                    ))),
                                "RNG" => ChatSetting::Tool(ProximaTool::Rng, None),
                                "Memory" => ChatSetting::Tool(ProximaTool::Memory, Some(ProximaToolData::Memory { access_mode_id: db_state.cursors.chosen_access_mode })),
                                _ => panic!("Impossible")
                            },
                            ChatSetting::MaxContextLength(length) => ChatSetting::MaxContextLength(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::ResponseTokenLimit(limit) => ChatSetting::ResponseTokenLimit(cc_setting_value_ref.cast::<web_sys::HtmlInputElement>().unwrap().value().parse().unwrap()),
                            ChatSetting::AccessMode(access_mode) => {
                                let access_mode_name = cc_setting_value_ref.cast::<web_sys::HtmlInputElement>()
                                .unwrap()
                                .value();
                                match db_state.db.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
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
                    db_state.dispatch(DatabaseAction::SetCurrentSetting(Some(new_setting.clone())));
                    match db_state.cursors.config_for_modification {
                        Some(config_id) => {
                            let proxima_state = proxima_state.clone();
                            spawn_local(async move {
                                let mut config = db_state.db.configs.get_configs()[config_id].clone();

                                match db_state.cursors.chosen_setting {
                                    Some(setting) => {
                                        config.raw_settings[setting] = new_setting.clone();
                                    },
                                    None => {

                                        db_state.dispatch(DatabaseAction::SetConfigSettingID(Some(config.raw_settings.len())));
                                        config.raw_settings.push((new_setting.clone()));
                                    },
                                }
                                config.tools = Tools::try_from_settings(config.raw_settings.clone());
                                config.last_updated = Utc::now();
                                db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::ChatConfiguration(config_id), DatabaseItem::ChatConfig(config.clone()))]));
                                let proxima_state = proxima_state.clone();
                                spawn_local(async move {
                                    let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::ChatConfig(config)) };
                                    match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                                        Ok(response) => (),
                                        Err(()) => ()
                                    }
                                });
                            });
                        },
                        None => ()
                    }
                        
                    
                })
            };
            let setting_config = {
                match (db_state.cursors.setting_for_modification).clone() {
                    Some(setting) => match setting {
                        ChatSetting::PrePrompt(prompt) => 
                        {
                            let text = prompt.data_to_text().concat();
                            html!(
                                <div class="label-input-combo second-level standard-padding-margin-corners">
                                    <div><p>{"Pre-prompt value : "}</p></div>
                                    <div class="label-input-combo"><textarea class="standard-padding-margin-corners" rows="10" onclick={on_click_callback} onchange={on_change_callback} placeholder="Pre-prompt here..." id="pre_pre_prompt" ref={cc_setting_value_ref} defaultvalue={text}/></div>
                                </div>
                            )
                        },
                        ChatSetting::RepeatedPrePrompt(prompt, position) => 
                        {
                            let text = prompt.data_to_text().concat();
                            html!(    
                                <div class="second-level standard-padding-margin-corners">
                                    <div><p>{"Pre-prompt added before the latest message : "}</p></div>
                                    <div class="label-input-combo"><textarea class="standard-padding-margin-corners" rows="10" placeholder="Pre-prompt here..." onclick={on_click_callback.clone()} onchange={on_change_callback.clone()} id="pre_prompt" ref={cc_setting_value_ref} defaultvalue={text}/></div>
                                    <div class="label-input-combo">
                                        <p>{"Role of the pre-prompt : "}</p>
                                        <select class="standard-padding-margin-corners" id="role_select" ref={cc_second_setting_value_ref} onchange={on_change_callback.clone()} onclick={on_click_callback.clone()}>
                                            <option value={"User"}>{"User"}</option>
                                            <option value={"AI"}>{"AI"}</option>
                                        </select>
                                    </div>
                                    <div class="label-input-combo">
                                        <p>{"Position of the pre-prompt : "}</p>
                                        <select class="standard-padding-margin-corners" id="position_select" ref={cc_third_setting_value_ref} onchange={on_change_callback} onclick={on_click_callback}>
                                            <option value={"Before latest"}>{"Before latest"}</option>
                                            <option value={"After latest"}>{"After latest"}</option>
                                        </select>
                                    </div>
                                </div>
                            )
                        },
                        ChatSetting::Temperature(temp) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{format!("Temperature : {}", temp as f64/100.0)}</p>
                                <input class="standard-padding-margin-corners" onclick={on_click_callback} onchange={on_change_callback} type="range" id="temp_slider" min="0" max="200" step="1" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::SystemPrompt(prompt) => 
                        {
                            let text = prompt.data_to_text().concat();
                            html!(
                                <div class="second-level standard-padding-margin-corners">
                                    <div><p>{"System prompt part : "}</p></div>
                                    
                                    <div class="label-input-combo"><textarea class="standard-padding-margin-corners" rows="10" onclick={on_click_callback} onchange={on_change_callback} placeholder="System prompt here..." id="system_prompt" ref={cc_setting_value_ref} defaultvalue={text}/></div>
                                </div>
                            )
                        },
                        ChatSetting::Tool(tool,_) => {
                            let addition = match tool {
                                ProximaTool::Agent => {
                                    let second_setting = cc_second_setting_value_ref.clone();
                                    let add_tool_callback = {
                                        let allocatable_agent_tools = allocatable_agent_tools.clone();
                                        let second_setting = second_setting;
                                        Callback::from(move |mouse_evt:MouseEvent| {
                                            let new_tool = match second_setting.cast::<web_sys::HtmlInputElement>().unwrap().value().trim() {
                                                "Calculator" => ProximaTool::Calculator,
                                                "Local Memory" => ProximaTool::LocalMemory,
                                                "Web" => ProximaTool::Web,
                                                "Python" => ProximaTool::Python,
                                                "Agent" => ProximaTool::Agent,
                                                "RNG" => ProximaTool::Rng,
                                                "Memory" => ProximaTool::Memory,
                                                _ => panic!("Should be impossible")
                                            };
                                            let mut allocatable = (*allocatable_agent_tools).clone();
                                            allocatable.insert(new_tool);
                                            allocatable_agent_tools.set(allocatable);
                                        })
                                    };
                                    let clear_tools_callback = {
                                        let allocatable_agent_tools = allocatable_agent_tools.clone();
                                        Callback::from(move |mouse_evt:MouseEvent| {
                                            allocatable_agent_tools.set(HashSet::new());
                                        })
                                    };
                                    html!(
                                        <div>
                                            <h2>{"Tools that can be allocated to agents"}</h2>
                                            <select class="standard-padding-margin-corners" id="tool_allocation_select" ref={cc_second_setting_value_ref}>
                                                <option value={"Calculator"}>{"Calculator"}</option>
                                                <option value={"Local Memory"}>{"Local Memory"}</option>
                                                <option value={"Web"}>{"Web"}</option>
                                                <option value={"Python"}>{"Python"}</option>
                                                <option value={"Agent"}>{"Agent"}</option>
                                                <option value={"RNG"}>{"RNG"}</option>
                                                <option value={"Memory"}>{"Memory"}</option>
                                            </select>
                                            <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={add_tool_callback}>{"Add"}</button>
                                            <div class="list-holder">
                                                {
                                                    allocatable_agent_tools.iter().map(|tool| {html!(<label>{tool.get_name().as_str()}</label>)}).collect::<Html>()
                                                }
                                            </div>

                                            <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={clear_tools_callback}>{"Clear list"}</button>
                                        </div>
                                    )
                                },
                                _ => html!()
                            };
                            html!(
                                <div class="label-input-combo second-level standard-padding-margin-corners">
                                    <p>{"Tool to add : "}</p>
                                    <select class="standard-padding-margin-corners" id="tool_select" ref={cc_setting_value_ref}>
                                        <option value={"Calculator"}>{"Calculator"}</option>
                                        <option value={"Local Memory"}>{"Local Memory"}</option>
                                        <option value={"Web"}>{"Web"}</option>
                                        <option value={"Python"}>{"Python"}</option>
                                        <option value={"Agent"}>{"Agent"}</option>
                                        <option value={"RNG"}>{"RNG"}</option>
                                        <option value={"Memory"}>{"Memory"}</option>
                                    </select>
                                    {addition}
                                </div>
                            )
                        },
                        ChatSetting::MaxContextLength(length) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{format!("Max context length (in tokens) : {}", length)}</p>
                                <input class="standard-padding-margin-corners" onclick={on_click_callback} onchange={on_change_callback} type="range" id="context_slider" min="512" max="32000" step="256" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::ResponseTokenLimit(limit) => html!(
                            <div class="label-input-combo second-level standard-padding-margin-corners">
                                <p>{format!("Max response length (in tokens) : {}", limit)}</p>
                                <input class="standard-padding-margin-corners" onclick={on_click_callback} onchange={on_change_callback} type="range" id="response_slider" min="512" max="32000" step="256" ref={cc_setting_value_ref} />
                            </div>
                        ),
                        ChatSetting::AccessMode(access_mode) => {
                            let access_modes_htmls:Vec<Html> = second_db_here.db.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
                                html!(
                                    <option value={access_mode.get_name().clone()}>{access_mode.get_name().clone()}</option>
                                )
                            }).collect();

                            html!(
                                <div class="label-input-combo second-level standard-padding-margin-corners">
                                    <p>{"System prompt part : "}</p>
                                    <select class="standard-padding-margin-corners" onclick={on_click_callback} onchange={on_change_callback} id="access_select" ref={cc_setting_value_ref}>
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

                        <div class="list-plus-other-col">
                            <div>
                                <h1>{"Chat configurations"}</h1>
                                <input class="standard-padding-margin-corners most-horizontal-space-no-flex" placeholder="Chat config name..." ref={cc_name_ref}/>
                                <button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={new_cc_callback}>{"New Chat Configuration"}</button>
                                <hr/>
                            </div>

                            <div class="list-holder most-horizontal-space-no-flex">
                                {
                                    if db_state.db.configs.get_configs().len() > 0 {
                                        ccs_htmls
                                    }
                                    else {
                                        html!({"To create a chat configuration, please give it a non-empty name and click \"New Configuration\" above"})
                                    }
                                }
                            </div>
                        </div>
                    </div>
                    <div class="all-vertical-space standard-padding-margin-corners first-level at-most-a-sixth-width">

                        <div class="list-plus-other-col">
                            <div>
                                <h1>{"Configuration settings"}</h1>
                                <h2>
                                {
                                    match db_state.cursors.config_for_modification {
                                        Some(config) => html!({format!("For : {}", db_state.db.configs.get_configs()[config].name.clone())}),
                                        None => html!()
                                    }
                                }
                                </h2>
                                <select class="most-horizontal-space-no-flex standard-padding-margin-corners" ref={cc_setting_ref} onchange={select_settings_callback}>
                                    <option value={"Temperature"}>{"Temperature"}</option>
                                    <option value={"System prompt"}>{"System prompt"}</option>
                                    <option value={"Initial Pre-prompt"}>{"Initial Pre-prompt"}</option>
                                    <option value={"Repeated Pre-prompt"}>{"Repeated Pre-prompt"}</option>
                                    <option value={"Max context length"}>{"Max context length"}</option>
                                    <option value={"Max response length"}>{"Max response length"}</option>
                                    <option value={"Tool"}>{"Tool"}</option>
                                </select>
                                <hr/>
                            </div>

                            <div class="list-holder most-horizontal-space-no-flex">
                                {
                                    chosen_cc_settings_htmls
                                }
                            </div>
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
                                match (db_state.cursors.setting_for_modification).clone() {
                                    Some(setting) => html!(<button class="mainapp-button standard-padding-margin-corners most-horizontal-space-no-flex" onclick={add_update_settings_callback}>
                                        {
                                            match db_state.cursors.chosen_setting {
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
        /* Notifications */6 => {
            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state.clone()}>
                    <NotificationTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
            )
        }
        _ => html!({"Something is very wrong"})
    };
    let access_mode_select = use_node_ref();
    let access_mode_callback = {
        let select_node = access_mode_select.clone();
        let db_clone = second_db_here.clone();
        Callback::from(move |mouse_evt:Event| {
            let access_mode_name = select_node.cast::<web_sys::HtmlInputElement>()
            .unwrap()
            .value();
            match db_state.db.access_modes.get_modes().iter().enumerate().find(|(i,access_mode)| {
                access_mode.get_name() == &access_mode_name
            }) {
                Some((id, access_mode)) => {
                    let second_db_clone = db_clone.clone();
                    let access_mode_name_clone = access_mode_name.clone();
                    if db_state.cursors.chosen_access_mode != id {
                        db_state.dispatch(DatabaseAction::SetChat(None));
                        
                        db_state.dispatch(DatabaseAction::SetModifiedTag(None));
                        db_state.dispatch(DatabaseAction::SetParentTag(None));
                    }
                    
                    db_state.dispatch(DatabaseAction::SetGlobalAM(id));
                    spawn_local(async move {
                        let args = serde_wasm_bindgen::to_value(&PrintArgs {value:access_mode_name_clone}).unwrap();
                        invoke("print_to_console", args).await;

                    });
                },
                None => ()
            }
            
        })
    };
    let access_modes_htmls:Vec<Html> = second_db_here.db.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
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
                    <button class="menu-item" id={values[5].clone()} onclick={tab_picker_callbacks[5].clone()}>{"Configurations"}</button>
                    <button class="menu-item" id={values[6].clone()} onclick={tab_picker_callbacks[6].clone()}>{"Notifications"}</button>
                    <select class="menu-item" ref={access_mode_select} onchange={access_mode_callback}>
                        {access_modes_htmls}
                    </select>
                </div>
            </div>
            <div class="interactive-part">
                {current_app}
            </div>
            <div ref={event_div_node_ref}></div>
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
    pub initialized:bool,
    pub loaded:bool,
    pub username:String,
    pub auth_token:String,
    pub chat_url:String,
    pub device_id:DeviceID,
    pub start_db:Option<ProxDatabase>
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
