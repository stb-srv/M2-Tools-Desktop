use serde::{Deserialize, Serialize};

// Deliberately not wired into any query yet - every table name is still
// hardcoded inline in db/shop.rs, db/item.rs etc. This struct exists as a
// placeholder so that supporting a differently-laid-out core fork later
// means threading a TableMapping through, not rewriting every query string.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMapping {
    pub item_proto_table: String,
    pub shop_table: String,
    pub shop_item_table: String,
    pub player_table: String,
    pub account_table: String,
}

impl Default for TableMapping {
    fn default() -> Self {
        // Basiswerte vom Metin2-Dev-Server (FreeBSD/MariaDB-Core):
        // item_proto/shop/shop_item/player liegen in der "player"-DB, nicht "common".
        Self {
            item_proto_table: "player.item_proto".into(),
            shop_table: "player.shop".into(),
            shop_item_table: "player.shop_item".into(),
            player_table: "player.player".into(),
            account_table: "account.account".into(),
        }
    }
}
