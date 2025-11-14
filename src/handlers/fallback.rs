use axum::{
    http::HeaderMap,
    response::{Html, IntoResponse, Response},
};
use crate::core::error::AnnounceError;

pub async fn fallback_handler(headers: HeaderMap) -> Response {
    // Check if this is a browser request
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    let is_browser = user_agent.contains("Mozilla") 
        || user_agent.contains("Chrome") 
        || user_agent.contains("Safari")
        || user_agent.contains("Firefox")
        || user_agent.contains("Edge");
    
    if is_browser {
        let html = "Nothing to see here. Lost in the void!";
        
        return Html(html).into_response();
    }

    AnnounceError::InvalidParameter(
        "Invalid endpoint. Valid endpoints: /announce, /health".to_string()
    ).into_response()
}
