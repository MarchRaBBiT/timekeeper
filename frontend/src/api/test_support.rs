#[cfg(test)]
pub mod mock {
    use crate::api::client::{register_mock, MockResponse, TestResponder};
    use crate::api::ApiError;
    use reqwest::Method;
    use serde_json::Value;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    pub const GET: Method = Method::GET;
    pub const POST: Method = Method::POST;
    pub const PUT: Method = Method::PUT;
    pub const DELETE: Method = Method::DELETE;

    #[derive(Clone)]
    pub struct MockServer {
        inner: Arc<Mutex<Inner>>,
        base: String,
    }

    struct Inner {
        routes: Vec<Route>,
    }

    #[derive(Clone)]
    struct Route {
        method: Method,
        path: String,
        response: MockResponse,
    }

    impl MockServer {
        pub async fn start_async() -> Self {
            Self::start()
        }

        pub fn start() -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            Self {
                inner: Arc::new(Mutex::new(Inner { routes: Vec::new() })),
                base: format!("http://mock-{}", id),
            }
        }

        pub fn url(&self, path: &str) -> String {
            let base_url = format!("{}{}", self.base, path);
            register_mock(base_url.clone(), Arc::new(self.clone()));
            base_url
        }

        pub fn mock<F>(&self, f: F)
        where
            F: FnOnce(&mut When, &mut Then),
        {
            let mut when = When::default();
            let mut then = Then::default();
            f(&mut when, &mut then);

            let method = when
                .method
                .clone()
                .expect("mock requires method");
            let path = when.path.clone().expect("mock requires path");
            let response = MockResponse::json(
                then.status.unwrap_or(200),
                then.body.unwrap_or_else(|| serde_json::json!({})),
            );

            let mut inner = self.inner.lock().expect("mock lock");
            inner.routes.push(Route {
                method,
                path,
                response,
            });
        }
    }

    impl TestResponder for MockServer {
        fn respond(&self, request: &reqwest::Request) -> Result<MockResponse, ApiError> {
            let method = request.method();
            let path = request.url().path();
            let inner = self.inner.lock().map_err(|_| ApiError::unknown("mock lock"))?;

            let route = inner
                .routes
                .iter()
                .rev()
                .find(|route| route.method == *method && route.path == path)
                .cloned();

            route
                .map(|route| route.response)
                .ok_or_else(|| ApiError::unknown(format!("No mock for {} {}", method, path)))
        }
    }

    #[derive(Default)]
    pub struct When {
        method: Option<Method>,
        path: Option<String>,
    }

    impl When {
        pub fn method(&mut self, method: Method) -> &mut Self {
            self.method = Some(method);
            self
        }

        pub fn path(&mut self, path: &str) -> &mut Self {
            self.path = Some(path.to_string());
            self
        }
    }

    #[derive(Default)]
    pub struct Then {
        status: Option<u16>,
        body: Option<Value>,
    }

    impl Then {
        pub fn status(&mut self, status: u16) -> &mut Self {
            self.status = Some(status);
            self
        }

        pub fn json_body(&mut self, body: Value) -> &mut Self {
            self.body = Some(body);
            self
        }
    }
}
