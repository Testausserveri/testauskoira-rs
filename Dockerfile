FROM rustlang/rust:nightly
COPY src src
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
RUN cargo build --release
CMD ["./target/release/testauskoira-rs"]
