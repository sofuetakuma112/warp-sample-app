use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ユーザーが持っているロールや、
// HTTPが特定のエンドポイントに到達することを許可されているかどうかを
// 決定するための他の有用な情報を暗号化することができるのでSessionと名付けた
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Session {
    pub exp: DateTime<Utc>,
    pub account_id: AccountId,
    pub nbf: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub id: Option<AccountId>,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(pub i32);
