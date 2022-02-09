FROM rust:latest AS build

WORKDIR /app

COPY Cargo* ./

RUN cargo fetch 

COPY src/main.rs ./src/

COPY . .

RUN cargo build -j 2 --release --target-dir /usr/local/cargo

RUN cargo install -j 2 diesel_cli --no-default-features --features "mysql"

# Final image
FROM debian:slim

RUN apt-get update

RUN apt-get install default-mysql-client --yes

WORKDIR /app

COPY --from=build /usr/local/cargo/bin/diesel /app/

COPY --from=build /usr/local/cargo/release/testauskoira-rs /app/

COPY migrations /app/migrations

COPY entrypoint.sh ./

CMD ["bash", "entrypoint.sh"] 
