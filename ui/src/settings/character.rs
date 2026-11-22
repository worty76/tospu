use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{PackSummary, Settings};

use crate::ipc;
use crate::popup::sprite::SpriteStill;
use crate::settings::store::{EmptyArgs, Store};

#[derive(Serialize)]
struct SaveSettingsArgs {
    settings: Settings,
}

#[component]
pub fn PackGrid<F>(
    packs: Signal<Vec<PackSummary>>,
    active_pack: Signal<String>,
    on_select: F,
) -> impl IntoView
where
    F: Fn(String) + Copy + Send + Sync + 'static,
{
    view! {
        <div class="pack-grid">
            <For
                each=move || packs.get()
                key=|p| (p.id.clone(), p.error.clone())
                children=move |pack| {
                    let id = pack.id.clone();
                    let id_for_click = pack.id.clone();
                    let broken = pack.error.is_some();
                    let is_selected = move || !broken && active_pack.get() == id;

                    view! {
                        <button
                            type="button"
                            class=move || {
                                if broken {
                                    "pack-card broken"
                                } else if is_selected() {
                                    "pack-card selected"
                                } else {
                                    "pack-card"
                                }
                            }
                            disabled=broken
                            on:click=move |_| {
                                if !broken {
                                    on_select(id_for_click.clone());
                                }
                            }
                        >
                            <div class="pack-preview">
                                {match pack.view {
                                    Some(pv) => view! { <SpriteStill pack=pv height=72.0 /> }.into_any(),
                                    None => view! { <span class="broken-icon">"!"</span> }.into_any(),
                                }}
                            </div>
                            <div class="pack-name">{pack.name}</div>
                            {pack.author.map(|a| {
                                view! { <div class="pack-author">{format!("by {a}")}</div> }
                            })}
                            {pack.error.map(|err| {
                                view! { <div class="pack-error">{err}</div> }
                            })}
                        </button>
                    }
                }
            />
        </div>
    }
}

#[component]
pub fn CharacterTab() -> impl IntoView {
    let store = use_context::<Store>().expect("Store provided");

    let packs = Signal::derive(move || store.view.get().map(|v| v.packs).unwrap_or_default());
    let active_pack = Signal::derive(move || {
        store
            .view
            .get()
            .map(|v| v.settings.active_pack)
            .unwrap_or_default()
    });

    let select_pack = move |pack_id: String| {
        let Some(mut next) = store.view.get_untracked().map(|v| v.settings) else {
            return;
        };
        next.active_pack = pack_id;
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
        <section class="tab-panel" aria-label="Character">
            <header class="panel-header">
                <h2>"Character"</h2>
            </header>

            <PackGrid packs=packs active_pack=active_pack on_select=select_pack />

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
                            ipc::invoke::<_, ()>("open_sprites_folder", &EmptyArgs {}).await
                        });
                    }
                >
                    "Open sprites folder"
                </button>
                <button
                    type="button"
                    class="btn secondary"
                    on:click=move |_| {
                        store.run(async {
                            ipc::invoke::<_, ()>("open_pack_guide", &EmptyArgs {}).await
                        });
                    }
                >
                    "How to make your own"
                </button>
            </div>
        </section>
    }
}
