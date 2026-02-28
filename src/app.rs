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

use crate::{db_sync::{UserCursors, apply_server_updates, get_delta_for_add, get_next_id_for_category, handle_add, handle_add_reducible}, tabs::{access_modes_tab::AccessModesTab, chat_configs_tab::ChatConfigsTab, chat_tab::ChatTab, home_tab::HomeTab, notification_tab::NotificationTab, tags_tab::TagsTab}};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
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
pub struct PrintArgs {
    pub value:String,
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

    let selected_value = use_state(|| "Option1".to_string());

    let pseudo_node_ref = use_node_ref();
    let prompt_node_ref = use_node_ref();
    let tag_desc_ref = use_node_ref();
    let tag_name_ref = use_node_ref();
    let am_name_ref = use_node_ref();
    let cc_select_ref = use_node_ref();
    let second_db_here = db_state.clone();
    let current_app = match db_state.cursors.chosen_tab {
        /*Home*/ 0 => {
            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state}>
                    <HomeTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
            )
        },
        /*Chat*/ 1 => 
        {
            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state}>
                    <ChatTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
            )
        }
        /* Tags */ 2 => {

            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state}>
                    <TagsTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
            )
        },
        /* Access Modes */ 3 => {
            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state}>
                    <AccessModesTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
            )
        },
        /*Files*/ 4 => html!(),
        /*Chat Settings*/ 5 => {
            let db_state = db_state.clone();
            html!(
                <ContextProvider<UseReducerHandle<DatabaseState>> context={db_state}>
                    <ChatConfigsTab/>
                </ContextProvider<UseReducerHandle<DatabaseState>>>
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
