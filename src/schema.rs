// @generated automatically by Diesel CLI.

diesel::table! {
    order_books (id) {
        id -> Integer,
        time -> Double,
        symbol -> Text,
        bids -> Binary,
        asks -> Binary,
    }
}
