FROM rust:latest as builder
WORKDIR /usr/src/backend_server
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get -y upgrade && apt-get install build-essential libc6 glibc-source -y
COPY --from=builder /usr/local/cargo/bin/backend_server /usr/local/bin/backend_server
CMD ["backend_server"]
