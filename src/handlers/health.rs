use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: i64,
}

/// Health check handler
/// 
/// GET /health
pub async fn health_handler() -> impl IntoResponse {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            timestamp,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await.into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_response_has_timestamp() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let response = health_handler().await.into_response();
        

        let (parts, body) = response.into_parts();
        assert_eq!(parts.status, StatusCode::OK);
        
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let health: HealthResponse = serde_json::from_slice(&bytes).unwrap();
        
        assert_eq!(health.status, "ok");
        assert!(health.timestamp > 0);
    }
}
