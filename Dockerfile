FROM rustlang/rust:nightly
WORKDIR /app
COPY . .
RUN cargo build --release
RUN cargo install diesel_cli
ENTRYPOINT ["./entrypoint.sh"]
