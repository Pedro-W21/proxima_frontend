use std::collections::HashSet;
use std::path::PathBuf;

use chrono::Utc;
use futures::StreamExt;
use gloo_events::EventListener;
use gloo_utils::format::JsValueSerdeExt;
use html_parser::{Dom, Node};
use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::access_modes::AMSetting;
use proxima_backend::database::chats::{Chat, ChatID, SessionType};
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::media::{Base64EncodedString, Media, MediaType};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseReplyVariant, DatabaseRequestVariant};
use proxima_backend::web_payloads::{DBPayload, DBResponse};
use serde::{Deserialize, Serialize};
use tauri_sys::dpi::PhysicalPosition;
use tauri_sys::window::DragDropEvent;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, Properties, UseReducerHandle, function_component, html, use_context, use_effect_with, use_node_ref, use_state_eq};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request, print};
use crate::db_sync::get_delta_for_add;
use crate::html_parsing::{HtmlNode, parse_html};

#[derive(Serialize, Deserialize)]
pub struct FileArgs {
    test1: PathBuf,
    test2:String, 
    test3:String
}

#[derive(Deserialize, Clone)]
pub struct SpecialDragDrop {
    paths:Vec<PathBuf>,
    position:PhysPos
}


#[derive(Deserialize, Clone)]
pub struct PhysPos {
    x:f64,
    y:f64,
}

#[derive(Clone, PartialEq, Eq)]
pub enum SortingMode {
    None,
    AscendingID,
    DescendingID,
    AscendingTime,
    DescendingTime
}

impl SortingMode {
    pub fn get_next(&self) -> SortingMode {
        match self {
            Self::None => Self::AscendingID,
            Self::AscendingID => Self::DescendingID,
            Self::DescendingID => Self::AscendingTime,
            Self::AscendingTime => Self::DescendingTime,
            Self::DescendingTime => Self::None
        }
    }
    pub fn get_title(&self) -> String {
        match self {
            Self::None => "Nothing".to_string(),
            Self::AscendingID => "Ascending ID".to_string(),
            Self::DescendingID => "Descending ID".to_string(),
            Self::AscendingTime => "Ascending time".to_string(),
            Self::DescendingTime => "Descending time".to_string()
        }
    }
}
#[function_component(ChatTab)]

