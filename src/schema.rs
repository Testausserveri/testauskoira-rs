table! {
    messages_day_stat (id) {
        id -> Integer,
        date -> Nullable<Date>,
        userid -> Nullable<Varchar>,
        message_count -> Nullable<Integer>,
    }
}
