---
title: VoiceSpaces Server
emoji: 🎤
colorFrom: indigo
colorTo: blue
sdk: docker
app_port: 7860
pinned: false
---

# VoiceSpaces Server

Real-time collaborative voice chat server built with Rust.

## Features

- WebSocket/Socket.IO for real-time communication
- SQLite database for persistence
- WebRTC signaling support
- Chat history

## API Endpoints

- `GET /health` - Health check
- `POST /api/register` - User registration
- `POST /api/login` - User login
- `GET /api/rooms` - List active rooms

## Socket.IO Events

### Client → Server

- `join_room` - Join a room
- `leave_room` - Leave a room
- `send_chat` - Send chat message
- `move` - Update position
- `update_user` - Update user profile

### Server → Client

- `room_state` - Initial room state
- `user_joined` - New user notification
- `user_left` - User left notification
- `chat_message` - Chat message
- `active_rooms` - Active rooms list