pub fn chat_tab() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let prompt_node_ref = use_node_ref();
    let cc_select_ref = use_node_ref();
    let file_ref = use_node_ref();
    let files_state = use_state_eq(Vec::<PathBuf>::default);
    let sort_state = use_state_eq(|| {SortingMode::None});

    use_effect_with(
        files_state.clone(),
        {
            let div_node_ref = file_ref.clone();
            let second_db = db_state.clone();
            let proxima_state = proxima_state.clone();
            let files_state = files_state.clone();
            move |_| {
                let mut custard_listener = None;

                let db_state = second_db.clone();
                let proxima_state = proxima_state.clone();
                if let Some(element) = div_node_ref.cast::<HtmlElement>() {
                    // Create your Callback as you normally would
                    let oncustard = Callback::from(move |e: Event| {
                        
                    });
                    spawn_local(async move {
                        let listener = tauri_sys::event::listen::<SpecialDragDrop>("special-drag-and-drop").await.unwrap();

                        print("STARTED LISTENING FOR DRAG AND DROP").await;
                        let (mut listener, mut abort_handle) = futures::stream::abortable(listener);
                        while let Some(raw_event) = listener.next().await {
                            let mut current_files = (*files_state).clone();
                            let mut new = false;
                            for path in &raw_event.payload.paths {
                                if !current_files.contains(path) {
                                    current_files.push(path.clone());
                                    new = true;
                                } 
                                
                            }
                            if new {
                                files_state.set(current_files);
                                print("RECEIVED FILE YAHOO").await;
                                break;
                            }
                        }
                    });
                    // Create a Closure from a Box<dyn Fn> - this has to be 'static
                    let listener = EventListener::new(
                        &element,
                        "special-drag-and-drop",
                        move |e| {
                        
                        }
                    );

                    custard_listener = Some(listener);
                }

                move || drop(custard_listener)
            }
        }
    );

    let chat_remove_callback = {
        let db_state = db_state.clone();
        let proxima_state = proxima_state.clone();

        Callback::from(move |mouse_evt:MouseEvent| {

            let db_state = db_state.clone();
            let proxima_state = proxima_state.clone();
            spawn_local(async move {
                if let Some(chat_id) = db_state.cursors.chosen_chat {
                    db_state.dispatch(DatabaseAction::SetChat(None));
                    let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Remove(DatabaseItemID::Chat(chat_id)) };
                    match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                        Ok(response) => {
                            db_state.dispatch(DatabaseAction::RemoveItem(DatabaseItemID::Chat(chat_id)));
                        },
                        Err(()) => ()
                    }
                }
                
            });
        })
    };

    let prompt_send_callback = {
        let prompt = prompt_node_ref.clone();
        let proxima_state = proxima_state.clone();
        let db_state = db_state.clone();
        let files_state = files_state.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            let prompt_text = prompt.cast::<web_sys::HtmlInputElement>()
            .unwrap()
            .value();
            prompt.cast::<web_sys::HtmlInputElement>()
            .unwrap().set_value("");
            let (mut local_id, mut starting_context, created, mut start_chat, config_opt) = match (db_state.cursors.chosen_chat.clone()) {
                Some(chatid) => {
                    let mut chat = db_state.db.chats.get_chats().get(&chatid).unwrap().clone();
                    let (context_part, config_opt) = match db_state.cursors.chosen_config {
                        Some(config) => {
                            chat.config = Some(config);
                            if let Some(config_clone) = db_state.db.configs.get_configs().get(&config) {
                                chat.latest_used_config = Some(config_clone.clone());
                                match &config_clone.tools {
                                    Some(tools) => {
                                        (ContextPart::new_user_prompt_with_tools(vec![ContextData::Text(prompt_text)]), Some(config_clone.clone()))
                                    },
                                    None => (ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User), Some(config_clone.clone()))
                                }
                            }
                            else {
                                panic!("Impossible, config should still exist here (except if server deleted it)")
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
                            if let Some(config_clone) = db_state.db.configs.get_configs().get(&config) {
                                match config_clone.tools.clone() {
                                    Some(tools) => {
                                        (WholeContext::new_with_all_settings(vec![ContextPart::new_user_prompt_with_tools(vec![ContextData::Text(prompt_text)])], &config_clone), Some(config_clone.clone()))
                                    },
                                    None => (WholeContext::new_with_all_settings(vec![ContextPart::new(vec![ContextData::Text(prompt_text)], ContextPosition::User)], &config_clone), Some(config_clone.clone()))
                                }
                            }
                            else {
                                panic!("Impossible, config should still exist here (except if server deleted it)")
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
            let files_state = files_state.clone();
            spawn_local(async move {
                let files = (*files_state).clone();
                if files.len() > 0 {
                    for file in files {
                        let args = serde_wasm_bindgen::to_value(&FileArgs {test1:file.clone(), test2:format!("{}/db", proxima_state.chat_url.clone()), test3:proxima_state.auth_token.clone()}).unwrap();

                        let return_val = invoke("add_media_from_file_if_exists", args).await;
                        
                        let value =
                        return_val
                        .into_serde::<(String, String, MediaType)>();

                        if let Ok((hash, file_name, media_type)) = value {
                            let mut last_user = None;
                            for (i,part) in starting_context.get_parts().iter().enumerate() {
                                if let ContextPosition::User = part.get_position() {
                                    last_user = Some(i);
                                }
                            }
                            if let Some(i) = last_user {
                                starting_context.get_parts_mut()[i].add_data(ContextData::Media(hash.clone()));
                            }
                            else {
                                starting_context.add_part(ContextPart::new(vec![ContextData::Media(hash.clone())], ContextPosition::User));
                            }
                            start_chat.context = starting_context.clone();
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Media(hash.clone()), DatabaseItem::Media(Media {hash, media_type, file_name, tags:HashSet::new(), access_modes:HashSet::from([0]), added_at:Utc::now()}, Base64EncodedString::new(vec![])))]));
                        }
                    }
                    files_state.set(Vec::new());
                }
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
                db_state.dispatch(DatabaseAction::AddToOngoingChats { chat: local_id });

                let streaming = config_opt.as_ref().is_some_and(|conf| {conf.is_streaming()});

                let json_request = proxima_backend::web_payloads::AIPayload::new(proxima_state.auth_token.clone(), EndpointRequestVariant::RespondToFullPrompt { whole_context: starting_context, streaming, session_type: SessionType::Chat, chat_settings:config_opt, chat_id:Some(local_id), access_mode:db_state.cursors.chosen_access_mode });


                let value = make_ai_request(json_request, proxima_state.chat_url.clone(), local_id).await;
                db_state.dispatch(DatabaseAction::RemoveFromOngoingChats { chat: local_id });
                match value {
                    Ok(response) => {
                        
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
    let sort_callback = {
        let sort_state = sort_state.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            sort_state.set(sort_state.get_next());
        })
    };
    let sort_title = sort_state.get_title();
    let mut chat_refs = db_state.db.chats.get_chats().iter().collect::<Vec<(&ChatID, &Chat)>>();
    match *sort_state {
        SortingMode::None => (),
        SortingMode::AscendingTime => chat_refs.sort_by(|(_,chat1), (_,chat2)| {chat1.latest_message.cmp(&chat2.latest_message)}),
        SortingMode::DescendingTime => chat_refs.sort_by(|(_,chat1), (_,chat2)| {chat2.latest_message.cmp(&chat1.latest_message)}),
        SortingMode::AscendingID => chat_refs.sort_by(|(_,chat1), (_,chat2)| {chat1.id.cmp(&chat2.id)}),
        SortingMode::DescendingID => chat_refs.sort_by(|(_,chat1), (_,chat2)| {chat2.id.cmp(&chat1.id)}),
    };
    let chat_htmls = chat_refs.iter().map(|(id, chat)| {
        let db_state = db_state.clone();
        let callback = {
            let id_clone = **id;
            let db_state = db_state.clone();
            Callback::from(move |mouse_evt:MouseEvent| {
                let db_state2 = db_state.clone();
                db_state.dispatch(DatabaseAction::SetChat(Some(id_clone)));
                
            })
        };

        if !chat.access_modes.contains(&db_state.cursors.chosen_access_mode) {
            html!()
        }
        else if let Some(chosen_id) = db_state.cursors.chosen_chat && chosen_id == **id {
            
            html!(

                <div><button onclick={callback} class="chat-option chosen-chat text-left">{match chat.get_title() {Some(title) => shorten_title_to_x_chars(title.clone(), 20), None => format!("Chat {}", *id)}}</button></div>
            )
        }
        else {
            html!(
                <div><button onclick={callback} class="chat-option text-left">{match chat.get_title() {Some(title) => shorten_title_to_x_chars(title.clone(), 20), None => format!("Chat {}", *id)}}</button></div>
            )
        }
    }).collect::<Html>();

    let chosen_chat_by_id = db_state.db.chats.get_chats().get(&(db_state.cursors.chosen_chat.unwrap_or(1000000)));
    let config_htmls:Vec<Html> = db_state.db.configs.get_configs().iter().map(|(id, config)| {
        html!(
            <option value={config.name.clone()}>{config.name.clone()}</option>
        )
    }).collect();

    let cc_select_callback = {
        let select_node = cc_select_ref.clone();
        let db_state = db_state.clone();
        Callback::from(move |mouse_evt:Event| {
            let cc_name = select_node.cast::<web_sys::HtmlInputElement>()
            .unwrap()
            .value();
            match db_state.db.configs.get_configs().iter().find(|(i,config)| {
                &config.name == &cc_name
            }) {
                Some((id, config)) => {
                    let config_data = format!("{:?} {:?}", config.name.clone(), config.tools.is_some());
                    
                    db_state.dispatch(DatabaseAction::ChangeUsedChatConfig(Some(*id)));
                    spawn_local(async move {
                        print(config_data).await;
                    });
                },
                None => {
                    db_state.dispatch(DatabaseAction::ChangeUsedChatConfig(None));
                }
            }
            
        })
    };
    let media_htmls = files_state.iter().enumerate().map(|(id, chat)| {
        let files_state = files_state.clone();
        let callback = {
            let path = chat.clone();
            let id_clone = id;
            let files_state = files_state.clone();
            Callback::from(move |mouse_evt:MouseEvent| {
                let mut new_state = (*files_state).clone();
                new_state.remove(new_state.iter().enumerate().find(|(i, p)| {**p == path}).map_or(0, |(i, p)| {i}));
                files_state.set(new_state);
            })
        };
        html!(

            <div><button onclick={callback} class="chat-option chosen-chat text-left">{shorten_title_to_x_chars_from_end(chat.to_string_lossy().to_string(), 20) }</button></div>
        )
    }).collect::<Html>();
    let (disabled, button_style) = if let Some(chat_id) = db_state.cursors.chosen_chat {
        if db_state.ongoing_chats.contains(&chat_id) {
            (true, "mainapp-unused-button standard-padding-margin-corners")
        }
        else {
            (false, "mainapp-button standard-padding-margin-corners")
        }
    } else {
        (false, "mainapp-button standard-padding-margin-corners")
    };

    let ui_settings = if let Some(access_mode) = db_state.db.access_modes.get_modes().get(&db_state.cursors.chosen_access_mode) {
        ChatUISettings {
            hide_time_tool: if let Some(AMSetting::Bool(val)) = access_mode.am_settings.get(&"Hide time tool".to_string()) {
                *val
            }
            else {
                false
            }
        }
    }
    else {
        ChatUISettings { hide_time_tool: false }
    };

    html!{
        <div class="chat-part">
            <div class="standard-padding-margin-corners first-level vertical-flex max-height-of-container">
                <div>
                    <h1>{"Past chats"}</h1>
                    <div class="horizontal-flex">
                        <button class="mainapp-button most-horizontal-space standard-padding-margin-corners" onclick={new_chat_callback}>{"New chat"}</button>
                    </div>
                    <div class="horizontal-flex">
                        <button class="mainapp-button most-horizontal-space standard-padding-margin-corners" onclick={sort_callback}>{format!("Sort by : {sort_title}")}</button>
                    </div>

                    <hr/>
                </div>
                <div class="list-holder">
                    {
                        if db_state.db.chats.get_chats().len() > 0 {
                            chat_htmls
                        }
                        else {
                            html!({"No chats yet !"})
                        }
                    }
                </div>
                {
                    if files_state.len() == 0 {
                        html!()
                    }
                    else {
                        html!(
                            <div>
                                <h1>{"Media"}</h1>
                                <hr/>
                            </div>
                        )
                    }
                }
                <div class={if files_state.len() > 0 {"list-holder"} else {""}}>
                    {
                        media_htmls
                    }
                </div>
            </div>
            <div class="standard-padding-margin-corners first-level most-horizontal-space vertical-flex max-height-of-container" ref={file_ref}>
                <div>
                    {
                    match chosen_chat_by_id {
                        Some(chat) => html!(
                            <div class="chat-title-display">
                            <h1>
                            {
                                match &chat.chat_title {
                                    Some(title) => title.clone(),
                                    None => format!("Untitled Chat {}", chat.id),
                                }
                            }
                            </h1>
                            <button class="mainapp-button standard-padding-margin-corners align-right" onclick={chat_remove_callback}>{"Delete Chat"}</button>
                            </div>
                        ),
                        None => html!(<h1>{"Please select a chat or start one :)"}</h1>)
                    }}
                </div>
                <div class="list-holder">
                {
                    match chosen_chat_by_id {
                        Some(chat) => {
                            chat.context.get_parts().iter().enumerate().map(|(i, context_part)| {
                                if context_part.in_visible_position() {
                                    html!(
                                        <ContextPartShow context_part={context_part.clone()} context_part_index={i} chat_id={chat.get_id()} deletable={!db_state.ongoing_chats.contains(&chat.get_id())} ui_settings={ui_settings.clone()}/>
                                        
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
                    <button class={button_style} onclick={prompt_send_callback} disabled={disabled}>{"Send"}</button>
                    
                </div>

                
            </div>
            
        </div>
    }
}

fn shorten_title_to_x_chars(title:String, max_chars:usize) -> String {
    let mut out = String::with_capacity(max_chars + 3);
    let mut chars_in_out = 0;
    for char in title.chars() {
        if chars_in_out < max_chars {
            out.push(char);
            chars_in_out += 1;
        }
        else {
            out += "...";
            break;
        }
    }
    out
}

fn shorten_title_to_x_chars_from_end(title:String, max_chars:usize) -> String {
    let mut out = String::with_capacity(max_chars + 3);
    let mut chars_in_out = 0;
    for char in title.chars().rev() {
        if chars_in_out < max_chars {
            out.insert(0, char);
            chars_in_out += 1;
        }
        else {
            out = "...".to_string() + &out;
            break;
        }
    }
    out
}

#[derive(Clone, Properties, PartialEq)]
pub struct ContextPartProp {
    context_part:ContextPart,
    chat_id:ChatID,
    context_part_index:usize,
    deletable:bool,
    ui_settings:ChatUISettings
}

#[function_component(ContextPartShow)]
fn context_part(prop:&ContextPartProp) -> Html {

    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let delete_part_callback = {
        let db_state = db_state.clone();
        let proxima_state = proxima_state.clone();
        let prop = prop.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            let mut new_chat = db_state.db.chats.get_chats().get(&prop.chat_id).unwrap().clone();
            new_chat.context.get_parts_mut().remove(prop.context_part_index);
            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Chat(new_chat.get_id()), DatabaseItem::Chat(new_chat.clone()))]));
            let proxima_state = proxima_state.clone();
            spawn_local(async move {
                make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Update(DatabaseItem::Chat(new_chat)) }, proxima_state.chat_url.clone()).await;
            });
        })
    };
    let pos_add = html!(
        <>
        {
            match prop.context_part.get_position() {
                ContextPosition::User => "User",
                ContextPosition::Tool(_) => "Tool",
                ContextPosition::AI => "AI",
                ContextPosition::System => "System",
                ContextPosition::Total => "Total"
            }
        }
        </>
    );
    let (disabled, button_style) = if prop.deletable {
        (false, "mainapp-button standard-padding-margin-corners align-right")
    }
    else {
        (true, "mainapp-unused-button standard-padding-margin-corners align-right")
    };
    let part_title_add = 
        html!(
            <div class="chat-title-display">
                <div>{pos_add}</div>
                <div>{if let Some(date) = prop.context_part.get_date() {format!("{date}")} else {"".to_string()}}</div>
                <button class={button_style} disabled={disabled} onclick={delete_part_callback}>{"Delete part"}</button>
            </div>
        );
    let mut all_text = prop.context_part.data_to_single_text();
    if prop.context_part.is_user() {
        all_text = all_text.trim().to_string();
        all_text.remove_matches("<user_prompt>");
        all_text.remove_matches("</user_prompt>");
        let mut media = Vec::with_capacity(4);
        for data in prop.context_part.get_data() {
            if let ContextData::Media(hash) = data {
                media.push(html!(
                    <MediaPartShow hash={hash.clone()}/>
                ));
            }
        }
        html!(
            <div class="standard-padding-margin-corners">
            <>{part_title_add}</>
            <div> {VNode::from_html_unchecked(AttrValue::from(to_html(&all_text.lines().intersperse("\n\n").collect::<Vec<&str>>().concat())))}</div>
            <>{media}</>
            </div>
        )
    }
    else {
        let parsed = parse_html(&all_text.chars().collect::<Vec<char>>(), 2);
        if parsed.has_elements() {
            let mut htmls = Vec::with_capacity(parsed.children.len());
            for child in parsed.children {
                if let HtmlNode::Element { name, content, children } = child {
                    if name == "think" {
                        htmls.push(
                            html!(
                                <ThinkingPartShow txt={content} finished=true/>
                            )
                        );
                    }
                    else if name == "call" && children.children.len() >= 3 && let Some(HtmlNode::Element { name:child_name, content:child_content, children:tool_children }) = &children.get_first_element() && child_name.trim() == "tool" {
                        let child_content = child_content.clone();
                        htmls.push(
                            
                            html!(
                                <CallPartShow txt={content.clone()} tool_name={child_content.trim().to_string()} finished=true/>
                            )
                        );
                    }
                    else if name == "outputs" {
                        htmls.push(
                            html!(
                                <CallOutputPartShow txt={content}/>
                            )
                        );
                    }
                    else if name == "automatic_memory" {
                        htmls.push(
                            html!(
                                <MemoryPartShow txt={content}/>
                            )
                        );
                    }
                    else if name == "response" {
                        htmls.push(
                            html!(
                                <ResponsePartShow children={children.children}/>
                            )
                        );
                    }
                    else if name == "current_time" {
                        if !prop.ui_settings.hide_time_tool {
                            htmls.push(
                                html!(
                                    <>
                                    {"(Current time context addition)"}
                                    </>
                                )
                            )
                        }
                    }
                    else if content.trim().len() > 0 {
                        htmls.push(
                            html!(
                                <div>
                                <>{name}</>
                                <div>{VNode::from_html_unchecked(AttrValue::from(to_html(content.trim())))}</div>
                                </div>
                            )
                            
                        );
                    }
                }
                else if let HtmlNode::Text(txt) = child && txt.trim().len() > 0 {
                    htmls.push(
                        html!(
                            <div> 
                                <div>{VNode::from_html_unchecked(AttrValue::from(to_html(&txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat())))}</div>
                            </div>
                        )
                        
                    );
                }
            }
            if htmls.len() > 0 {
                html!(
                    <div class="standard-padding-margin-corners nonuser-turn">
                    <>{part_title_add}</>
                    <div>{htmls}</div>
                    </div>
                )
            }
            else {
                html!()
            }
        }
        else if all_text.contains("<think>") {
            html!(
                <div class="standard-padding-margin-corners nonuser-turn">
                <>{part_title_add}</>
                <ThinkingPartShow txt={all_text} finished=false/>
                </div>
            )
        } 
        else if all_text.trim().len() > 0 {

            html!(
                <div class="standard-padding-margin-corners nonuser-turn">
                <>{part_title_add}</>
                <div> {VNode::from_html_unchecked(AttrValue::from(to_html(all_text.trim())))}</div>
                </div>
            )
        }
        else {
            html!(

            )
        }
    }
}

#[derive(Properties, PartialEq)]
struct ThinkingPartProp {
    txt:String,
    finished:bool
}

#[derive(Clone, PartialEq)]
pub struct ChatUISettings {
    hide_time_tool:bool
}

#[function_component(ThinkingPartShow)]
fn thinking_part(prop:&ThinkingPartProp) -> Html {
    let should_show = use_state_eq(|| {false});
    let callback = {
        let should_show = should_show.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            if *should_show {
                should_show.set(false);
            }
            else {
                should_show.set(true);
            }
        })
    };
    let name = if prop.finished {
        if *should_show {
            format!("Thought process (click to hide)")
        }
        else {
            format!("Thought process (click to show)")
        }
    }
    else {
        if *should_show {
            format!("Thinking... (click to hide)")
        }
        else {
            format!("Thinking... (click to show)")
        }
    };
    if prop.txt.trim().len() > 0 {
        if *should_show {
            html!(
                <div>
                <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
                <div>{VNode::from_html_unchecked(AttrValue::from(to_html(prop.txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat().trim())))}</div>
                </div>
            )
        }
        else {
            html!(
                <div>
                <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
                </div>
            )
        }

    }
    else {
        html!(

        )
    }
}

