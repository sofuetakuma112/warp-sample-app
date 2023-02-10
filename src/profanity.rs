use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};

// エラーの場合は{ message }のJSONが返ってくるので、そのフィールド値を取り出すためのstruct
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct APIResponse {
    message: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWord {
    original: String,
    word: String,
    deviations: i64,
    info: i64,
    #[serde(rename = "replacedLen")]
    replaced_len: i64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct BadWordsResponse {
    content: String,
    bad_words_total: i64,
    bad_words_list: Vec<BadWord>,
    censored_content: String,
}

pub async fn check_profanity(content: String) -> Result<String, handle_errors::Error> {
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    //　ここでのエラーはMiddlewareReqwestAPIErrorとして扱う
    let res = client
        .post("https://api.apilayer.com/bad_words?censor_character=*")
        .header("apikey", "hoge")
        .body(content)
        .send()
        .await
        .map_err(|e| handle_errors::Error::MiddlewareReqwestAPIError(e))?;

    // 4xx, 5xxのステータスかチェック
    // ここでのエラーはClientErrorかServerErrorとして扱う
    if !res.status().is_success() {
        if res.status().is_client_error() {
            let err = transform_error(res).await;
            return Err(handle_errors::Error::ClientError(err));
        } else {
            let err = transform_error(res).await;
            return Err(handle_errors::Error::ServerError(err));
        }
    }

    // ここでのエラーはReqwestAPIErrorとして扱う
    match res.json::<BadWordsResponse>().await {
        Ok(res) => Ok(res.censored_content),
        Err(e) => Err(handle_errors::Error::ReqwestAPIError(e)),
    }
}

async fn transform_error(res: reqwest::Response) -> handle_errors::APILayerError {
    handle_errors::APILayerError {
        status: res.status().as_u16(),
        message: res.json::<APIResponse>().await.unwrap().message,
    }
}
