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
async fn st_008_c_admin_login_page_requires_discord() {
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
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(header::LOCATION).unwrap(),
        "/login"
    );
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
        "/login"
    );
}

#[tokio::test]
async fn st_008_g_api_users_htmx_unauthorized_without_discord() {
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
    // Anonymous → Discord login required (401); logged-in non-admin would be 403.
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn st_008_h_admin_login_positive_grants_dashboard() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-008-h", "admin-user")
        .await
        .expect("user");
    let user_cookie = login_test_user(&app, &pool, user.id).await;

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/login")
                .header(header::COOKIE, &user_cookie)
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
    let cookie = cookie_from(&login)
        .unwrap_or(user_cookie);

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
        body.contains("Admin") || body.contains("Allowlist") || body.contains("Nutzer"),
        "dashboard body unexpected"
    );
}

#[tokio::test]
async fn st_008_i_admin_login_negative_wrong_password() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-008-i", "admin-user")
        .await
        .expect("user");
    let user_cookie = login_test_user(&app, &pool, user.id).await;

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/login")
                .header(header::COOKIE, &user_cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("password=wrong-password"))
                .unwrap(),
        )
        .await
        .unwrap();

    let cookie = cookie_from(&login).unwrap_or(user_cookie);
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
    if login.status().is_success() {
        let body = body_text(login).await;
        assert!(
            body.contains("error")
                || body.contains("Fehler")
                || body.contains("Falsch")
                || body.contains("falsch")
                || body.contains("Passwort"),
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
    let cookie = login_test_user(&app, &pool, user.id).await;

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
    let cookie = login_test_user(&app, &pool, user.id).await;

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
    let cookie = login_test_user(&app, &pool, user.id).await;

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
    let cookie = login_test_user(&app, &pool, user.id).await;

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
    let cookie = login_test_user(&app, &pool, user.id).await;

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
    let cookie = login_test_user(&app, &pool, other.id).await;

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
    let cookie = login_test_user(&app, &pool, user.id).await;

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

/// REQ-015 −: denied Discord login creates no users row.
#[tokio::test]
async fn st_015_a_denied_oauth_creates_no_user_row() {
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let before = db::list_users(&pool).await.expect("list").len();

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__test/discord_login/999888777666")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(header::LOCATION).unwrap(),
        "/login?denied=1"
    );
    let after = db::list_users(&pool).await.expect("list").len();
    assert_eq!(before, after, "denied login must not create users row");
}

/// REQ-015 −: GET /admin without Discord session → /login.
#[tokio::test]
async fn st_015_b_admin_without_discord_redirects_login() {
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
    assert_eq!(res.headers().get(header::LOCATION).unwrap(), "/login");
}

/// REQ-015 −: Discord session without admin → no dashboard.
#[tokio::test]
async fn st_015_c_discord_user_not_admin_no_dashboard() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-015-c", "plain-user")
        .await
        .expect("user");
    let cookie = login_test_user(&app, &pool, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .header(header::COOKIE, cookie)
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

/// REQ-015 +: allowlist admin Discord session → /admin 200.
#[tokio::test]
async fn st_015_d_allowlist_admin_opens_dashboard() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-015-d", "admin-discord")
        .await
        .expect("user");
    db::upsert_discord_allowlist(&pool, "st-015-d", Some("admin-discord"), true, "test")
        .await
        .expect("allowlist");
    let cookie = login_test_user(&app, &pool, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/admin")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    assert!(body.contains("Allowlist") || body.contains("Discord-Allowlist"));
}

/// REQ-015 +: allowlisted user + password → admin.
#[tokio::test]
async fn st_015_e_allowlisted_user_password_admin() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "st-015-e", "elevated")
        .await
        .expect("user");
    db::upsert_discord_allowlist(&pool, "st-015-e", Some("elevated"), false, "test")
        .await
        .expect("allowlist");
    let user_cookie = login_test_user(&app, &pool, user.id).await;

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/login")
                .header(header::COOKIE, &user_cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("password=admin-secret"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(login.status().is_redirection());
    let cookie = cookie_from(&login).unwrap_or(user_cookie);

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
}

/// REQ-015 −: empty allowlist → login denied.
#[tokio::test]
async fn st_015_f_empty_allowlist_denies_login() {
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    assert!(
        db::list_discord_allowlist(&pool)
            .await
            .expect("list")
            .is_empty()
    );

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/__test/discord_login/111222333444")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(
        res.headers().get(header::LOCATION).unwrap(),
        "/login?denied=1"
    );
    assert!(
        db::find_by_discord_id(&pool, "111222333444")
            .await
            .expect("find")
            .is_none()
    );
}

/// REQ-015 +: denied login page shows German message.
#[tokio::test]
async fn st_015_login_denied_query_shows_message() {
    let (_dir, app, _, _) = test_app("admin-secret").await;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/login?denied=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    assert!(
        body.contains("nicht freigeschaltet") || body.contains("Allowlist"),
        "expected denied copy, got: {body}"
    );
}

/// REQ-015 −: last admin cannot be demoted via toggle.
#[tokio::test]
async fn st_015_g_last_admin_toggle_rejected() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "9001", "sole-admin")
        .await
        .expect("user");
    db::upsert_discord_allowlist(&pool, "9001", Some("sole-admin"), true, "test")
        .await
        .expect("allowlist");
    let cookie = login_test_user(&app, &pool, user.id).await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/allowlist/9001/admin")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = body_text(res).await;
    assert!(
        body.contains("letzte Admin") && body.contains("allowlist-list"),
        "expected last-admin error with list kept, got: {body}"
    );
    assert!(
        db::is_discord_allowlist_admin(&pool, "9001")
            .await
            .expect("admin check")
    );
}

/// REQ-015 +: allowlist add via admin HTMX.
#[tokio::test]
async fn st_015_h_allowlist_add_htmx() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "9002", "admin")
        .await
        .expect("user");
    db::upsert_discord_allowlist(&pool, "9002", Some("admin"), true, "test")
        .await
        .expect("allowlist");
    let cookie = login_test_user(&app, &pool, user.id).await;

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/allowlist")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("identity=9003"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    assert!(body.contains("9003"));
    assert!(
        db::is_discord_allowlisted(&pool, "9003")
            .await
            .expect("allowlisted")
    );

    // Add by Discord username (provisional until first OAuth).
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/allowlist")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("identity=coolphotog"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        db::is_discord_allowlisted_identity(&pool, "nope", Some("coolphotog"))
            .await
            .expect("username allowlisted")
    );

    // Re-add sole admin without checkbox must not demote.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/admin/allowlist")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header("HX-Request", "true")
                .body(Body::from("identity=9002"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(
        db::is_discord_allowlist_admin(&pool, "9002")
            .await
            .expect("still admin")
    );
}

/// REQ-015 −: removed from allowlist → session no longer authenticates.
#[tokio::test]
async fn st_015_i_revoked_allowlist_clears_session() {
    use crate::app::test_support::login_test_user;
    use crate::db;

    let (_dir, app, _, pool) = test_app("admin-secret").await;
    let user = db::upsert_discord_user(&pool, "9004", "revoked")
        .await
        .expect("user");
    let cookie = login_test_user(&app, &pool, user.id).await;
    db::delete_discord_allowlist(&pool, "9004")
        .await
        .expect("delete allowlist");

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/me")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_redirection());
    assert_eq!(res.headers().get(header::LOCATION).unwrap(), "/login");
}
