FROM rustlang/rust:nightly-bullseye-slim AS build

RUN apt update \
    && apt upgrade -y \
    && apt install -y git default-libmysqlclient-dev pkg-config libssl-dev perl make

RUN cargo install diesel_cli --no-default-features --features "mysql" \
    && mkdir /out \
    && cp /usr/local/cargo/bin/diesel /out

RUN cargo new --bin testauskoira-rs

WORKDIR /testauskoira-rs

COPY Cargo.toml Cargo.lock ./

RUN cargo build --release && rm -rf .git src/ target/release/deps/testauskoira*

COPY ./src ./src

COPY .git .git

RUN cargo build --release && mv target/release/testauskoira-rs /out



FROM debian:bullseye-slim AS runner

RUN apt update \
    && apt upgrade -y \
    && apt install --no-install-recommends default-mysql-client ca-certificates -y \
    && rm -rf /var/lib/apt/lists/*

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/none" \
    --shell "/sbin/nologin" \
    --no-create-home \
    doggo

WORKDIR /app

COPY --from=build /out/diesel ./
COPY --from=build /out/testauskoira-rs ./
COPY migrations /app/migrations
COPY entrypoint.sh ./

RUN chown -R doggo:doggo /app

USER doggo

CMD ["sh", "entrypoint.sh"]
