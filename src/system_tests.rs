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
    let (_dir, app, _, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn st_008_b_login_ok() {
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;
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
    let (_dir, app, _, _) = test_app("admin-secret").await;

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
    let (_dir, app, _, _) = test_app("admin-secret").await;

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

#[tokio::test]
async fn st_006_a_convert_without_secure_id_returns_400() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-006", "st-user")
        .await
        .expect("user");
    let ticket = db::create_ticket(
        &pool,
        user.id,
        "544850-103401",
        None,
        None,
        None,
        None,
        "PROCESSING",
        None,
    )
    .await
    .expect("ticket");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tickets/{}/convert", ticket.id))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("secure_id="))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_text(res).await;
    assert!(
        body.contains("class=\"error\"") && body.contains("Secure-ID"),
        "expected visible Secure-ID error, got: {body}"
    );
}

/// REQ-014: positive gear create returns notice feedback for the user.
#[tokio::test]
async fn st_014_p1_create_camera_returns_notice() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-014-p1", "st-user")
        .await
        .expect("user");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gear/cameras")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("label=Canon+AE-1"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    assert!(
        body.contains("gear-cameras-out")
            && body.contains("class=\"notice\"")
            && body.contains("Kamera gespeichert"),
        "expected success notice in gear out, got: {body}"
    );
    assert!(
        body.contains("Canon AE-1") || body.contains("gear-cameras-list"),
        "expected list OOB cue, got: {body}"
    );
}

/// REQ-014 −: invalid order number must show HTML error in the HTMX target.
#[tokio::test]
async fn st_014_n1_order_check_invalid_format_shows_error() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-014-n1", "st-user")
        .await
        .expect("user");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/order/check")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("order_number=not-an-order"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_text(res).await;
    assert!(
        body.contains("class=\"error\""),
        "expected .error feedback, got: {body}"
    );
}

/// REQ-014 −: empty camera label must surface German error in gear out slot.
#[tokio::test]
async fn st_014_n3_create_camera_empty_label_shows_error() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-014-n3", "st-user")
        .await
        .expect("user");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gear/cameras")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("label=+++"))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_text(res).await;
    assert!(
        body.contains("gear-cameras-out") && body.contains("class=\"error\""),
        "expected error in #gear-cameras-out, got: {body}"
    );
    assert!(
        body.contains("Kamera-Bezeichnung") || body.contains("eingeben"),
        "expected German empty-label cue, got: {body}"
    );
}

/// REQ-014 −: invalid lens aperture must show error in lenses out slot.
#[tokio::test]
async fn st_014_n4_create_lens_invalid_aperture_shows_error() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-014-n4", "st-user")
        .await
        .expect("user");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/gear/lenses")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("name=Summicron&focal_mm=50&aperture=0"))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_text(res).await;
    assert!(
        body.contains("gear-lenses-out") && body.contains("class=\"error\""),
        "expected error in #gear-lenses-out, got: {body}"
    );
    assert!(
        body.contains("Blende") || body.contains("größer"),
        "expected German aperture cue, got: {body}"
    );
}

/// REQ-014 −: cannot delete another user's ticket; must return visible error.
#[tokio::test]
async fn st_014_n5_delete_foreign_ticket_not_found_error() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let owner = db::upsert_discord_user(&pool, "st-014-n5-owner", "owner")
        .await
        .expect("owner");
    let other = db::upsert_discord_user(&pool, "st-014-n5-other", "other")
        .await
        .expect("other");
    let ticket = db::create_ticket(
        &pool,
        owner.id,
        "544850-103402",
        None,
        None,
        None,
        None,
        "PROCESSING",
        None,
    )
    .await
    .expect("ticket");
    let cookie = login_test_user(&app, other.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/tickets/{}", ticket.id))
                .header(header::COOKIE, cookie)
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body = body_text(res).await;
    assert!(
        body.contains("class=\"error\""),
        "expected .error feedback, got: {body}"
    );
}

/// REQ-014 −: convert with valid Secure-ID but PhotoPrism unset → user-visible unavailable.
#[tokio::test]
async fn st_014_n7_convert_without_photoprism_shows_error() {
    use crate::app::test_support::{login_test_user, test_app};
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-014-n7", "st-user")
        .await
        .expect("user");
    let ticket = db::create_ticket(
        &pool,
        user.id,
        "544850-103403",
        None,
        None,
        None,
        None,
        "PROCESSING",
        None,
    )
    .await
    .expect("ticket");
    let _camera = db::create_user_camera(&pool, user.id, "Praktika")
        .await
        .expect("camera");
    let cookie = login_test_user(&app, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/tickets/{}/convert", ticket.id))
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("secure_id=H5GGX3T5&camera_label=Praktika"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = body_text(res).await;
    assert!(
        body.contains("class=\"error\"") && body.contains("PhotoPrism"),
        "expected PhotoPrism unavailable error, got: {body}"
    );
}
