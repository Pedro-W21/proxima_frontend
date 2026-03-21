use std::collections::HashSet;
use std::path::PathBuf;

use chrono::Utc;
use futures::StreamExt;
use gloo_events::EventListener;
use gloo_utils::format::JsValueSerdeExt;
use html_parser::{Dom, Node};
use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::chats::{ChatID, SessionType};
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::media::{Base64EncodedString, Media, MediaType};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::web_payloads::DBPayload;
use serde::{Deserialize, Serialize};
use tauri_sys::dpi::PhysicalPosition;
use tauri_sys::window::DragDropEvent;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlElement;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, Properties, UseReducerHandle, function_component, html, use_context, use_effect_with, use_node_ref, use_state_eq};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request};
use crate::db_sync::get_delta_for_add;

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
#[function_component(ChatTab)]

pub fn chat_tab() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let prompt_node_ref = use_node_ref();
    let cc_select_ref = use_node_ref();
    let file_ref = use_node_ref();
    let files_state = use_state_eq(HashSet::<PathBuf>::default);

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

                        let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("STARTED LISTENING FOR DRAG & DROPS")}).unwrap();
                        invoke("print_to_console", args).await;
                        let (mut listener, mut abort_handle) = futures::stream::abortable(listener);
                        while let Some(raw_event) = listener.next().await {
                            let mut current_files = (*files_state).clone();
                            let mut new = false;
                            for path in &raw_event.payload.paths {
                                new = new || current_files.insert(path.clone());
                            }
                            if new {
                                files_state.set(current_files);
                                let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("RECEIVED FILE YAHOOO : {}", raw_event.payload.paths.iter().map(|path| {format!("{}", path.to_string_lossy())}).collect::<Vec<String>>().concat() )}).unwrap();
                                invoke("print_to_console", args).await;
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
                        .into_serde::<(String, String)>();

                        if let Ok((hash, file_name)) = value {
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
                            db_state.dispatch(DatabaseAction::ApplyUpdates(vec![(DatabaseItemID::Media(hash.clone()), DatabaseItem::Media(Media {hash, media_type:MediaType::Image, file_name, tags:HashSet::new(), access_modes:HashSet::from([0]), added_at:Utc::now()}, Base64EncodedString::new(vec![])))]));
                        }
                    }
                    files_state.set(HashSet::new());
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

                let streaming = true;

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
    let chat_htmls = db_state.db.chats.get_chats().iter().map(|(id, chat)| {
        let db_state = db_state.clone();
        let callback = {
            let id_clone = *id;
            let db_state = db_state.clone();
            Callback::from(move |mouse_evt:MouseEvent| {
                let db_state2 = db_state.clone();
                spawn_local(async move {
                    let args = serde_wasm_bindgen::to_value(&PrintArgs {value:format!("SELECTING CHAT {id_clone} FROM {:?}", db_state2.cursors.chosen_chat)}).unwrap();
                    invoke("print_to_console", args).await;
                });
                db_state.dispatch(DatabaseAction::SetChat(Some(id_clone)));
                
            })
        };

        if !chat.access_modes.contains(&db_state.cursors.chosen_access_mode) {
            html!()
        }
        else if let Some(chosen_id) = db_state.cursors.chosen_chat && chosen_id == *id {
            
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
    let media_htmls = files_state.iter().enumerate().map(|(id, chat)| {
        let files_state = files_state.clone();
        let callback = {
            let path = chat.clone();
            let id_clone = id;
            let files_state = files_state.clone();
            Callback::from(move |mouse_evt:MouseEvent| {
                let mut new_state = (*files_state).clone();
                new_state.remove(&path);
                files_state.set(new_state);
            })
        };
        html!(

            <div><button onclick={callback} class="chat-option chosen-chat text-left">{shorten_title_to_x_chars_from_end(chat.to_string_lossy().to_string(), 20) }</button></div>
        )
    }).collect::<Html>();
    html!{
        <div class="chat-part">
            <div class="standard-padding-margin-corners first-level vertical-flex max-height-of-container">
                <div>
                    <h1>{"Past chats"}</h1>
                    <button class="mainapp-button most-horizontal-space-no-flex standard-padding-margin-corners" onclick={new_chat_callback}>{"New chat"}</button>
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
                <div class="list-holder">
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
                                        <ContextPartShow context_part={context_part.clone()} context_part_index={i} chat_id={chat.get_id()} deletable={!db_state.ongoing_chats.contains(&chat.get_id())}/>
                                        
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
    deletable:bool
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
    let part_title_add = if prop.deletable {
        html!(
            <div class="chat-title-display">
                <>{pos_add}</>
                <button class="mainapp-button standard-padding-margin-corners align-right" onclick={delete_part_callback}>{"Delete part"}</button>
            </div>
        )
    }
    else {
        html!(
            <div class="chat-title-display">
                {pos_add}
            </div>
        )
    };
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
        match Dom::parse(&all_text) {
            Ok(dom) => {
                let mut htmls = Vec::with_capacity(dom.children.len());
                for child in dom.children {
                    if let Some(elem) = child.element() {
                        if elem.name == "think" {
                            htmls.push(
                                html!(
                                    <ThinkingPartShow txt={elem.source_span.text.clone()} finished=true/>
                                )
                            );
                        }
                        else if elem.name == "call" && elem.children.len() >= 3 && let Some(tool_child) = elem.children[0].element() && tool_child.children.len() > 0 && let Some(tool_name) = tool_child.children[0].text() {
                            htmls.push(
                                html!(
                                    <CallPartShow txt={elem.source_span.text.clone()} tool_name={tool_name.to_string()} finished=true/>
                                )
                            );
                        }
                        else if elem.name == "outputs" {
                            htmls.push(
                                html!(
                                    <CallOutputPartShow txt={elem.source_span.text.clone()}/>
                                )
                            );
                        }
                        else if elem.name == "automatic_memory" {
                            htmls.push(
                                html!(
                                    <MemoryPartShow txt={elem.source_span.text.clone()}/>
                                )
                            );
                        }
                        else if elem.name == "response" {
                            htmls.push(
                                html!(
                                    <ResponsePartShow children={elem.children.clone()}/>
                                )
                            );
                        }
                        else if elem.source_span.text.trim().len() > 0 {
                            htmls.push(
                                html!(
                                    <div>
                                    <div>{VNode::from_html_unchecked(AttrValue::from(to_html(elem.source_span.text.trim())))}</div>
                                    </div>
                                )
                                
                            );
                        }
                    }
                    else if let Some(txt) = child.text() && txt.trim().len() > 0 {
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
            },
            Err(_) => if all_text.contains("<think>") {
                html!(
                    <div class="standard-padding-margin-corners nonuser-turn">
                    <>{part_title_add}</>
                    <ThinkingPartShow txt={all_text} finished=false/>
                    </div>
                )
            } 
            else {

                html!(
                    <div class="standard-padding-margin-corners nonuser-turn">
                    <>{part_title_add}</>
                    <div> {VNode::from_html_unchecked(AttrValue::from(to_html(all_text.trim())))}</div>
                    </div>
                )
            }
        }
    }
}

#[derive(Properties, PartialEq)]
struct ThinkingPartProp {
    txt:String,
    finished:bool
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
    children:Vec<Node>,
}
#[function_component(ResponsePartShow)]
fn response_part(prop:&ResponsePartProp) -> Html {
    let mut final_htmls = Vec::with_capacity(prop.children.len());
    for child in &prop.children {
        if let Some(elem) = child.element() && elem.name == "think" {
            final_htmls.push(
                html!(
                    <ThinkingPartShow txt={elem.source_span.text.clone()} finished=true/>
                )
            );
        }
        else if let Some(txt) = child.text() {
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
    if let Some(media) = db_state.db.media.get_media(&prop.hash) {
        match media.media_type {
            MediaType::Text => html!(<div>{format!("{}", media.file_name.clone())}</div>),
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