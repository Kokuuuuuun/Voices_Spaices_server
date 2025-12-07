# VoiceSpaces Server - HuggingFace Spaces Deployment
FROM rust:latest as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/voice-spaces-server ./server

# HuggingFace Spaces persistent storage is mounted at /data
# Create the directory to ensure it exists
RUN mkdir -p /data && chmod 777 /data
VOLUME /data

# Expose port (HuggingFace uses 7860)
EXPOSE 7860

# Environment variables
ENV PORT=7860
ENV DATABASE_URL=sqlite:/data/voicespaces.db?mode=rwc

# Run the server
CMD ["./server"]


