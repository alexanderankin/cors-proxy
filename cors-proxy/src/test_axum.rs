#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    pub fn test() {
        let mut headers = HeaderMap::new();
        headers.insert("X-My-Header", HeaderValue::from_static("my value"));

        // let get = headers.get("x-my-header")
        let get = headers.get("x-my-header")
            .map(|f| f.to_str() )
            .map(|f| f.unwrap_or(&""))
            .unwrap_or(&"");

        println!("{get}");

    }
}
