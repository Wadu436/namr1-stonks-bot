FROM rust:1.81.0 AS builder

RUN USER=root cargo new --bin namr1-stonks-bot
WORKDIR /namr1-stonks-bot

# Cache dependencies
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --bin namr1-stonks-bot --release
RUN rm src/*.rs
RUN rm /namr1-stonks-bot/target/release/deps/namr1_stonks_bot*

# Build App
COPY ./src ./src
RUN cargo build --bin namr1-stonks-bot --release

# Final image
FROM debian:bookworm-slim
WORKDIR /usr/app/

# Copy the executable
COPY --from=builder /namr1-stonks-bot/target/release/namr1-stonks-bot /usr/app/

# Start command
CMD [ "./namr1-stonks-bot" ]