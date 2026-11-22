//! Typed wrappers around Tauri's global `window.__TAURI__` IPC bridge.

use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(event: &str, handler: &js_sys::Function) -> Result<JsValue, JsValue>;
}

fn to_js_args<A: Serialize>(args: &A) -> Result<JsValue, String> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    args.serialize(&serializer).map_err(|e| e.to_string())
}

fn js_error_to_string(err: JsValue) -> String {
    if let Some(s) = err.as_string() {
        return s;
    }
    if let Ok(json) = js_sys::JSON::stringify(&err) {
        if let Some(s) = json.as_string() {
            return s;
        }
    }
    format!("{err:?}")
}

pub async fn invoke<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, String> {
    let js_args = to_js_args(args)?;
    let val = tauri_invoke(cmd, js_args)
        .await
        .map_err(js_error_to_string)?;
    let val = if val.is_undefined() {
        JsValue::NULL
    } else {
        val
    };
    serde_wasm_bindgen::from_value(val).map_err(|e| format!("decode {cmd}: {e}"))
}

pub async fn invoke_vec_err<A: Serialize, R: DeserializeOwned>(
    cmd: &str,
    args: &A,
) -> Result<R, Vec<String>> {
    let js_args = to_js_args(args).map_err(|e| vec![e])?;
    let val = match tauri_invoke(cmd, js_args).await {
        Ok(v) => v,
        Err(err) => {
            if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<String>>(err.clone()) {
                return Err(list);
            }
            return Err(vec![js_error_to_string(err)]);
        }
    };
    let val = if val.is_undefined() {
        JsValue::NULL
    } else {
        val
    };
    serde_wasm_bindgen::from_value(val).map_err(|e| vec![format!("decode {cmd}: {e}")])
}

pub fn listen<T: DeserializeOwned + 'static>(event: &str, f: impl Fn(T) + 'static) {
    let event_name = event.to_string();
    let event_for_log = event_name.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let cb = Closure::<dyn FnMut(JsValue)>::new(move |ev: JsValue| {
            let payload =
                js_sys::Reflect::get(&ev, &JsValue::from_str("payload")).unwrap_or(JsValue::NULL);
            let payload = if payload.is_undefined() {
                JsValue::NULL
            } else {
                payload
            };
            match serde_wasm_bindgen::from_value::<T>(payload) {
                Ok(parsed) => f(parsed),
                Err(err) => {
                    web_sys::console::error_1(
                        &format!("[tospu] failed to decode {event_for_log}: {err}").into(),
                    );
                }
            }
        });
        let _ = tauri_listen(&event_name, cb.as_ref().unchecked_ref()).await;
        cb.forget();
    });
}
