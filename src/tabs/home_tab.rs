use proxima_backend::ai_interaction::endpoint_api::{EndpointRequestVariant, EndpointResponseVariant};
use proxima_backend::database::chats::SessionType;
use proxima_backend::database::context::{ContextData, ContextPart, ContextPosition, WholeContext};
use proxima_backend::database::{DatabaseItem, DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::database::notifications::Notification;
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::{Callback, Html, MouseEvent, UseReducerHandle, function_component, html, use_context, use_node_ref};
use yew::ContextProvider;

use crate::app::{DatabaseAction, DatabaseState, ProximaState, make_ai_request, make_db_request};
use crate::db_sync::get_delta_for_add;


#[function_component(HomeTab)]
pub fn home_tab() -> Html {


    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let prompt_node_ref = use_node_ref();
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
}