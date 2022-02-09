#!/bin/sh
while [ 1 ];
do
    /app/iesel database setup && break;
done
/app/diesel migration run
/app/testauskoira-rs
