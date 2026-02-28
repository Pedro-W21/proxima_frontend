use markdown::to_html;
use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::chats::SessionType;
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::description::Description;
use proxima_backend::database::tags::NewTag;
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::database::notifications::Notification;
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::virtual_dom::VNode;
use yew::{AttrValue, Callback, Event, Html, MouseEvent, UseReducerHandle, function_component, html, use_context, use_node_ref};

use crate::app::{DatabaseAction, DatabaseState, PrintArgs, ProximaState, invoke, make_ai_request, make_db_request};
use crate::db_sync::get_delta_for_add;


#[function_component(TagsTab)]
pub fn tags_tab() -> Html {
    let tag_name_ref = use_node_ref();
    let tag_desc_ref = use_node_ref();
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");

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
            db_state.dispatch(DatabaseAction::SetModifiedTag(None));
            db_state.dispatch(DatabaseAction::SetParentTag(None));
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
}