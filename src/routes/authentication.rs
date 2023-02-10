use std::future;

use argon2::{self, Config};
use chrono::Utc;
use rand::Rng;
use warp::http::StatusCode;
use warp::Filter;

use crate::store::Store;
use crate::types::account::{Account, AccountId, Session};

pub async fn register(store: Store, account: Account) -> Result<impl warp::Reply, warp::Rejection> {
    let hashed_password = hash(account.password.as_bytes());

    let account = Account {
        id: account.id,
        email: account.email,
        password: hashed_password,
    };

    match store.add_account(account).await {
        Ok(_) => Ok(warp::reply::with_status("Account added", StatusCode::OK)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub async fn login(store: Store, login: Account) -> Result<impl warp::Reply, warp::Rejection> {
    // 1. emailでDBを検索
    // 2. DBに保存されたハッシュ値と平文のパスワードを比較
    // 3. pasetoトークン形式で返す(JWTと同じカテゴリ)
    match store.get_account(login.email).await {
        Ok(account) => match verify_password(&account.password, login.password.as_bytes()) {
            Ok(verified) => {
                if verified {
                    // アカウントIDをカプセル化したトークンを作成し、HTTPレスポンスとして返送
                    Ok(warp::reply::json(&issue_token(
                        account.id.expect("id not found"),
                    )))
                } else {
                    // verifyの結果、一致しなかった
                    Err(warp::reject::custom(handle_errors::Error::WrongPassword))
                }
            }
            // argon2::verify_encodedの実行そのものに失敗
            Err(e) => Err(warp::reject::custom(
                handle_errors::Error::ArgonLibraryError(e),
            )),
        },
        Err(e) => Err(warp::reject::custom(e)),
    }
}

pub fn hash(password: &[u8]) -> String {
    // rand::thread_rng().gen: 標準分布をサポートするランダムな値を返します。
    // ソルトはパスワードハッシュの一部で、argon2クレートが平文パスワードでハッシュを検証するために使用する
    let salt = rand::thread_rng().gen::<[u8; 32]>();
    let config = Config::default();
    // パスワードをハッシュ化し、エンコードされたハッシュを返す。
    argon2::hash_encoded(password, &salt, &config).unwrap()
}

fn verify_password(hash: &str, password: &[u8]) -> Result<bool, argon2::Error> {
    // ソルトはハッシュ値に含まれておりそれを使用することで
    // 平文のパスワードと照合することが出来る
    argon2::verify_encoded(hash, password)
}

pub fn verify_token(token: String) -> Result<Session, handle_errors::Error> {
    let token = paseto::tokens::validate_local_token(
        &token,
        None,
        &"RANDOM WORDS WINTER MACINTOSH PC".as_bytes(),
        &paseto::tokens::TimeBackend::Chrono,
    )
    .map_err(|_| handle_errors::Error::CannotDecryptToken)?;

    serde_json::from_value::<Session>(token).map_err(|_| handle_errors::Error::CannotDecryptToken)
}

fn issue_token(account_id: AccountId) -> String {
    let current_date_time = Utc::now();
    let dt = current_date_time + chrono::Duration::days(1);

    // Pasetoトークン形式に暗号化する
    paseto::tokens::PasetoBuilder::new()
        // pasetoトークンに使用する暗号化キーを設定します
        .set_encryption_key(&Vec::from("RANDOM WORDS WINTER MACINTOSH PC".as_bytes()))
        // 有効期限(上限)を設定
        .set_expiration(&dt)
        // 有効期限(下限)を設定
        .set_not_before(&Utc::now())
        // claimにトークンに持たせたいユーザーの識別情報を持たせる
        .set_claim("account_id", serde_json::json!(account_id))
        .build()
        .expect("Failed to construct paseto token w/ builder!")
}

pub fn auth() -> impl Filter<Extract = (Session,), Error = warp::Rejection> + Clone {
    warp::header::<String>("Authorization").and_then(|token: String| {
        let token = match verify_token(token) {
            Ok(t) => t,
            Err(_) => return future::ready(Err(warp::reject::reject())),
        };

        future::ready(Ok(token))
    })
}
