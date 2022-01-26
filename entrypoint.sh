#!/bin/sh
while [ 1 ];
do
    /usr/local/cargo/bin/diesel database setup && break;
    /usr/local/cargo/bin/diesel migration run && break;
done
./target/release/testauskoira-rs
