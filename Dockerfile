# Step 1: Build stage
FROM rust:latest AS builder

# Set the working directory
WORKDIR /usr/src/app

# Copy the Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Pre-fetch dependencies (cached if they don't change)
RUN cargo fetch

# Copy the source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

# Step 2: Runtime stage
FROM debian:buster-slim

# Install required libraries
RUN apt-get update && apt-get install -y libssl-dev && apt-get clean

# Set the working directory
WORKDIR /usr/src/app

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/app/target/release/util .

# Expose application port
EXPOSE 8080

# Run the binary
CMD ["./util"]
