use std::collections::HashMap;

use warp::hyper::StatusCode;

use crate::{
    profanity::check_profanity,
    store::Store,
    types::{
        answer::{Answer, AnswerId, NewAnswer},
        question::QuestionId,
    },
};

// POST /answers
pub async fn add_answer(
    store: Store,
    params: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let content = match check_profanity(params.get("content").unwrap().to_string()).await {
        Ok(res) => res,
        Err(e) => return Err(warp::reject::custom(e)),
    };

    let answer = NewAnswer {
        content,
        question_id: QuestionId(params.get("questionId").unwrap().parse().unwrap()),
    };

    match store.add_answer(answer).await {
        Ok(_) => Ok(warp::reply::with_status("Answer added", StatusCode::OK)),
        Err(e) => Err(warp::reject::custom(e)),
    }
}
