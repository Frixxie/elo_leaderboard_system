FROM rust:latest
WORKDIR /usr/src/backend_server
COPY . .
RUN cargo install --path .
RUN cargo clean
CMD ["backend_server"]
