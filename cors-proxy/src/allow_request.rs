use std::collections::HashMap;

use axum::http::Request;
use serde_qs::from_str;

struct Url<'l> {
    pub pathname: &'l str,
    pub query: HashMap<String, String>,
}

impl<'l> Url<'l> {
    pub fn new<B>(req: &'l Request<B>) -> Url {
        let map: HashMap<String, String> = from_str(req.uri().query().unwrap_or_default()).unwrap_or_default();
        return Url { pathname: req.uri().path(), query: map };
    }
}

trait ChecksHeaders {
    fn header_contains(&self, header_name: &str, contains: &str) -> bool;
    fn header_matches(&self, header_name: &str, predicate: &dyn Fn(&str) -> bool) -> bool;
}

impl<B> ChecksHeaders for Request<B> {
    fn header_contains(&self, header_name: &str, contains: &str) -> bool {
        self.header_matches(header_name, &|v: &str| v.contains(contains))
    }

    fn header_matches(&self, header_name: &str, predicate: &dyn Fn(&str) -> bool) -> bool {
        let header_value = self.headers().get(header_name).map(|h| h.to_str().ok()).flatten();
        header_value.map(predicate).unwrap_or(false)
    }
}

fn is_preflight_info_refs<B>(req: &Request<B>, u: &Url) -> bool {
    req.method() == "OPTIONS"
        && u.pathname.ends_with("/info/refs")
        && (u.query.get("service")
        .map(|s| s == "git-upload-pack" || s == "git-receive-pack").unwrap_or(false))
}

fn is_info_refs<B>(req: &Request<B>, u: &Url) -> bool {
    req.method() == "GET"
        && u.pathname.ends_with("/info/refs")
        && (u.query.get("service")
        .map(|s| s == "git-upload-pack" || s == "git-receive-pack").unwrap_or(false))
}

fn is_preflight_pull<B>(req: &Request<B>) -> bool {
    req.method() == "OPTIONS"
        && req.headers().get("access-control-request-headers").unwrap().to_str().unwrap_or("").contains("content-type")
        && req.uri().path().ends_with("git-upload-pack")
}

fn is_pull<B>(req: &Request<B>) -> bool {
    req.method() == "POST"
        && req.headers().get("content-type").unwrap() == "application/x-git-upload-pack-request"
        && req.uri().path().ends_with("git-upload-pack")
}

fn is_preflight_push<B>(req: &Request<B>) -> bool {
    req.method() == "OPTIONS"
        && req.header_contains("access-control-request-headers", "content-type")
        && req.uri().path().ends_with("git-receive-pack")
}

fn is_push<B>(req: &Request<B>) -> bool {
    req.method() == "POST"
        && req.header_matches("content-type",
                              &|s| s == "application/x-git-receive-pack-request")
        && req.uri().path().ends_with("git-receive-pack")
}

