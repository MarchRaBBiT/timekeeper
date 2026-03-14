use std::fs;
use std::path::PathBuf;

fn frontend_file(path: &str) -> String {
    let mut full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    full_path.push(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", full_path.display()))
}

#[test]
fn security_headers_snippet_emits_header_based_clickjacking_protection() {
    let snippet = frontend_file("nginx-security-headers.conf");

    assert!(
        snippet.contains("add_header Content-Security-Policy")
            && snippet.contains("frame-ancestors 'none'")
            && snippet.contains("always;"),
        "security header snippet must send a header-based CSP with frame-ancestors 'none'"
    );
    assert!(
        snippet.contains("add_header X-Frame-Options \"DENY\" always;"),
        "security header snippet must emit X-Frame-Options: DENY"
    );
}

#[test]
fn html_files_do_not_keep_anti_framing_in_meta_csp() {
    for path in ["index.html", "index.html.template"] {
        let html = frontend_file(path);

        assert!(
            !html.contains("frame-ancestors"),
            "{path} should not claim anti-framing via meta CSP once headers are authoritative"
        );
    }
}

#[test]
fn nginx_conf_scopes_security_headers_to_browser_routes_only() {
    let nginx_conf = frontend_file("nginx.conf");

    assert!(
        nginx_conf
            .contains("location / {\n        include /etc/nginx/snippets/security-headers.conf;")
            && nginx_conf.contains(
                "location /pkg/ {\n        include /etc/nginx/snippets/security-headers.conf;"
            ),
        "browser-facing routes must include the shared security headers snippet"
    );
    assert!(
        !nginx_conf.contains(
            "location /api/ {\n        include /etc/nginx/snippets/security-headers.conf;"
        ),
        "/api/ responses should not inherit browser-only security headers from nginx"
    );
}

#[test]
fn dockerfile_generates_nginx_config_from_template_values() {
    let dockerfile = frontend_file("Dockerfile");

    assert!(
        dockerfile.contains("frontend/nginx.conf")
            && dockerfile.contains("nginx.conf.template")
            && dockerfile.contains("frontend/nginx-security-headers.conf")
            && dockerfile.contains("nginx-security-headers.conf.template")
            && dockerfile.contains("sed \"s/__CSP_CONNECT_SRC__/$escaped/g\" ./nginx.conf.template")
            && dockerfile.contains(
                "sed \"s/__CSP_CONNECT_SRC__/$escaped/g\" ./nginx-security-headers.conf.template"
            ),
        "Dockerfile must render nginx config and shared security header snippet from the same CSP connect-src placeholder wiring as index.html"
    );
}
