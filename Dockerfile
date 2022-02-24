FROM --platform=$BUILDPLATFORM rustlang/rust:nightly AS build

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="aarch64-linux-gnu-gcc"

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/none" \
    --shell "/bin/nologin" \
    --no-create-home \
    doggo

WORKDIR /app

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-gnu" > /target.txt ;; \
    "linux/arm64") echo "aarch64-unknown-linux-gnu" > /target.txt ;; \
    *) exit 1 ;; \
esac

RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
    dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install gcc-aarch64-linux-gnu libc6-dev-arm64-cross -y \
    && apt-get install libmariadb-dev:arm64 libmariadb-dev-compat:arm64 default-libmysqlclient-dev:arm64 -y \
fi

RUN dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install gcc-aarch64-linux-gnu libc6-dev-arm64-cross -y \
    && apt-get install libmariadb-dev:arm64 libmariadb-dev-compat:arm64 default-libmysqlclient-dev:arm64 -y

RUN rustup target add $(cat /target.txt)

RUN cargo install --target $(cat /target.txt) diesel_cli --no-default-features --features "mysql" \
    && mkdir /out \
    && cp /usr/local/cargo/bin/diesel /out

# cache dependencies into a layer
RUN cargo new testauskoira-rs
WORKDIR /app/testauskoira-rs
COPY Cargo.toml Cargo.lock ./
RUN cargo build --target $(cat /target.txt) --release

COPY src .

RUN cargo build --release --target $(cat /target.txt) \
	&& mv target/$(cat /target.txt)/release/testauskoira-rs /out

FROM debian:buster-slim

RUN apt-get update \
	&& apt-get install --no-install-recommends default-mysql-client ca-certificates -y \
	&& rm -rf /var/lib/apt/lists/*

# doggo
COPY --from=build /etc/passwd /etc/passwd
COPY --from=build /etc/group /etc/group

WORKDIR /app

COPY --from=build /out/diesel ./
COPY --from=build /out/testauskoira-rs ./
COPY migrations /app/migrations
COPY entrypoint.sh ./

RUN chown -R doggo:doggo /app

USER doggo

CMD ["sh", "entrypoint.sh"] 
