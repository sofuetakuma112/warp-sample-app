#![warn(clippy::all)]

use handle_errors::return_error;
use routes::answer::add_answer;
use routes::question::{add_question, delete_question, get_questions, update_question};
use store::Store;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::hyper::Method;
use warp::Filter;

mod profanity;
mod routes;
mod store;
mod types;

#[tokio::main]
async fn main() {
    // アプリケーションのログレベルを追加
    let log_filter = std::env::var("RUST_LOG")
        // 記録するイベントの種類を指定している
        .unwrap_or_else(|_| "practical_rust_book=info,warp=error".to_owned());

    // データベースの接続を開くのは非同期なのでawaitを付ける必要がある
    let store = Store::new("postgres://admin:admin@localhost:5432/rustwebdev").await;

    // コード上でマイグレーションを実行
    sqlx::migrate!()
        .run(&store.clone().connection)
        .await
        .expect("Cannot run migration");

    // anyフィルタはあらゆるリクエストにマッチし、ハンドラ関数からstoreを取得できるので、その都度clone()して渡す
    // mapで返した値は後続のフィルターのハンドラ関数に渡せる？
    let store_filter = warp::any().map(move || store.clone());

    tracing_subscriber::fmt()
        .with_env_filter(log_filter) // どのトレースイベントを記録するか伝える
        .with_span_events(FmtSpan::CLOSE) // スパンを閉じるときのイベントも記録するようにする
        .init();

    // サーバー側で以下のレスポンスヘッダを含めることでCORSのルールを満たし、オリジン間通信が可能になる
    // Access-Control-Allow-Origin
    // Access-Control-Allow-Headers
    // Access-Control-Allow-Methods（無くても良い？）
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(&[Method::PUT, Method::DELETE, Method::GET, Method::POST]);

    let get_questions = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter.clone())
        .and_then(get_questions)
        // カスタムイベントのロギングを設定する
        // カスタムイベントや各受信リクエストのログを取ることができる
        .with(warp::trace(|info| {
            // tracing::info_span!: ログレベル INFO で tracing::span を呼び出す
            tracing::info_span!(
                "get_questions request",
                method = %info.method(), // %はDisplay トレイトを使用してデータを表示することを意味する
                path = %info.path(),
                id = %uuid::Uuid::new_v4(),
            )
        }));

    let add_question = warp::post()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json()) // ルートハンドラの引数の型情報からjson<T>の型パラメータを決定する
        .and_then(add_question);

    let update_question = warp::put()
        .and(warp::path("questions"))
        .and(warp::path::param::<i32>()) // パスパラメータ
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json()) // application/json
        .and_then(update_question);

    let delete_question = warp::delete()
        .and(warp::path("questions"))
        .and(warp::path::param::<i32>())
        .and(warp::path::end())
        .and(store_filter.clone())
        .and_then(delete_question);

    let add_answer = warp::post()
        .and(warp::path("answers"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::form()) // application/x-www-form-urlencoded
        .and_then(add_answer);

    let routes = get_questions
        .or(update_question)
        .or(add_question)
        .or(add_answer)
        .or(delete_question)
        .with(cors)
        .with(warp::trace::request()) // 受信リクエストのロギングを設定する
        .recover(return_error);

    warp::serve(routes) // warpのserveメソッドにroutesフィルタを渡して、サーバを起動します。
        .run(([127, 0, 0, 1], 3030))
        .await;
}
