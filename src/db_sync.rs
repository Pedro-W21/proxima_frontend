use std::collections::HashSet;

use proxima_backend::{database::{DatabaseInfoReply, DatabaseInfoRequest, DatabaseItem, DatabaseItemID, DatabaseReplyVariant, DatabaseRequestVariant, ProxDatabase, configuration::ChatSetting}, web_payloads::{DBPayload, DBResponse}};

pub enum NeedToSync {
    Everything,
    RetrieveThoseItems(Vec<DatabaseItemID>),
    UpdateThoseItems(Vec<(DatabaseItemID, DatabaseItem)>),
    AddThoseItemsAndMoveTheRest(Vec<(DatabaseItemID, DatabaseItem)>)
}

pub fn get_database_conflict_fix<F:Fn(DatabaseRequestVariant) -> Result<DatabaseReplyVariant, ()>>(client_db:ProxDatabase, request_func:&F) -> Result<Vec<NeedToSync>, ()> {
    let mut syncs = Vec::with_capacity(32);
    match request_func(DatabaseRequestVariant::Info(DatabaseInfoRequest::NumbersOfItems))? {
        DatabaseReplyVariant::Info(DatabaseInfoReply::NumbersOfItems { devices, chats, folders, files, tags, access_modes }) => {
            
        },
        _ => return Err(())
    }
    Ok(syncs)
}

#[derive(Clone, PartialEq, Eq)]
pub struct UserCursors {
    pub chosen_tab:usize,
    pub chosen_chat:Option<usize>,
    pub chosen_access_mode:usize,
    pub access_mode_for_modification:Option<usize>,
    pub chosen_tag:Option<usize>,
    pub chosen_tags:HashSet<usize>,
    pub chosen_parent_tag:Option<usize>,
    pub chosen_config:Option<usize>,
    pub config_for_modification:Option<usize>,
    pub chosen_setting:Option<usize>,
    pub setting_for_modification:Option<ChatSetting>,
    pub chosen_access_mode_tags:HashSet<usize>,
}

impl UserCursors {
    pub fn zero() -> Self {
        Self { chosen_tab:0, chosen_chat: None, chosen_access_mode: 0, access_mode_for_modification: None, chosen_tag: None, chosen_parent_tag: None, chosen_access_mode_tags: HashSet::new(), chosen_config:None, config_for_modification:None, chosen_setting:None, setting_for_modification:None, chosen_tags:HashSet::new() }
    }
    pub fn from_state(
        chosen_tab:usize,
        chosen_chat:Option<usize>,
        chosen_access_mode:usize,
        access_mode_for_modification:Option<usize>,
        chosen_tag:Option<usize>,
        chosen_parent_tag:Option<usize>,
        chosen_config:Option<usize>,
        chosen_setting:Option<usize>,
        config_for_modification:Option<usize>,
        chosen_access_mode_tags:HashSet<usize>
    ) -> Self {
        Self { chosen_tab, chosen_chat, chosen_access_mode, access_mode_for_modification, chosen_tag, chosen_parent_tag, chosen_access_mode_tags, chosen_config, config_for_modification, chosen_setting, setting_for_modification:None, chosen_tags:HashSet::new() }
    }
}

