use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{DisplayTarget, Settings};

use crate::ipc;
use crate::settings::store::{EmptyArgs, Store};

const NAMED_PREFIX: &str = "named:";

fn display_target_value(target: &DisplayTarget) -> String {
    match target {
        DisplayTarget::Cursor => "cursor".to_string(),
        DisplayTarget::Primary => "primary".to_string(),
        DisplayTarget::Named { name } => format!("{NAMED_PREFIX}{name}"),
    }
}

fn parse_display_target(val: &str) -> DisplayTarget {
    if let Some(name) = val.strip_prefix(NAMED_PREFIX) {
        DisplayTarget::Named {
            name: name.to_string(),
        }
    } else if val == "primary" {
        DisplayTarget::Primary
    } else {
        DisplayTarget::Cursor
    }
}

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
        store.run_then(
            async move {
                ipc::invoke_vec_err::<_, Settings>(
                    "save_settings",
                    &SaveSettingsArgs { settings: next },
                )
                .await
                .map_err(|errs| errs.join("; "))
            },
            move |saved| {
                store.view.update(|opt| {
                    if let Some(v) = opt {
                        v.settings = saved;
                    }
                });
            },
        );
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
    let monitors = move || store.view.get().map(|v| v.monitors).unwrap_or_default();
    let display_target = move || {
        store
            .view
            .get()
            .map(|v| v.settings.display_target)
            .unwrap_or_default()
    };

    let save_display_target = move |target: DisplayTarget| {
        let Some(mut next) = store.view.get_untracked().map(|v| v.settings) else {
            return;
        };
        next.display_target = target;
        store.run_then(
            async move {
                ipc::invoke_vec_err::<_, Settings>(
                    "save_settings",
                    &SaveSettingsArgs { settings: next },
                )
                .await
                .map_err(|errs| errs.join("; "))
            },
            move |saved| {
                store.view.update(|opt| {
                    if let Some(v) = opt {
                        v.settings = saved;
                    }
                });
            },
        );
    };

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

                <hr class="divider" />

                <div class="control-row">
                    <div>
                        <h3>"Popup screen"</h3>
                        <p class="hint">"Choose which monitor Tospu pops up on."</p>
                    </div>
                    <select
                        class="field-input"
                        aria-label="Popup screen"
                        prop:value=move || display_target_value(&display_target())
                        on:change=move |ev| {
                            save_display_target(parse_display_target(&event_target_value(&ev)));
                        }
                    >
                        <option value="cursor">"Follow cursor"</option>
                        <option value="primary">"Primary display"</option>
                        {move || {
                            let connected = monitors();
                            let current = display_target();
                            let disconnected = match &current {
                                DisplayTarget::Named { name }
                                    if !connected.iter().any(|m| m.name == *name) =>
                                {
                                    Some(name.clone())
                                }
                                _ => None,
                            };

                            view! {
                                {connected
                                    .into_iter()
                                    .map(|m| {
                                        let val = format!("{NAMED_PREFIX}{}", m.name);
                                        let label = if m.is_primary {
                                            format!("{} ({}×{}) · Primary", m.name, m.width, m.height)
                                        } else {
                                            format!("{} ({}×{})", m.name, m.width, m.height)
                                        };
                                        view! { <option value=val>{label}</option> }
                                    })
                                    .collect_view()}
                                {disconnected.map(|name| {
                                    let val = format!("{NAMED_PREFIX}{name}");
                                    let label = format!("{name} (disconnected)");
                                    view! { <option value=val>{label}</option> }
                                })}
                            }
                        }}
                    </select>
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
                        store.run_then(
                            async {
                                ipc::invoke::<_, tospu_model::SettingsView>(
                                    "reload_files",
                                    &EmptyArgs {},
                                )
                                .await
                            },
                            move |fresh| store.view.set(Some(fresh)),
                        );
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
