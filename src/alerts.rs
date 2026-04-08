use std::rc::Rc;

use chrono::{DateTime, Local};
use wasm_bindgen_futures::spawn_local;
use yew::{Callback, Html, MouseEvent, Properties, Reducible, UseReducerHandle, component, html, use_context};

use crate::app::print;

#[derive(Clone, PartialEq, Eq)]
pub enum AlertTab {
    App,
    Chats,
    Initialization
}

#[derive(Clone, PartialEq, Eq)]
pub enum AlertCategory {
    Frontend,
    Database,
    AIEndpoint
}

#[derive(Properties, Clone, PartialEq, Eq)]
pub struct AlertData {
    tab:AlertTab,
    category:AlertCategory,
    reason:String,
    time:DateTime<Local>
}

impl AlertData {
    pub fn new(tab:AlertTab, category:AlertCategory, reason:String) -> Self {
        Self { tab, category, reason, time: Local::now() }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Alerts {
    alerts:Vec<AlertData>,
}

impl Alerts {
    pub fn new() -> Self {
        Self { alerts: Vec::with_capacity(16) }
    }
}

pub enum AlertsAction {
    RemoveLast,
    AddAlert(AlertData),
}

impl Reducible for Alerts {
    type Action = AlertsAction;
    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        let mut new_self = self.alerts.clone();
        match action {
            AlertsAction::RemoveLast => {
                if new_self.len() > 0 {
                    new_self.remove(new_self.len() - 1);
                }
            },
            AlertsAction::AddAlert(data) => {
                new_self.insert(0, data);
            }
        }
        Rc::new(Alerts { alerts: new_self })
    }
}

#[component(Alert)]
fn alert_comp(prop:&AlertData) -> Html {

    let alerts_state = use_context::<UseReducerHandle<Alerts>>().expect("no ctx found");
    
    let callback = {
        Callback::from(move |mouse_evt:MouseEvent| {
            alerts_state.dispatch(AlertsAction::RemoveLast);
        })
    };

    let tab = match prop.tab {
        AlertTab::App => format!("Application"),
        AlertTab::Chats => format!("Chats"),
        AlertTab::Initialization => format!("Initialization")
    };
    let category = match prop.category {
        AlertCategory::Frontend => format!("Frontend"),
        AlertCategory::Database => format!("Database"),
        AlertCategory::AIEndpoint => format!("AI Endpoint")
    };
    let header = format!("Alert : {tab}/{category}");
    html!(
        <div class="dialog-on-top first-level">
            <h3>{header}</h3>
            <p>{prop.reason.clone()}</p>
            <p>{format!("At : {}", prop.time.clone())}</p>
            <button class="mainapp-button standard-padding-margin-corners" onclick={callback}>{"Delete"}</button>
        </div>
    )
}


#[component(AlertsShow)]
pub fn alert_comp() -> Html {
    let alerts_state = use_context::<UseReducerHandle<Alerts>>().expect("no ctx found");
    if alerts_state.alerts.len() > 0 {
        spawn_local(async move {
            print("AAAAAAA").await;
        });
        let alert_htmls = alerts_state.alerts.iter().map(|alert| {
            html!(
                <Alert tab={alert.tab.clone()} category={alert.category.clone()} time={alert.time.clone()} reason={alert.reason.clone()}/>
            )
        }).collect::<Html>();
        alert_htmls
    }
    else {
        html!(

        )
    }
}