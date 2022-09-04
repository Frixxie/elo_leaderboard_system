FROM rust:latest as builder
WORKDIR /usr/src/backend_server
COPY . .
RUN cargo install --path backend_server

FROM debian:buster-slim
RUN apt-get update && apt-get -y upgrade
COPY --from=builder /usr/local/cargo/bin/backend_server /usr/local/bin/backend_server
CMD ["backend_server"]
