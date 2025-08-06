# Multi-stage build for Daystrom TUI
FROM rust:1.75-alpine as builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    pkgconfig

# Set working directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src/ ./src/

# Build the application
RUN cargo build --release

# Runtime stage
FROM alpine:3.19

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    openssl \
    && rm -rf /var/cache/apk/*

# Create non-root user
RUN addgroup -g 1001 -S daystrom && \
    adduser -u 1001 -S daystrom -G daystrom

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/daystrom-tui /app/daystrom-tui

# Copy configuration file
COPY config.yaml /app/config.yaml

# Change ownership to non-root user
RUN chown -R daystrom:daystrom /app

# Switch to non-root user
USER daystrom

# Expose port (if needed for web interface in future)
# EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV TERM=xterm-256color

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ps aux | grep daystrom-tui || exit 1

# Default command
ENTRYPOINT ["/app/daystrom-tui"]

# Default arguments
CMD ["--config", "/app/config.yaml"] 