pub fn apply_server_updates(client_db: &mut ProxDatabase, updates:Vec<(DatabaseItemID, DatabaseItem)>, cursors:UserCursors) -> UserCursors {
    let mut new_cursors = cursors.clone();
    for (id, new_item) in updates {
        match new_item {
            DatabaseItem::AccessMode(access_mode) => {
                if access_mode.get_id() >= client_db.access_modes.get_modes().len() {
                    client_db.access_modes.add_mode(access_mode);
                }
                else {
                    let id = access_mode.get_id();
                    if !client_db.insert_or_update(DatabaseItem::AccessMode(access_mode)) {
                        match cursors.access_mode_for_modification {
                            Some(am) => if am >= id {
                                new_cursors.access_mode_for_modification = Some(am + 1);
                            },
                            None => ()
                        }
                        if cursors.chosen_access_mode >= id {
                            new_cursors.chosen_access_mode += 1;
                        }
                    }
                    
                }
            },
            DatabaseItem::Chat(chat) => {
                if chat.get_id() >= client_db.chats.get_chats().len() {
                    client_db.chats.add_chat_raw(chat);
                }
                else {
                    let id = chat.get_id();
                    if !client_db.insert_or_update(DatabaseItem::Chat(chat)) {
                        match cursors.chosen_chat {
                            Some(chat_id) => if chat_id >= id {
                                new_cursors.chosen_chat = Some(chat_id + 1);
                            },
                            None => ()
                        }
                    }
                   
                }
            },
            DatabaseItem::Device(device) => {
                if device.get_id() >= client_db.devices.get_devices().len() {
                    client_db.devices.add_device(device);
                }
                else {
                    client_db.insert_or_update(DatabaseItem::Device(device));
                }
            },
            DatabaseItem::File(file) => {
                if file.get_id() >= client_db.files.len() {
                    client_db.files.add_file_raw(file);
                }
                else {
                    client_db.insert_or_update(DatabaseItem::File(file));
                }
            },
            DatabaseItem::Folder(folder) => {
                if folder.get_id() >= client_db.folders.number_of_folders() {
                    client_db.folders.add_folder_raw(folder);
                }
                else {
                    client_db.insert_or_update(DatabaseItem::Folder(folder));
                }
            },
            DatabaseItem::Tag(tag) => {
                if tag.get_id() >= client_db.tags.get_tags().len() {
                    client_db.tags.add_tag_raw(tag);
                }
                else {
                    let id = tag.get_id();
                    if !client_db.insert_or_update(DatabaseItem::Tag(tag)) {
                        match cursors.chosen_tag {
                            Some(tag_id) => if tag_id >= id {
                                new_cursors.chosen_tag = Some(tag_id + 1);
                            }
                            None => ()
                        }
                        match cursors.chosen_parent_tag {
                            Some(tag_id) => if tag_id >= id {
                                new_cursors.chosen_parent_tag = Some(tag_id + 1);
                            }
                            None => ()
                        }
                        let mut new_set = HashSet::new();
                        for tag_id in cursors.chosen_access_mode_tags.iter() {
                            if *tag_id >= id {
                                new_set.insert(*tag_id + 1);
                            }
                            else {
                                new_set.insert(*tag_id);
                            }
                        }
                        new_cursors.chosen_access_mode_tags = new_set;
                    }
                    
                }
            },
            DatabaseItem::ChatConfig(config) => {
                if config.id >= client_db.configs.get_configs().len() {
                    client_db.configs.add_config(config);
                }
                else {
                    let id = config.id;
                    if !client_db.insert_or_update(DatabaseItem::ChatConfig(config)) {
                        match cursors.chosen_config {
                            Some(config_id) => if config_id >= id {
                                new_cursors.chosen_config = Some(config_id + 1);
                            },
                            None => ()
                        }
                        match cursors.config_for_modification {
                            Some(config_id) => if config_id >= id {
                                new_cursors.config_for_modification = Some(config_id + 1);
                            },
                            None => ()
                        }
                    }
                }
            },
            DatabaseItem::UserData(user_data) => {
                client_db.personal_info.user_data = user_data
            },
        }
    }
    new_cursors
}

pub fn get_next_id_for_category(db:&ProxDatabase, category:&DatabaseItem) -> DatabaseItemID {
    match category {
        DatabaseItem::AccessMode(_) => DatabaseItemID::AccessMode(db.access_modes.get_modes().len()),
        DatabaseItem::Chat(_) => DatabaseItemID::Chat(db.chats.get_chats().len()),
        DatabaseItem::ChatConfig(_) => DatabaseItemID::ChatConfiguration(db.configs.get_configs().len()),
        DatabaseItem::Device(_) => DatabaseItemID::Device(db.devices.get_devices().len()),
        DatabaseItem::File(_) => DatabaseItemID::File(db.files.len()),
        DatabaseItem::Folder(_) => DatabaseItemID::Folder(db.folders.number_of_folders()),
        DatabaseItem::Tag(_) => DatabaseItemID::Tag(db.tags.get_tags().len()),
        DatabaseItem::UserData(_) => DatabaseItemID::UserData
    }
}
pub async fn get_delta_for_add<F:AsyncFn(DatabaseRequestVariant) -> Result<DatabaseReplyVariant, ()>>(local_given_id:DatabaseItemID, mut added_item:DatabaseItem, request_func:F) -> (Vec<(DatabaseItemID, DatabaseItem)>, DatabaseItemID, DatabaseItem) {
    added_item.set_id(local_given_id.clone());
    let mut new_id = local_given_id.clone();
    let mut delta = Vec::with_capacity(2);

    if let Ok(reply) = request_func(DatabaseRequestVariant::Add(added_item.clone())).await {
        match reply {
            DatabaseReplyVariant::AddedItem(id) => if local_given_id != id {
                added_item.set_id(id.clone());
                for i in local_given_id.clone()..id.clone() {
                    match request_func(DatabaseRequestVariant::Get(i.clone())).await {
                        Ok(reply) => match reply {
                            DatabaseReplyVariant::ReturnedItem(item) => delta.push((i, item)),
                            _ => delta.push((i, added_item.clone()))
                        },
                        Err(()) => delta.push((i, added_item.clone()))
                    }
                }
                new_id = id;
            },
            _ => ()
        }
    }
    (delta, new_id, added_item)
}

