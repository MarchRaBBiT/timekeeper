use std::fs;
use std::path::PathBuf;

fn frontend_file(path: &str) -> String {
    let mut full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    full_path.push(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", full_path.display()))
}

#[test]
fn nginx_conf_emits_header_based_clickjacking_protection() {
    let nginx_conf = frontend_file("nginx.conf");

    assert!(
        nginx_conf.contains("add_header Content-Security-Policy")
            && nginx_conf.contains("frame-ancestors 'none'")
            && nginx_conf.contains("always;"),
        "nginx.conf must send a header-based CSP with frame-ancestors 'none'"
    );
    assert!(
        nginx_conf.contains("add_header X-Frame-Options \"DENY\" always;"),
        "nginx.conf must emit X-Frame-Options: DENY"
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
fn dockerfile_generates_nginx_config_from_template_values() {
    let dockerfile = frontend_file("Dockerfile");

    assert!(
        dockerfile.contains("frontend/nginx.conf")
            && dockerfile.contains("nginx.conf.template")
            && dockerfile.contains("sed \"s/__CSP_CONNECT_SRC__/$escaped/g\" ./nginx.conf.template"),
        "Dockerfile must render nginx.conf with the same CSP connect-src placeholder wiring as index.html"
    );
}
