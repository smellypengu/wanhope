# Requires common lib and server bin.
# There's definitely a more user friendly way to do this but it'll do for now.

FROM rust:slim-buster as build

WORKDIR /wanhope

RUN USER=root cargo new --lib common
RUN USER=root cargo new --bin server

COPY ./common/Cargo.toml ./common/Cargo.toml
COPY ./server/Cargo.toml ./server/Cargo.toml

RUN cargo build --manifest-path ./server/Cargo.toml --release

RUN rm ./common/src/*.rs
RUN rm ./server/src/*.rs

COPY ./common/src ./common/src
COPY ./server/src ./server/src

RUN rm ./server/target/release/deps/server*

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install --target x86_64-unknown-linux-musl --path ./server

FROM scratch

COPY --from=build /wanhope/server/target/release/server .

EXPOSE 8080

CMD ["server"]