#[derive(Properties, PartialEq)]
struct CallPartProp {
    txt:String,
    tool_name:String,
    finished:bool
}

#[function_component(CallPartShow)]
fn call_part(prop:&CallPartProp) -> Html {
    let should_show = use_state_eq(|| {false});
    let callback = {
        let should_show = should_show.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            if *should_show {
                should_show.set(false);
            }
            else {
                should_show.set(true);
            }
        })
    };
    let name = if prop.finished {
        if *should_show {
            format!("Tool call : {} (click to hide)", prop.tool_name)
        }
        else {format!("Tool call : {} (click to show)", prop.tool_name)
        }
    }
    else {
        if *should_show {
            format!("Calling tool : {}... (click to hide)", prop.tool_name)
        }
        else {
            format!("Calling tool : {}... (click to show)", prop.tool_name)
        }
    };
    if *should_show {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            <div>{VNode::from_html_unchecked(AttrValue::from(to_html(prop.txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat().trim())))}</div>
            </div>
        )
    }
    else {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            </div>
        )
    }
}

#[derive(Properties, PartialEq)]
struct CallOutputPartProp {
    txt:String,
}

#[function_component(CallOutputPartShow)]
fn call_output_part(prop:&CallOutputPartProp) -> Html {
    let should_show = use_state_eq(|| {false});
    let callback = {
        let should_show = should_show.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            if *should_show {
                should_show.set(false);
            }
            else {
                should_show.set(true);
            }
        })
    };
    let name = if *should_show {
        format!("Tool call outputs (click to hide)")
    }
    else {
        format!("Tool call outputs (click to show)")
    };
    if *should_show {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            <div>{VNode::from_html_unchecked(AttrValue::from(to_html(prop.txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat().trim())))}</div>
            </div>
        )
    }
    else {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            </div>
        )
    }
}