pub fn handle_add_reducible(client_db: &mut ProxDatabase, local_given_id:DatabaseItemID, remote_id:DatabaseItemID, added_item:DatabaseItem, cursors:UserCursors, delta:Vec<(DatabaseItemID, DatabaseItem)>) -> UserCursors {
    let mut new_cursors = cursors.clone();
    new_cursors = apply_server_updates(client_db, delta, new_cursors);
    new_cursors = apply_server_updates(client_db, vec![(remote_id.clone(), added_item)], new_cursors);
    match local_given_id.clone() {
        DatabaseItemID::AccessMode(local_id) => match remote_id {
            DatabaseItemID::AccessMode(remote_id) => {
                match cursors.access_mode_for_modification {
                    Some(am) => if am == local_id {
                        new_cursors.access_mode_for_modification = Some(remote_id);
                    }
                    None => ()
                }
                if cursors.chosen_access_mode == local_id {
                    new_cursors.chosen_access_mode = remote_id;
                }
            },
            _ => panic!("Wrong kind of id")
        },
        DatabaseItemID::Chat(local_id) => match remote_id {
            DatabaseItemID::Chat(remote_id) => {
                match cursors.chosen_chat {
                    Some(chat) => if chat == local_id {
                        new_cursors.chosen_chat = Some(remote_id)
                    },
                    None => ()   
                }
            },
            _ => panic!("Wrong kind of id")
        },
        DatabaseItemID::Tag(local_id) => match remote_id {
            DatabaseItemID::Tag(remote_id_m) => {
                match cursors.chosen_parent_tag {
                    Some(tag) => if tag == local_id {
                        new_cursors.chosen_parent_tag = Some(remote_id_m);
                    },
                    None => ()
                }
                match cursors.chosen_tag {
                    Some(tag) => if tag == local_id {
                        new_cursors.chosen_tag = Some(remote_id_m);
                    },
                    None => ()
                }
                if new_cursors.chosen_access_mode_tags.remove(&local_id) {
                    new_cursors.chosen_access_mode_tags.insert(remote_id_m);
                }
            },
            _ => panic!("Wrong kind of id")
        },
        _ => ()
    }
    new_cursors
}

pub async fn handle_add<F:AsyncFn(DatabaseRequestVariant) -> Result<DatabaseReplyVariant, ()>>(client_db: &mut ProxDatabase, local_given_id:DatabaseItemID, mut added_item:DatabaseItem, db_response:DatabaseReplyVariant, cursors:UserCursors, request_func:F) -> (UserCursors, DatabaseItemID) {
    added_item.set_id(local_given_id.clone());
    let mut new_cursors = cursors.clone();
    let mut new_id = local_given_id.clone();
    match db_response {
        DatabaseReplyVariant::AddedItem(id) => if local_given_id != id {
            for i in local_given_id.clone()..id.clone() {
                match request_func(DatabaseRequestVariant::Get(i)).await {
                    Ok(reply) => match reply {
                        DatabaseReplyVariant::ReturnedItem(item) => client_db.insert_directly(item),
                        _ => client_db.insert_directly(added_item.clone())
                    },
                    Err(()) => client_db.insert_directly(added_item.clone())
                }

            }
            match local_given_id.clone() {
                DatabaseItemID::AccessMode(local_id) => match id {
                    DatabaseItemID::AccessMode(remote_id) => {
                        match cursors.access_mode_for_modification {
                            Some(am) => if am == local_id {
                                new_cursors.access_mode_for_modification = Some(remote_id);
                            }
                            None => ()
                        }
                        if cursors.chosen_access_mode == local_id {
                            new_cursors.chosen_access_mode = remote_id;
                        }
                    },
                    _ => panic!("Wrong kind of id")
                },
                DatabaseItemID::Chat(local_id) => match id {
                    DatabaseItemID::Chat(remote_id) => {
                        match cursors.chosen_chat {
                            Some(chat) => if chat == local_id {
                                new_cursors.chosen_chat = Some(remote_id)
                            },
                            None => ()   
                        }
                    },
                    _ => panic!("Wrong kind of id")
                },
                DatabaseItemID::Tag(local_id) => match id {
                    DatabaseItemID::Tag(remote_id) => {
                        match cursors.chosen_parent_tag {
                            Some(tag) => if tag == local_id {
                                new_cursors.chosen_parent_tag = Some(remote_id);
                            },
                            None => ()
                        }
                        match cursors.chosen_tag {
                            Some(tag) => if tag == local_id {
                                new_cursors.chosen_tag = Some(remote_id);
                            },
                            None => ()
                        }
                        if new_cursors.chosen_access_mode_tags.remove(&local_id) {
                            new_cursors.chosen_access_mode_tags.insert(remote_id);
                        }
                    },
                    _ => panic!("Wrong kind of id")
                },
                _ => ()
            }
            new_id = id;
        },
        _ => ()
    }
    (new_cursors, new_id)
}