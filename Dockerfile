FROM rust:latest AS build

WORKDIR /app

COPY Cargo* ./

COPY src/main.rs ./src/

RUN cargo fetch 

COPY . .

RUN cargo build -j 2 --release --target-dir /usr/local/cargo

RUN cargo install -j 2 diesel_cli --no-default-features --features "mysql"

CMD ["bash", "entrypoint.sh"]

# Final image
FROM debian:latest

RUN apt update

RUN apt-get install default-mysql-client --yes

WORKDIR /app

COPY --from=build /usr/local/cargo/bin/diesel /app/

COPY --from=build /usr/local/cargo/release/testauskoira-rs /app/

COPY migrations /app/migrations

RUN apt-get update

COPY entrypoint.sh ./

CMD ["bash", "entrypoint.sh"] 