#[allow(dead_code)]
pub fn allow<B>(req: &Request<B>) -> bool {
    let u = &Url::new(&req);
    return is_preflight_info_refs(req, u) ||
        is_info_refs(req, u) ||
        is_preflight_pull(req) ||
        is_pull(req) ||
        is_preflight_push(req) ||
        is_push(req);
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, Uri};
    use maplit::hashmap;

    use super::*;

    /// we use the static lifetime here to enforce that strings passed are string literals
    /// if they were not string literals, they could be dropped before the Request would
    /// which would lead to uses after free
    fn build_request(method: Method,
                     url: &'static str,
                     headers: Option<HashMap<String, String>>) -> Request<()> {
        let mut builder = Request::builder().method(method).uri(Uri::from_static(url));
        if let Some(headers) = headers {
            for (key, value) in headers {
                builder = builder.header(key, value);
            }
        }
        builder.body(()).unwrap()
    }

    trait ToStringCapableMap {
        fn to_string_map(&self) -> HashMap<String, String>;
    }

    impl ToStringCapableMap for HashMap<&str, &str> {
        fn to_string_map(&self) -> HashMap<String, String> {
            self.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        }
    }

    #[test]
    fn test_allow_preflight_info_refs() {
        let req = build_request(Method::OPTIONS, "/test/info/refs?service=git-upload-pack", None);

        assert_eq!(is_preflight_info_refs(&req, &Url::new(&req)), true);
    }

    #[cfg(test)]
    mod allow_preflight_info_refs_tests {
        use axum::http::{Method, Uri};
        use maplit::hashmap;

        use super::*;

        #[test]
        fn test_is_preflight_info_refs_returns_false_because_method_was_not_options() {
            let req = Request::builder().method(Method::GET).body(()).unwrap();
            let url = Url { pathname: "/test/info/refs", query: HashMap::new() };
            assert_eq!(is_preflight_info_refs(&req, &url), false);
        }

        #[test]
        fn test_is_preflight_info_refs_returns_false_because_pathname_didnt_end_with_info_refs() {
            let req = build_request(Method::OPTIONS, "/test", None);
            let url = Url { pathname: "/test", query: HashMap::new() };
            assert_eq!(is_preflight_info_refs(&req, &url), false);
        }

        #[test]
        fn test_is_preflight_info_refs_returns_false_because_service_query_parameter_didnt_exist() {
            let req = build_request(Method::OPTIONS, "/test/info/refs", None);
            assert_eq!(is_preflight_info_refs(&req, &Url::new(&req)), false);
        }

        #[test]
        fn test_is_preflight_info_refs_returns_false_because_service_query_parameter_was_not_git_upload_or_receive_pack() {
            let req = build_request(Method::OPTIONS, "/test/info/refs?service=invalid", None);
            assert_eq!(is_preflight_info_refs(&req, &Url::new(&req)), false);
        }

        #[test]
        fn test_is_preflight_info_refs_returns_true_because_service_query_parameter_was_git_upload_pack() {
            let req = Request::builder().method(Method::OPTIONS).uri(Uri::from_static("/test/info/refs?service=git-upload-pack")).body(()).unwrap();
            let url = Url { pathname: "/test/info/refs", query: hashmap!["service".to_string() => "git-upload-pack".to_string()] };
            assert_eq!(is_preflight_info_refs(&req, &url), true);
        }

        #[test]
        fn test_is_preflight_info_refs_returns_true_because_service_query_parameter_was_git_receive_pack() {
            let req = Request::builder().method(Method::OPTIONS).uri(Uri::from_static("/test/info/refs?service=git-receive-pack")).body(()).unwrap();
            let url = Url { pathname: "/test/info/refs", query: hashmap!["service" => "git-receive-pack"].to_string_map() };
            assert_eq!(is_preflight_info_refs(&req, &url), true);
        }
    }


    #[test]
    fn test_is_info_refs() {
        let req = build_request(Method::GET, "/test/info/refs?service=git-upload-pack", Some(hashmap!["Content-Type" => "application/json"].to_string_map()));
        assert_eq!(is_info_refs(&req, &Url::new(&req)), true);
    }

    #[test]
    fn test_is_preflight_pull() {
        let req = build_request(Method::OPTIONS, "/test/git-upload-pack", Some(hashmap!["access-control-request-headers" => "content-type"].to_string_map()));
        assert_eq!(is_preflight_pull(&req), true);
    }

    #[test]
    fn test_is_pull() {
        let req = build_request(Method::POST, "/test/git-upload-pack", Some(hashmap!["Content-Type" => "application/x-git-upload-pack-request"].to_string_map()));
        assert_eq!(is_pull(&req), true);
    }

    #[test]
    fn test_allow_preflight_push() {
        let req = build_request(Method::OPTIONS, "/test/git-receive-pack", Some(hashmap!["Content-Type" => "application/json", "Access-Control-Request-Headers" => "content-type"].to_string_map()));
        assert_eq!(is_preflight_push(&req), true);
    }

    #[test]
    fn test_allow_push() {
        let req = build_request(Method::POST, "/test/git-receive-pack", Some(hashmap!["Content-Type" => "application/x-git-receive-pack-request"].to_string_map()));
        assert_eq!(is_push(&req), true);
    }
}
