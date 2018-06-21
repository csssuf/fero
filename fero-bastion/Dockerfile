FROM rust:stretch as build

ADD https://github.com/google/protobuf/releases/download/v3.5.1/protoc-3.5.1-linux-x86_64.zip \
    /tmp/protoc.zip

RUN apt-get update && \
    apt-get install -yy \
        build-essential \
        cmake \
        golang-go \
        libsqlite3-dev \
        musl-tools \
        unzip && \
    unzip -d /usr /tmp/protoc.zip && \
    rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/fero
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./fero-client/ ./fero-client/
COPY ./fero-bastion/ ./fero-bastion/
COPY ./fero-proto/ ./fero-proto/
COPY ./fero-server/ ./fero-server/
RUN cargo build --target=x86_64-unknown-linux-musl --release --package fero-bastion && \
    strip target/x86_64-unknown-linux-musl/release/fero-bastion

FROM scratch
WORKDIR /opt/fero-bastion/bin
EXPOSE 50051
ENTRYPOINT ["./fero-bastion"]
COPY --from=build /usr/src/fero/target/x86_64-unknown-linux-musl/release/fero-bastion .
