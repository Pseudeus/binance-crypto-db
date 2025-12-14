// @generated automatically by Diesel CLI.

diesel::table! {
    agg_trades (id) {
        id -> Integer,
        time -> Double,
        symbol -> Text,
        agg_trade_id -> BigInt,
        price -> Double,
        quantity -> Double,
        first_trade_id -> BigInt,
        last_trade_id -> BigInt,
        is_buyer_maker -> Bool,
    }
}

diesel::table! {
    order_books (id) {
        id -> Integer,
        time -> Double,
        symbol -> Text,
        bids -> Binary,
        asks -> Binary,
    }
}

diesel::allow_tables_to_appear_in_same_query!(agg_trades, order_books,);
