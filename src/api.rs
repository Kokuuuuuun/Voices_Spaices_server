use axum::{
    extract::State,
    response::{IntoResponse, Json},
};
use crate::state::AppState;
use serde::Serialize;

#[derive(Serialize)]
pub struct RoomSummary {
    id: String,
    name: String,
    user_count: usize,
}

pub async fn list_rooms(State(state): State<AppState>) -> impl IntoResponse {
    let mut rooms = Vec::new();
    
    // Get active rooms from memory
    for room in state.rooms.iter() {
        rooms.push(RoomSummary {
            id: room.id.clone(),
            name: room.name.clone(),
            user_count: room.users.len(),
        });
    }

    // TODO: Also fetch persistent rooms from DB if not in memory?
    // For now, let's stick to active rooms or maybe fetch all from DB and merge count.
    // Let's just return active rooms for this "Live" dashboard feel.
    
    Json(rooms)
}
