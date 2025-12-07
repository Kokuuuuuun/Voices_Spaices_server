use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite, SqlitePool};
use crate::types::{Room, RoomObject, User};
use std::sync::Arc;

#[derive(Clone)]
pub struct Db {
    pub pool: Pool<Sqlite>,
}

impl Db {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(url).await?;
        
        // Create tables if they don't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS rooms (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS room_objects (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                type TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                width REAL NOT NULL,
                height REAL NOT NULL,
                content TEXT NOT NULL,
                z_index INTEGER NOT NULL,
                rotation REAL NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                room_id TEXT
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        // Create messages table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                text TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    // The original init function is removed as its logic is now in new()
    // async fn init(&self) -> Result<(), sqlx::Error> {
    //     // Create tables if not exist
    //     sqlx::query(
    //         r#"
    //         CREATE TABLE IF NOT EXISTS rooms (
    //             id TEXT PRIMARY KEY,
    //             name TEXT NOT NULL
    //         );
    //         CREATE TABLE IF NOT EXISTS room_objects (
    //             id TEXT PRIMARY KEY,
    //             room_id TEXT NOT NULL,
    //             obj_type TEXT NOT NULL,
    //             x REAL NOT NULL,
    //             y REAL NOT NULL,
    //             width REAL NOT NULL,
    //             height REAL NOT NULL,
    //             content TEXT NOT NULL,
    //             z_index INTEGER NOT NULL,
    //             rotation REAL NOT NULL,
    //             FOREIGN KEY(room_id) REFERENCES rooms(id)
    //         );
    //         "#
    //     )
    //     .execute(&self.pool)
    //     .await?;
        
    //     Ok(())
    // }

    pub async fn get_rooms(&self) -> Result<Vec<Room>, sqlx::Error> {
        // ... (existing implementation)
        // Note: We might want to load users into rooms here too, or separate call.
        // For simplicity, let's keep rooms simple and load objects.
        // Users are usually transient in memory for socket server, but we want persistence.
        // Let's just load rooms and objects for now as before.
        let rows = sqlx::query_as::<_, (String, String)>("SELECT id, name FROM rooms")
            .fetch_all(&self.pool)
            .await?;

        let mut rooms = Vec::new();
        for row in rows {
            rooms.push(Room {
                id: row.0,
                name: row.1,
                users: Vec::new(), // Users will join or be loaded separately if we want "offline" users
                objects: Vec::new(),
                background: None,
            });
        }
        Ok(rooms)
    }

    pub async fn save_user(&self, user: &User, room_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO users (id, name, color, x, y, room_id) VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, color=excluded.color, x=excluded.x, y=excluded.y, room_id=excluded.room_id",
        )
        .bind(&user.id)
        .bind(&user.name)
        .bind(&user.color)
        .bind(user.x)
        .bind(user.y)
        .bind(room_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_room_objects(&self, room_id: &str) -> Result<Vec<RoomObject>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, f64, f64, f64, f64, String, i32, f64)>(
            r#"
            SELECT id, type, x, y, width, height, content, z_index, rotation
            FROM room_objects
            WHERE room_id = ?
            "#
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, obj_type, x, y, width, height, content, z_index, rotation)| RoomObject {
            id,
            obj_type,
            x,
            y,
            width,
            height,
            content,
            z_index,
            rotation,
        }).collect())
    }

    pub async fn save_room(&self, room: &Room) -> Result<(), sqlx::Error> {
        // Upsert room
        sqlx::query(
            "INSERT INTO rooms (id, name) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET name = ?"
        )
        .bind(&room.id)
        .bind(&room.name)
        .bind(&room.name)
        .execute(&self.pool)
        .await?;

        for obj in &room.objects {
            self.save_object(&room.id, obj).await?;
        }
        Ok(())
    }

    pub async fn save_object(&self, room_id: &str, obj: &RoomObject) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO room_objects (id, room_id, type, x, y, width, height, content, z_index, rotation)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
            x = ?, y = ?, width = ?, height = ?, content = ?, z_index = ?, rotation = ?
            "#
        )
        .bind(&obj.id)
        .bind(room_id)
        .bind(&obj.obj_type)
        .bind(obj.x)
        .bind(obj.y)
        .bind(obj.width)
        .bind(obj.height)
        .bind(&obj.content)
        .bind(obj.z_index)
        .bind(obj.rotation)
        // Updates
        .bind(obj.x)
        .bind(obj.y)
        .bind(obj.width)
        .bind(obj.height)
        .bind(&obj.content)
        .bind(obj.z_index)
        .bind(obj.rotation)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_object(&self, object_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM room_objects WHERE id = ?")
            .bind(object_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_messages_table(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                room_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                user_name TEXT NOT NULL,
                text TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn save_message(&self, msg: &crate::types::ChatMessage, room_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO messages (id, room_id, user_id, user_name, text, timestamp) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&msg.id)
        .bind(room_id)
        .bind(&msg.user_id)
        .bind(&msg.user_name)
        .bind(&msg.text)
        .bind(msg.timestamp)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_messages(&self, room_id: &str) -> Result<Vec<crate::types::ChatMessage>, sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
            "SELECT id, user_id, user_name, text, timestamp FROM messages WHERE room_id = ? ORDER BY timestamp ASC"
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(id, user_id, user_name, text, timestamp)| crate::types::ChatMessage {
            id,
            user_id,
            user_name,
            text,
            timestamp,
        }).collect())
    }
}
