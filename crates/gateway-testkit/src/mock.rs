use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

pub use wiremock::MockServer;

enum Body {
    None,
    Json(serde_json::Value),
    Text(String),
    /// Pre-framed SSE payload, served with `text/event-stream`.
    Sse(String),
}

/// Builds and mounts a single wiremock `Mock` — generalizes the
/// `Mock::given(method(...)).and(path(...)).respond_with(...).expect(...).mount(...)`
/// chain that used to be hand-written per test file.
///
/// Defaults: `POST`, any path, `200`, empty body, no call-count expectation.
pub struct MockProviderBuilder {
    method: String,
    path: Option<String>,
    status: u16,
    body: Body,
    expect: Option<u64>,
    delay: Option<std::time::Duration>,
    /// Top-level JSON keys that must be ABSENT from the request body for
    /// this mock to match.
    absent_body_fields: Vec<String>,
}

impl Default for MockProviderBuilder {
    fn default() -> Self {
        Self {
            method: "POST".to_string(),
            path: None,
            status: 200,
            body: Body::None,
            expect: None,
            delay: None,
            absent_body_fields: Vec::new(),
        }
    }
}

impl MockProviderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn method(mut self, method: &str) -> Self {
        self.method = method.to_string();
        self
    }

    /// Omit to match any path — needed for a fallback test simulating a
    /// vendor that 404s regardless of the request path.
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    pub fn status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn json_body(mut self, body: serde_json::Value) -> Self {
        self.body = Body::Json(body);
        self
    }

    pub fn text_body(mut self, body: impl Into<String>) -> Self {
        self.body = Body::Text(body.into());
        self
    }

    /// Serves an SSE stream built from `data:`-payload strings — each entry
    /// becomes one `data: <payload>\n\n` frame, and a trailing `[DONE]` is
    /// appended the way OpenAI-wire providers send one.
    ///
    /// Pass already-formatted frames via `raw_sse_body` when a test needs
    /// `event:` lines (Anthropic) or deliberately odd framing.
    pub fn sse_body(self, payloads: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut out = String::new();
        for payload in payloads {
            out.push_str(&format!("data: {}\n\n", payload.into()));
        }
        out.push_str("data: [DONE]\n\n");
        self.raw_sse_body(out)
    }

    /// Serves `body` verbatim as `text/event-stream` — full control over
    /// framing, for providers that name their events or for testing a
    /// truncated stream.
    pub fn raw_sse_body(mut self, body: impl Into<String>) -> Self {
        self.body = Body::Sse(body.into());
        self
    }

    /// Only match a request whose JSON body does NOT contain `field` at the
    /// top level. Turns "we didn't send X" into a matching condition rather
    /// than something the test has to remember to assert — an unexpected
    /// field means no mock matches and the call fails loudly.
    pub fn without_body_field(mut self, field: &str) -> Self {
        self.absent_body_fields.push(field.to_string());
        self
    }

    /// Holds the response back before sending it — used to prove the
    /// gateway streams rather than buffers, by making a buffered
    /// implementation measurably slower to first byte.
    pub fn delay(mut self, delay: std::time::Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    /// Omit for an unbounded mock (no call-count assertion on drop).
    pub fn expect(mut self, calls: u64) -> Self {
        self.expect = Some(calls);
        self
    }

    fn build(self) -> Mock {
        let mut response = ResponseTemplate::new(self.status);
        response = match self.body {
            Body::None => response,
            Body::Json(v) => response.set_body_json(v),
            Body::Text(s) => response.set_body_string(s),
            Body::Sse(s) => response.set_body_raw(s.into_bytes(), "text/event-stream"),
        };
        if let Some(delay) = self.delay {
            response = response.set_delay(delay);
        }

        let mut mock = Mock::given(method(self.method.as_str()));
        if let Some(p) = &self.path {
            mock = mock.and(path(p.as_str()));
        }
        for field in self.absent_body_fields {
            mock = mock.and(BodyFieldAbsent(field));
        }
        let mut mock = mock.respond_with(response);
        if let Some(n) = self.expect {
            mock = mock.expect(n);
        }
        mock
    }

    /// Starts a fresh `MockServer`, mounts this mock on it, and returns the
    /// server handle (for `.uri()` / `.received_requests()`).
    pub async fn mount(self) -> MockServer {
        let server = MockServer::start().await;
        self.build().mount(&server).await;
        server
    }

    /// Mounts this mock onto a `MockServer` you already started — for tests
    /// stacking more than one mock on the same server.
    pub async fn mount_on(self, server: &MockServer) {
        self.build().mount(server).await;
    }
}

/// wiremock matcher for "this top-level JSON key is not present in the body".
/// A non-JSON body counts as absent — nothing to reject.
struct BodyFieldAbsent(String);

impl wiremock::Match for BodyFieldAbsent {
    fn matches(&self, request: &wiremock::Request) -> bool {
        match serde_json::from_slice::<serde_json::Value>(&request.body) {
            Ok(v) => v.get(&self.0).is_none(),
            Err(_) => true,
        }
    }
}
