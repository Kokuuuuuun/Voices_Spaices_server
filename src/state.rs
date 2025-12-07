use dashmap::DashMap;
use std::sync::Arc;
use crate::types::{Room, User};
use crate::db::Db;

#[derive(Clone)]
pub struct AppState {
    pub rooms: Arc<DashMap<String, Room>>,
    pub db: Db,
}

impl AppState {
    pub async fn new(db_url: &str) -> Result<Self, sqlx::Error> {
        let db = Db::new(db_url).await?;
        let rooms = DashMap::new();
        
        // Load rooms from DB
        let loaded_rooms = db.get_rooms().await?;
        for room in loaded_rooms {
            rooms.insert(room.id.clone(), room);
        }

        Ok(Self {
            rooms: Arc::new(rooms),
            db,
        })
    }

    pub fn get_room(&self, room_id: &str) -> Option<Room> {
        self.rooms.get(room_id).map(|r| r.clone())
    }

    pub fn add_user_to_room(&self, room_id: String, user: User) {
        self.rooms.entry(room_id.clone())
            .or_insert_with(|| Room {
                id: room_id.clone(),
                name: format!("Room {}", room_id),
                users: Vec::new(),
                objects: Vec::new(),
                background: None,
            });
        
        // Ensure room exists in DB (upsert)
        // We do this outside the entry lock to avoid holding it too long, 
        // but we need to know if we just created it. 
        // For simplicity, just upsert always or check existence.
        // Actually, entry API makes it hard to async await inside.
        // Let's just fire and forget an upsert for the room itself.
        let db = self.db.clone();
        let rid = room_id.clone();
        let rname = format!("Room {}", room_id);
        tokio::spawn(async move {
            // Minimal room struct for saving
            let room = Room { id: rid, name: rname, users: vec![], objects: vec![], background: None };
            let _ = db.save_room(&room).await;
        });

        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            // Check if user already exists to avoid duplicates
            if !room.users.iter().any(|u| u.id == user.id) {
                room.users.push(user);
            }
        }
    }

    pub fn remove_user(&self, user_id: &str) {
        // This is a bit inefficient, iterating all rooms. 
        // In a real DB scenario this would be a direct lookup.
        // For in-memory with DashMap, we iterate.
        for mut room in self.rooms.iter_mut() {
            if let Some(pos) = room.users.iter().position(|u| u.id == user_id) {
                room.users.remove(pos);
                // If room empty, maybe remove? Keeping it simple for now.
                return;
            }
        }
    }
    
    pub fn get_user_room(&self, user_id: &str) -> Option<String> {
        for room in self.rooms.iter() {
            if room.users.iter().any(|u| u.id == user_id) {
                return Some(room.id.clone());
            }
        }
        None
    }

    pub fn get_user(&self, room_id: &str, user_id: &str) -> Option<User> {
        if let Some(room) = self.rooms.get(room_id) {
            return room.users.iter().find(|u| u.id == user_id).cloned();
        }
        None
    }

    pub fn update_user_position(&self, user_id: &str, x: f64, y: f64) -> Option<String> {
        for mut room in self.rooms.iter_mut() {
            if let Some(user) = room.users.iter_mut().find(|u| u.id == user_id) {
                user.x = x;
                user.y = y;
                return Some(room.id.clone());
            }
        }
        None
    }

    pub fn update_user_details(&self, user_id: &str, name: Option<String>, color: Option<String>) -> Option<(String, User)> {
        for mut room in self.rooms.iter_mut() {
            let room_id = room.id.clone();
            if let Some(user) = room.users.iter_mut().find(|u| u.id == user_id) {
                if let Some(n) = name { user.name = n; }
                if let Some(c) = color { user.color = c; }
                return Some((room_id, user.clone()));
            }
        }
        None
    }

    pub fn add_object(&self, room_id: String, object: crate::types::RoomObject) {
        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            room.objects.push(object.clone());
            // Async save (fire and forget for now, or spawn task)
            let db = self.db.clone();
            let rid = room_id.clone();
            tokio::spawn(async move {
                let _ = db.save_object(&rid, &object).await;
            });
        }
    }

    pub fn update_object(&self, room_id: String, object: crate::types::RoomObject) {
        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            if let Some(obj) = room.objects.iter_mut().find(|o| o.id == object.id) {
                *obj = object.clone();
                let db = self.db.clone();
                let rid = room_id.clone();
                let obj_clone = object.clone();
                tokio::spawn(async move {
                    let _ = db.save_object(&rid, &obj_clone).await;
                });
            }
        }
    }

    pub fn update_room_background(&self, room_id: String, background: Option<String>) {
        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            room.background = background;
        }
    }

    pub fn remove_object(&self, room_id: String, object_id: String) {
        if let Some(mut room) = self.rooms.get_mut(&room_id) {
            if let Some(pos) = room.objects.iter().position(|o| o.id == object_id) {
                room.objects.remove(pos);
                let db = self.db.clone();
                tokio::spawn(async move {
                    let _ = db.delete_object(&object_id).await;
                });
            }
        }
    }
}
