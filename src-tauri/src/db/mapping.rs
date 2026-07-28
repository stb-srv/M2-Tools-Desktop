use serde::{Deserialize, Serialize};

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
        Self {
            item_proto_table: "item_proto".into(),
            shop_table: "shop".into(),
            shop_item_table: "shop_item".into(),
            player_table: "player".into(),
            account_table: "account.account".into(),
        }
    }
}
