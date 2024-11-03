FROM rust:1.73

WORKDIR /usr/src/hyve-message-server
COPY . .

RUN cargo build --release

EXPOSE 8080

CMD ["./target/release/hyve-message-server"]