#[derive(Properties, PartialEq)]
struct ResponsePartProp {
    children:Vec<HtmlNode>,
}
#[function_component(ResponsePartShow)]
fn response_part(prop:&ResponsePartProp) -> Html {
    let mut final_htmls = Vec::with_capacity(prop.children.len());
    for child in &prop.children {
        if let HtmlNode::Element { name, content, children } = child && name == "think" {
            final_htmls.push(
                html!(
                    <ThinkingPartShow txt={content.clone()} finished=true/>
                )
            );
        }
        else if let HtmlNode::Text(txt) = child {
            final_htmls.push(html!(
                <div> 
                    <div>{VNode::from_html_unchecked(AttrValue::from(to_html(&txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat())))}</div>
                </div>
            ));
        }
    }
    html!(
        <>{final_htmls}</>
    )
}

#[derive(Properties, PartialEq)]
struct MemoryPartProp {
    txt:String,
}
#[function_component(MemoryPartShow)]
fn memory_part(prop:&MemoryPartProp) -> Html {
    let should_show = use_state_eq(|| {false});
    let callback = {
        let should_show = should_show.clone();
        Callback::from(move |mouse_evt:MouseEvent| {
            if *should_show {
                should_show.set(false);
            }
            else {
                should_show.set(true);
            }
        })
    };
    let name = if *should_show {
        format!("Automatic memory (click to hide)")
    }
    else {
        format!("Automatic memory (click to show)")
    };
    if *should_show {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            <div>{VNode::from_html_unchecked(AttrValue::from(to_html(prop.txt.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat().trim())))}</div>
            </div>
        )
    }
    else {
        html!(
            <div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
            </div>
        )
    }
}

