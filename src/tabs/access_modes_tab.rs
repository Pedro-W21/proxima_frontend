use std::collections::HashSet;

use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::access_modes::AccessMode;
use proxima_backend::database::chats::SessionType;
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, UseReducerHandle, function_component, html, use_context, use_node_ref};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request};
use crate::db_sync::{get_delta_for_add, get_next_id_for_category};



#[function_component(AccessModesTab)]
pub fn access_modes_tab() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");

    let am_name_ref = use_node_ref();

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
}