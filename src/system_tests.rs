//! REQ-008 system tests — HTTP router + sessions + DB (no external networks).

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use crate::app::test_support::test_app;

async fn body_text(res: axum::response::Response) -> String {
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn cookie_from(res: &axum::response::Response) -> Option<String> {
    res.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(|c| c.split(';').next().unwrap_or(c).to_string())
        .collect::<Vec<_>>()
        .into_iter()
        .reduce(|a, b| format!("{a}; {b}"))
}

#[tokio::test]
async fn st_008_a_home_ok() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn st_008_b_login_ok() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn st_008_c_admin_login_page_ok() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/admin/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn st_008_d_api_me_redirects_when_anonymous() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        res.status().is_redirection(),
        "expected redirect, got {}",
        res.status()
    );
    let loc = res.headers().get(header::LOCATION).unwrap();
    assert_eq!(loc, "/login");
}

#[tokio::test]
async fn st_008_e_analog_ingest_htmx_unauthorized() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/analog/ingest")
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st_008_f_admin_dashboard_redirects_when_anonymous() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(header::LOCATION).unwrap(),
        "/admin/login"
    );
}

#[tokio::test]
async fn st_008_g_api_users_htmx_forbidden() {
    let (_dir, app, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn st_008_h_admin_login_positive_grants_dashboard() {
    let (_dir, app, _) = test_app("admin-secret").await;

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/login")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("password=admin-secret"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        login.status().is_redirection() || login.status().is_success(),
        "login status {}",
        login.status()
    );
    let cookie = cookie_from(&login).expect("session cookie");

    let dash = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(dash.status(), StatusCode::OK);
    let body = body_text(dash).await;
    assert!(
        body.contains("Admin") || body.contains("admin") || body.contains("Users"),
        "dashboard body unexpected"
    );
}

#[tokio::test]
async fn st_008_i_admin_login_negative_wrong_password() {
    let (_dir, app, _) = test_app("admin-secret").await;

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/login")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("password=wrong-password"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Wrong password should not redirect to dashboard as success.
    let cookie = cookie_from(&login);
    if let Some(cookie) = cookie {
        let dash = app
            .oneshot(
                Request::builder()
                    .uri("/admin")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            dash.status().is_redirection() || dash.status() == StatusCode::FORBIDDEN,
            "wrong password must not open admin, got {}",
            dash.status()
        );
    } else {
        // No session cookie is also a valid negative outcome.
        assert!(
            login.status().is_success() || login.status().is_redirection(),
            "unexpected status {}",
            login.status()
        );
        let body = body_text(login).await;
        assert!(
            body.contains("error")
                || body.contains("Fehler")
                || body.contains("falsch")
                || body.contains("Invalid")
                || body.contains("password"),
            "expected error copy on failed admin login"
        );
    }
}