#[derive(Properties, PartialEq)]
struct MediaPartProp {
    hash:String,
}

#[function_component(MediaPartShow)]
fn media_part(prop:&MediaPartProp) -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let media_data = use_state_eq(|| {Base64EncodedString::new(vec![])});
    let should_show = use_state_eq(|| {false});
    if let Some(media) = db_state.db.media.get_media(&prop.hash) {
        match media.media_type {
            MediaType::Text => {
                let hash = media.hash.clone();
                let media_data2 = media_data.clone();
                spawn_local(async move {
                    match make_db_request(DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Get(DatabaseItemID::Media(hash)) },proxima_state.chat_url.clone()).await {
                        Ok(DBResponse { reply:DatabaseReplyVariant::ReturnedItem(DatabaseItem::Media(_, data)) }) => media_data2.set(data),
                        _ => ()
                    }
                });
                let callback = {
                    let should_show = should_show.clone();
                    Callback::from(move |mouse_evt:MouseEvent| {
                        if *should_show {
                            should_show.set(false);
                        }
                        else {
                            should_show.set(true);
                        }
                    })
                };
                let name = if *should_show {
                    format!("File {} (click to hide)",  media.file_name.clone())
                }
                else {
                    format!("File {} (click to show)",  media.file_name.clone())
                };
                if *should_show {
                    let string = String::from_utf8(media_data.get_data()).unwrap();
                    html!(
                        <div>
                            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
                            <>{VNode::from_html_unchecked(AttrValue::from(to_html(string.trim().lines().intersperse("\n\n").collect::<Vec<&str>>().concat().trim())))}</>
                        </div>
                    )
                }
                else {
                    html!(
                        <div>
                            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{name}</button>
                        </div>
                    )
                }
                
            },
            MediaType::PDF => html!(<div>{format!("{}", media.file_name.clone())}</div>),
            _ => {
                let url = proxima_state.chat_url.clone();
                let full_url = format!("{url}/media/{}", media.file_name);
                html!(
                    <div>
                    <img src={full_url} class="hundred-p-width"/>
                    </div>
                )
            }
        }
        
    }
    else {
        html!(
            <>
            {"media not found"}
            </>
        )
    }
}