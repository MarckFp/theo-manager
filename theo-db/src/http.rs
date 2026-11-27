//! One HTTP POST, the same way on every platform: `fetch` in the browser, `ureq` on a background
//! thread on desktop and phone (so the app does not freeze while it waits).

use crate::DbError;

/// POSTs `body` to `url` with `headers`. Returns the status and the body text. Any status is
/// returned (the caller decides); only a failed connection is an error.
pub(crate) async fn post(url: &str, headers: &[(&str, String)], body: String) -> Result<(u16, String), DbError> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let fail = |_| DbError::Storage("The server could not be reached.".into());
        // A browser hides why a request failed: no network, a wrong address, or a server that
        // does not allow this app's address (CORS) all look the same.
        let unreachable = |_| {
            let origin = web_sys::window().and_then(|w| w.location().origin().ok()).unwrap_or_default();
            DbError::Unreachable(format!(
                "The server could not be reached. Check the address and your connection. If they are right, the server's owner must allow this app's address ({origin}) for browser access (CORS)."
            ))
        };
        let opts = web_sys::RequestInit::new();
        opts.set_method("POST");
        opts.set_body(&wasm_bindgen::JsValue::from_str(&body));
        let h = web_sys::Headers::new().map_err(fail)?;
        for (k, v) in headers {
            h.set(k, v).map_err(fail)?;
        }
        opts.set_headers(&h);
        let request = web_sys::Request::new_with_str_and_init(url, &opts).map_err(fail)?;
        let window = web_sys::window().ok_or_else(|| DbError::Storage("No browser window.".into()))?;
        let resp = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await.map_err(unreachable)?;
        let resp: web_sys::Response = resp.dyn_into().map_err(fail)?;
        let status = resp.status();
        let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(fail)?).await.map_err(fail)?;
        Ok((status, text.as_string().unwrap_or_default()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let url = url.to_string();
        let headers: Vec<(String, String)> = headers.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        let (tx, rx) = futures_channel::oneshot::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<(u16, String), String> {
                let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).build().into();
                let mut req = agent.post(&url);
                for (k, v) in &headers {
                    req = req.header(k.as_str(), v.as_str());
                }
                let mut resp = req.send(body).map_err(|e| e.to_string())?;
                let status = resp.status().as_u16();
                let text = resp.body_mut().with_config().limit(256 * 1024 * 1024).read_to_string().map_err(|e| e.to_string())?;
                Ok((status, text))
            })();
            let _ = tx.send(result);
        });
        rx.await.map_err(|_| DbError::Storage("The request stopped.".into()))?.map_err(|e| DbError::Unreachable(format!("The server could not be reached. Check the address and your connection. ({e})")))
    }
}
