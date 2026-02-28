use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::chats::SessionType;
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, UseReducerHandle, function_component, html, use_context, use_node_ref};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request};
use crate::db_sync::get_delta_for_add;


#[function_component(ChatTab)]

pub fn chat_tab() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let prompt_node_ref = use_node_ref();
    let cc_select_ref = use_node_ref();

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
    let config_htmls:Vec<Html> = db_state.db.configs.get_configs().iter().enumerate().map(|(id, config)| {
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