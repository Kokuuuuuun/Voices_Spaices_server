use socketioxide::extract::{SocketRef, Data, State};
use socketioxide::socket::DisconnectReason;
use crate::state::AppState;
use crate::types::{User, ChatMessage, DrawData, SignalPayload};
use serde_json::json;

pub async fn on_connect(socket: SocketRef, state: State<AppState>) {
    println!("User connected: {}", socket.id);

    // Helper to broadcast active rooms
    let broadcast_active_rooms = |state: &AppState, socket: &SocketRef| {
        let mut active_rooms = Vec::new();
        for room in state.rooms.iter() {
            if !room.users.is_empty() {
                active_rooms.push(json!({
                    "id": room.id,
                    "name": room.name,
                    "userCount": room.users.len(),
                    "users": room.users.iter().map(|u| json!({ "name": u.name, "color": u.color })).collect::<Vec<_>>()
                }));
            }
        }
        let _ = socket.broadcast().emit("active_rooms", active_rooms.clone()); // Notify others
        let _ = socket.emit("active_rooms", active_rooms); // Notify self
    };

    // Initial Active Rooms
    broadcast_active_rooms(&state, &socket);

    socket.on("join_room", |socket: SocketRef, Data::<(String, String)>(data), state: State<AppState>| {
        let (room_id, name) = data;
        
        socket.join(room_id.clone());

        let new_user = User {
            id: socket.id.to_string(),
            name,
            color: format!("#{:06x}", rand::random::<u32>() & 0xFFFFFF),
            x: rand::random::<f64>() * 800.0,
            y: rand::random::<f64>() * 600.0,
            room_id: room_id.clone(),
        };

        // Add to state
        state.add_user_to_room(room_id.clone(), new_user.clone());
        
        // Emit room state to user
        if let Some(room) = state.get_room(&room_id) {
            let _ = socket.emit("room_state", room);
        }

        // Notify others
        let _ = socket.to(room_id.clone()).emit("user_joined", new_user.clone());

        // Broadcast active rooms update
        let mut active_rooms = Vec::new();
        for room in state.rooms.iter() {
            if !room.users.is_empty() {
                active_rooms.push(json!({
                    "id": room.id,
                    "name": room.name,
                    "userCount": room.users.len(),
                    "users": room.users.iter().map(|u| json!({ "name": u.name, "color": u.color })).collect::<Vec<_>>()
                }));
            }
        }
        let _ = socket.broadcast().emit("active_rooms", active_rooms);

        // WebRTC: Send existing participants
        if let Some(room) = state.get_room(&room_id) { 
            let others: Vec<String> = room.users.iter()
                .filter(|u| u.id != socket.id.to_string())
                .map(|u| u.id.clone())
                .collect();
            let _ = socket.emit("existing_participants", json!(vec![others]));
        }

        // Send chat history
        let db = state.db.clone();
        let rid = room_id.clone();
        let socket_clone = socket.clone();
        tokio::spawn(async move {
            if let Ok(messages) = db.get_messages(&rid).await {
                for msg in messages {
                    let _ = socket_clone.emit("chat_message", msg);
                }
            }
        });
    });

    socket.on("leave_room", |socket: SocketRef, Data::<String>(room_id), state: State<AppState>| {
        state.remove_user(&socket.id.to_string());
        let _ = socket.leave(room_id.clone());
        let _ = socket.to(room_id).emit("user_left", socket.id.to_string());
        
        // Broadcast active rooms update
        let mut active_rooms = Vec::new();
        for room in state.rooms.iter() {
            if !room.users.is_empty() {
                active_rooms.push(json!({
                    "id": room.id,
                    "name": room.name,
                    "userCount": room.users.len(),
                    "users": room.users.iter().map(|u| json!({ "name": u.name, "color": u.color })).collect::<Vec<_>>()
                }));
            }
        }
        let _ = socket.broadcast().emit("active_rooms", active_rooms);
    });

    socket.on("move", |socket: SocketRef, Data::<(f64, f64)>(data), state: State<AppState>| async move {
        let (x, y) = data;
        let user_id = socket.id.to_string();
        if let Some(room_id) = state.get_user_room(&user_id) { 
            state.update_user_position(&user_id, x, y); 
            let _ = socket.to(room_id.clone()).emit("user_moved", (user_id.clone(), x, y));
            
            if let Some(user) = state.get_user(&room_id, &user_id) { 
                 let db = state.db.clone();
                 let room_id_clone = room_id.clone();
                 tokio::spawn(async move {
                     if let Err(e) = db.save_user(&user, &room_id_clone).await {
                         eprintln!("Failed to save user move: {}", e);
                     }
                 });
            }
        }
    });

    socket.on("send_chat", |socket: SocketRef, Data::<(String, String)>(data), state: State<AppState>| {
        let (room_id, text) = data;
        let user_id = socket.id.to_string();
        if let Some(user) = state.get_user(&room_id, &user_id) {
            let msg = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                user_id: user.id.clone(),
                user_name: user.name.clone(),
                text,
                timestamp: chrono::Utc::now().timestamp_millis(),
            };
            
            let db = state.db.clone();
            let rid = room_id.clone();
            let msg_clone = msg.clone();
            tokio::spawn(async move {
                let _ = db.save_message(&msg_clone, &rid).await;
            });

            // Emit to all in room including sender using within()
            let _ = socket.within(room_id).emit("chat_message", msg);
        }
    });

    socket.on("draw_line", |socket: SocketRef, Data::<(String, DrawData)>(data)| {
        let (room_id, draw_data) = data;
        let _ = socket.to(room_id).emit("draw_line", draw_data);
    });

    socket.on("share_embed", |socket: SocketRef, Data::<(String, Option<String>)>(data)| {
        let (room_id, url) = data;
        let _ = socket.to(room_id).emit("update_embed", url);
    });

    // Object Handlers
    socket.on("add_object", |socket: SocketRef, Data::<(String, crate::types::RoomObject)>(data), state: State<AppState>| {
        let (room_id, object) = data;
        state.add_object(room_id.clone(), object.clone());
        let _ = socket.to(room_id).emit("object_added", object);
    });

    socket.on("update_object", |socket: SocketRef, Data::<(String, crate::types::RoomObject)>(data), state: State<AppState>| {
        let (room_id, object) = data;
        state.update_object(room_id.clone(), object.clone());
        let _ = socket.to(room_id).emit("object_updated", object);
    });

    socket.on("remove_object", |socket: SocketRef, Data::<(String, String)>(data), state: State<AppState>| {
        let (room_id, object_id) = data;
        state.remove_object(room_id.clone(), object_id.clone());
        let _ = socket.to(room_id).emit("object_removed", object_id);
    });

    socket.on("update_room_settings", |socket: SocketRef, Data::<(String, serde_json::Value)>(data), state: State<AppState>| {
        let (room_id, settings) = data;
        if let Some(background) = settings.get("background").and_then(|v| v.as_str()) {
             state.update_room_background(room_id.clone(), Some(background.to_string()));
             let _ = socket.to(room_id).emit("room_settings_updated", json!({ "background": background }));
        }
    });

    socket.on("update_user", |socket: SocketRef, Data::<(String, String, String)>(data), state: State<AppState>| {
        let (_room_id, name, color) = data;
        let user_id = socket.id.to_string();
        if let Some((room_id, updated_user)) = state.update_user_details(&user_id, Some(name), Some(color)) {
            let _ = socket.to(room_id.clone()).emit("user_updated", updated_user.clone());
            let _ = socket.emit("user_updated", updated_user);
        }
    });

    socket.on("send_emoji", |socket: SocketRef, Data::<(String, String)>(data)| {
        let (room_id, emoji) = data;
        let _ = socket.to(room_id).emit("emoji_reaction", json!({
            "emoji": emoji,
            "userId": socket.id.to_string()
        }));
    });

    socket.on("typing", |socket: SocketRef, Data::<String>(room_id)| {
        let _ = socket.to(room_id).emit("user_typing", socket.id.to_string());
    });

    socket.on("stop_typing", |socket: SocketRef, Data::<String>(room_id)| {
        let _ = socket.to(room_id).emit("user_stop_typing", socket.id.to_string());
    });

    // WebRTC Signaling
    socket.on("sending_signal", |socket: SocketRef, Data::<SignalPayload>(payload)| {
        if let Some(user_to_signal) = payload.user_to_signal {
             let _ = socket.to(user_to_signal.clone()).emit("user_connected", payload.caller_id.clone());
             let _ = socket.to(user_to_signal).emit("signal_received", json!({
                 "signal": payload.signal,
                 "callerID": payload.caller_id
             }));
        }
    });

    socket.on("returning_signal", |socket: SocketRef, Data::<SignalPayload>(payload)| {
        let _ = socket.to(payload.caller_id).emit("return_signal", json!({
            "signal": payload.signal,
            "id": socket.id.to_string()
        }));
    });

    socket.on_disconnect(|socket: SocketRef, reason: DisconnectReason, state: State<AppState>| {
        println!("User disconnected: {} ({:?})", socket.id, reason);
        state.remove_user(&socket.id.to_string());
        
        // Notify all rooms
        for room in state.rooms.iter() {
             println!("Notifying room {} about user left: {}", room.key(), socket.id);
             let _ = socket.to(room.key().clone()).emit("user_left", socket.id.to_string());
        }

        // Broadcast active rooms update
        let mut active_rooms = Vec::new();
        for room in state.rooms.iter() {
            if !room.users.is_empty() {
                active_rooms.push(json!({
                    "id": room.id,
                    "name": room.name,
                    "userCount": room.users.len(),
                    "users": room.users.iter().map(|u| json!({ "name": u.name, "color": u.color })).collect::<Vec<_>>()
                }));
            }
        }
        let _ = socket.broadcast().emit("active_rooms", active_rooms);
    });
}
