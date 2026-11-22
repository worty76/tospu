use leptos::prelude::*;
use serde::Serialize;
use tospu_model::Settings;

use crate::ipc;
use crate::settings::store::{EmptyArgs, Store};

#[derive(Serialize)]
struct LaunchArgs {
    enabled: bool,
}

#[derive(Serialize)]
struct SaveSettingsArgs {
    settings: Settings,
}

#[component]
pub fn GeneralTab() -> impl IntoView {
    let store = use_context::<Store>().expect("Store provided");

    let save_toggle = move |mutate: fn(&mut Settings, bool), val: bool| {
        let Some(mut next) = store.view.get_untracked().map(|v| v.settings) else {
            return;
        };
        mutate(&mut next, val);
        store.run(async move {
            ipc::invoke_vec_err::<_, Settings>(
                "save_settings",
                &SaveSettingsArgs { settings: next },
            )
            .await
            .map_err(|errs| errs.join("; "))
        });
    };

    let launch_at_login = move || store.view.get().map(|v| v.launch_at_login).unwrap_or(false);
    let quiet_mode = move || {
        store
            .view
            .get()
            .map(|v| v.settings.quiet_mode)
            .unwrap_or(false)
    };
    let privacy_blur = move || {
        store
            .view
            .get()
            .map(|v| v.settings.privacy_blur)
            .unwrap_or(true)
    };
    let shortcut = move || {
        store
            .view
            .get()
            .map(|v| v.shortcut_label)
            .unwrap_or_default()
    };
    let problems = move || store.view.get().map(|v| v.problems).unwrap_or_default();
    let version = move || store.view.get().map(|v| v.version).unwrap_or_default();

    view! {
        <section class="tab-panel" aria-label="General">
            <header class="panel-header">
                <h2>"General"</h2>
            </header>

            <div class="card">
                <div class="control-row">
                    <div>
                        <h3>"Launch at login"</h3>
                        <p class="hint">"Start Tospu automatically when you log in."</p>
                    </div>
                    <label class="switch">
                        <input
                            type="checkbox"
                            prop:checked=launch_at_login
                            on:change=move |ev| {
                                let enabled = event_target_checked(&ev);
                                store.run(async move {
                                    ipc::invoke::<_, bool>(
                                        "set_launch_at_login",
                                        &LaunchArgs { enabled },
                                    )
                                    .await
                                });
                            }
                        />
                        <span class="slider"></span>
                    </label>
                </div>

                <hr class="divider" />

                <div class="control-row">
                    <div>
                        <h3>"Quiet mode"</h3>
                        <p class="hint">
                            {move || format!(
                                "Show \"Keep going.\" until clicked. Shortcut: {}",
                                shortcut()
                            )}
                        </p>
                    </div>
                    <label class="switch">
                        <input
                            type="checkbox"
                            prop:checked=quiet_mode
                            on:change=move |ev| {
                                save_toggle(|s, v| s.quiet_mode = v, event_target_checked(&ev));
                            }
                        />
                        <span class="slider"></span>
                    </label>
                </div>

                <hr class="divider" />

                <div class="control-row">
                    <div>
                        <h3>"Hide message text until hover"</h3>
                        <p class="hint">"Blurs your messages in this window so passersby can't read them."</p>
                    </div>
                    <label class="switch">
                        <input
                            type="checkbox"
                            prop:checked=privacy_blur
                            on:change=move |ev| {
                                save_toggle(|s, v| s.privacy_blur = v, event_target_checked(&ev));
                            }
                        />
                        <span class="slider"></span>
                    </label>
                </div>
            </div>

            <div class="button-row">
                <button
                    type="button"
                    class="btn primary"
                    on:click=move |_| {
                        store.run(async {
                            ipc::invoke::<_, ()>("show_test_popup", &EmptyArgs {}).await
                        });
                    }
                >
                    "Show a test popup"
                </button>
                <button
                    type="button"
                    class="btn secondary"
                    on:click=move |_| {
                        store.run(async {
                            ipc::invoke::<_, ()>("open_data_folder", &EmptyArgs {}).await
                        });
                    }
                >
                    "Open data folder"
                </button>
                <button
                    type="button"
                    class="btn secondary"
                    on:click=move |_| {
                        store.run(async {
                            ipc::invoke::<_, serde_json::Value>("reload_files", &EmptyArgs {}).await
                        });
                    }
                >
                    "Reload files"
                </button>
            </div>

            {move || {
                let list = problems();
                if list.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div class="card problems-card">
                            <h3>"Problems"</h3>
                            <ul class="problems-list">
                                {list
                                    .into_iter()
                                    .map(|p| view! { <li>{p}</li> })
                                    .collect_view()}
                            </ul>
                        </div>
                    })
                }
            }}

            <footer class="version-footer">
                {move || format!("Tospu {}", version())}
            </footer>
        </section>
    }
}
