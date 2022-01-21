FROM rustlang/rust:nightly

WORKDIR /app

COPY . .

RUN apt-get update

RUN apt-get install build-essential

RUN rustup target add aarch64-unknown-linux-gnu

RUN RUSTFLAGS="-C target-feature=+crt-static" cargo b --release --target aarch64-unknown-linux-gnu

RUN cargo install diesel_cli

ENTRYPOINT ["./entrypoint.sh"]
