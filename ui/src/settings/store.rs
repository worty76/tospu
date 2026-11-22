use std::future::Future;

use leptos::prelude::*;
use serde::Serialize;
use tospu_model::SettingsView;
use wasm_bindgen_futures::spawn_local;

use crate::ipc;

#[derive(Serialize)]
pub struct EmptyArgs {}

#[derive(Clone, Copy)]
pub struct Store {
    pub view: RwSignal<Option<SettingsView>>,
    pub error: RwSignal<Option<String>>,
}

impl Store {
    pub fn new() -> Self {
        let store = Self {
            view: RwSignal::new(None),
            error: RwSignal::new(None),
        };
        store.refresh();
        ipc::listen::<()>("tospu://data-changed", move |()| {
            store.refresh();
        });
        store
    }

    pub fn refresh(&self) {
        let view = self.view;
        let error = self.error;
        spawn_local(async move {
            match ipc::invoke::<_, SettingsView>("get_settings_view", &EmptyArgs {}).await {
                Ok(v) => view.set(Some(v)),
                Err(e) => error.set(Some(e)),
            }
        });
    }

    pub fn run<R: 'static>(&self, fut: impl Future<Output = Result<R, String>> + 'static) {
        self.run_then(fut, |_| {});
    }

    pub fn run_then<R: 'static>(
        &self,
        fut: impl Future<Output = Result<R, String>> + 'static,
        on_ok: impl FnOnce(R) + 'static,
    ) {
        let store = *self;
        spawn_local(async move {
            match fut.await {
                Ok(val) => {
                    store.error.set(None);
                    on_ok(val);
                    store.refresh();
                }
                Err(e) => store.error.set(Some(e)),
            }
        });
    }
}
