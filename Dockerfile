FROM rustlang/rust:nightly
WORKDIR /app
COPY . .
RUN cargo build -j 2 --release
RUN cargo install  -j 2 diesel_cli
ENTRYPOINT ["./entrypoint.sh"]
