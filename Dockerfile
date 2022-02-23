FROM rustlang/rust:nightly AS build

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/none" \
    --shell "/bin/nologin" \
    --no-create-home \
    doggo

WORKDIR /app

RUN cargo install diesel_cli --no-default-features --features "mysql"

# cache dependencies into a layer
RUN cargo new testauskoira-rs
WORKDIR /app/testauskoira-rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release

COPY src .

RUN cargo build --release \
	&& mkdir /out \
	&& mv target/release/testauskoira-rs /out

FROM debian:buster-slim

RUN apt-get update \
	&& apt-get install --no-install-recommends default-mysql-client ca-certificates -y \
	&& rm -rf /var/lib/apt/lists/*

# doggo
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

WORKDIR /app

COPY --from=build /usr/local/cargo/bin/diesel ./
COPY --from=build /out/testauskoira-rs ./
COPY migrations /app/migrations
COPY entrypoint.sh ./

RUN chown -R doggo:doggo /app

USER doggo

CMD ["sh", "entrypoint.sh"] 
