use proxima_backend::database::{DatabaseItemID, DatabaseRequestVariant};
use proxima_backend::database::notifications::Notification;
use proxima_backend::web_payloads::DBPayload;
use wasm_bindgen_futures::spawn_local;
use yew::{Callback, Html, MouseEvent, UseReducerHandle, function_component, html, use_context};
use yew::ContextProvider;

use crate::app::{DatabaseAction, DatabaseState, ProximaState, make_db_request};

#[function_component(NotificationTab)]
pub fn notification_tab() -> Html {

    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");

    let notification_htmls = db_state.db.notifications.get_notifications().iter().filter_map(|(id, notification)| {
        let notification = notification.clone();
        if notification.access_modes.contains(&db_state.cursors.chosen_access_mode) {
            Some(
                html!(
                    <ContextProvider<Notification> context={notification.clone()}>
                        <SingleNotification/>
                    </ContextProvider<Notification>>
                )
            )
        }
        else {
            None
        }
    }).collect::<Html>();

    html!(
        <div class="chat-part">
            <div class="all-vertical-space standard-padding-margin-corners first-level most-horizontal-space-no-flex">
                <div class="list-plus-other-col">
                    <div>
                        <h1>{"Current notifications"}</h1>
                        <hr/>
                    </div>
                    <div class="list-holder all-vertical-space-flex">
                        {notification_htmls}
                    </div>
                </div>
            </div>
        </div>
    )
}

#[function_component(SingleNotification)]
pub fn single_notification() -> Html {
    let proxima_state = use_context::<UseReducerHandle<ProximaState>>().expect("no ctx found");
    let db_state = use_context::<UseReducerHandle<DatabaseState>>().expect("no ctx found");
    let my_notification = use_context::<Notification>().expect("no ctx found");

    let specific = match my_notification.related_item {
        Some(item_id) => match item_id {
            DatabaseItemID::Chat(chat_id) => {
                let goto_callback = {
                    let db_state = db_state.clone();
                    let proxima_state = proxima_state.clone();
                    let notification = my_notification.clone();
                    Callback::from(move |mouse_evt:MouseEvent| {
                        db_state.dispatch(DatabaseAction::SetTab(1));
                        db_state.dispatch(DatabaseAction::SetChat(Some(chat_id)));
                    })
                };
                html!(
                    <div class="label-input-combo most-horizontal-space third-level"> 
                        <p class="standard-padding-margin-corners">{format!("Chat {} updated", chat_id)}</p>
                        <button class="mainapp-button standard-padding-margin-corners" onclick={goto_callback}>{"Go to chat"}</button>
                    </div>
                )
            },
            _ => html!()
        },
        None => html!()
    };

    let delete_callback = {
        let db_state = db_state.clone();
        let proxima_state = proxima_state.clone();
        let notification = my_notification.clone();
        Callback::from(move |mouse_evt:MouseEvent| {

            let db_state = db_state.clone();
            let proxima_state = proxima_state.clone();
            let notification = notification.clone();
            spawn_local(async move {
                let json_request = DBPayload { auth_key: proxima_state.auth_token.clone(), request: DatabaseRequestVariant::Remove(DatabaseItemID::Notification(notification.id)) };
                match make_db_request(json_request, proxima_state.chat_url.clone()).await {
                    Ok(response) => {
                        db_state.dispatch(DatabaseAction::RemoveItem(DatabaseItemID::Notification(notification.id)));
                    },
                    Err(()) => ()
                }
            });
        })
    };

    html!(
        <div class="label-input-combo most-horizontal-space third-level standard-padding-margin-corners">
            <p class="standard-padding-margin-corners">{format!("{}", my_notification.timestamp)}</p>
            <div>{specific}</div>
            <button class="mainapp-button standard-padding-margin-corners" onclick={delete_callback}>{"Delete notification"}</button>
            
        </div>
    )
}