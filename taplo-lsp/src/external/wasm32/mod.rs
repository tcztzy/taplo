use crate::{create_server, create_world, utils, World};
use anyhow::anyhow;
use futures::{Future, Sink};
use js_sys::Uint8Array;
use lsp_async_stub::{rpc::Message, Server};
use once_cell::sync::Lazy;
use std::{io, task::Poll};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
         $crate::external::log_info(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
         $crate::external::log_warn(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
         $crate::external::log_error(&format!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {  $crate::external::log_info(&format!($($arg)*)) }
    };
}

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn js_log_info(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    fn js_log_warn(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn js_log_error(s: &str);

    #[wasm_bindgen(js_namespace = global, js_name = sendMessage)]
    fn js_send_message(message: JsValue);

    #[wasm_bindgen(js_namespace = global, js_name = readFile, catch)]
    async fn js_read_file(path: &str) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = global, js_name = writeFile, catch)]
    async fn js_write_file(path: &str, data: JsValue) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = global, js_name = fileExists)]
    fn js_file_exists(path: &str) -> bool;

    #[wasm_bindgen(js_namespace = global, js_name = mkdir, catch)]
    fn js_mkdir(path: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = global, js_name = needsUpdate, catch)]
    fn js_needs_update(path: &str, new_date_ms: u64) -> Result<bool, JsValue>;

    #[wasm_bindgen(js_namespace = global, js_name = isAbsolutePath)]
    fn js_is_absolute_path(path: &str) -> bool;
}

struct ImplSend<T>(pub T);

// safety: we're in a WASM context with a single thread.
unsafe impl<T> Send for ImplSend<T> {} 
unsafe impl<T> Sync for ImplSend<T> {} 

static SERVER: Lazy<ImplSend<Server<World>>> = Lazy::new(|| ImplSend(create_server()));
static WORLD: Lazy<ImplSend<World>> = Lazy::new(|| ImplSend(create_world()));

#[wasm_bindgen]
pub async fn initialize() {
    utils::set_panic_hook();
}

#[wasm_bindgen]
pub fn message(message: JsValue) {
    log_debug!("in: {:?}", message);
    let msg = message.into_serde().unwrap();
    spawn(async move {
        SERVER
            .0
            .handle_message(WORLD.0.clone(), msg, MessageWriter)
            .await
            .unwrap();
    });
}

pub(crate) fn spawn<F: Future<Output = ()> + 'static>(fut: F) {
    spawn_local(fut)
}

pub(crate) fn log_info(s: &str) {
    js_log_info(s)
}

pub(crate) fn log_warn(s: &str) {
    js_log_warn(s)
}

pub(crate) fn log_error(s: &str) {
    js_log_error(s)
}

pub(crate) fn is_absolute_path(s: &str) -> bool {
    js_is_absolute_path(s)
}

pub(crate) async fn read_file(p: &str) -> Result<Vec<u8>, anyhow::Error> {
    let res: JsValue = js_read_file(p).await.map_err(|e| anyhow!("{:?}", e))?;
    Ok(Uint8Array::from(res).to_vec())
}

pub(crate) async fn write_file(p: &str, data: &[u8]) -> Result<(), anyhow::Error> {
    js_write_file(p, Uint8Array::from(data).into())
        .await
        .map_err(|e| anyhow!("{:?}", e))?;
    Ok(())
}

pub(crate) fn file_exists(p: &str) -> bool {
    js_file_exists(p)
}

pub(crate) fn mkdir(p: &str) -> Result<(), anyhow::Error> {
    js_mkdir(p).map_err(|e| anyhow!("{:?}", e))?;
    Ok(())
}

pub(crate) fn needs_update(p: &str, new_date_ms: u64) -> Result<bool, anyhow::Error> {
    js_needs_update(p, new_date_ms).map_err(|e| anyhow!("{:?}", e))
}

#[derive(Clone)]
struct MessageWriter;

impl Sink<Message> for MessageWriter {
    type Error = io::Error;

    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        log_debug!("out: {}", serde_json::to_string(&item).unwrap());
        js_send_message(JsValue::from_serde(&item).unwrap());
        Ok(())
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
