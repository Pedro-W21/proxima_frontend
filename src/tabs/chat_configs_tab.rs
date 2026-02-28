use std::collections::HashSet;

use chrono::Utc;
use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::ai_interaction::tools::{AgentToolData, ProximaTool, ProximaToolData, Tools};
use proxima_backend::database::access_modes::AccessMode;
use proxima_backend::database::chats::SessionType;
use proxima_backend::database::configuration::{ChatConfiguration, ChatSetting, RepeatPosition};
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, UseReducerHandle, function_component, html, use_context, use_node_ref, use_state};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request};
use crate::db_sync::{get_delta_for_add, get_next_id_for_category};

#[function_component(ChatConfigsTab)]
pub fn chat_configs_tab() -> Html {
    /* Sidebar with list picker from all possible settings, add button, and then the list of all current settings with removal buttons if necessary */
    /* The main area is for configuration of a single setting */


    let cc_name_ref = use_node_ref();
    let cc_setting_ref = use_node_ref();
    let cc_setting_value_ref = use_node_ref();
    let cc_second_setting_value_ref = use_node_ref();
    let cc_third_setting_value_ref = use_node_ref();


    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");

    let allocatable_agent_tools = use_state(|| HashSet::<ProximaTool>::new());

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
        let db_state = db_state.clone();
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
                    let access_modes_htmls:Vec<Html> = db_state.db.access_modes.get_modes().iter().enumerate().map(|(id, access_mode)| {
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
}