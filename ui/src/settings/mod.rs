pub mod messages;
pub mod store;

use leptos::prelude::*;

use crate::settings::messages::MessagesTab;
use crate::settings::store::Store;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Messages,
    Schedule,
    Character,
    General,
}

#[component]
pub fn SettingsApp() -> impl IntoView {
    let store = Store::new();
    provide_context(store);

    let tab = RwSignal::new(Tab::Messages);

    view! {
        <div class="settings-shell">
            <nav class="settings-nav" role="tablist" aria-label="Settings sections">
                <h1 class="nav-title">"Tospu"</h1>
                <button
                    type="button"
                    role="tab"
                    class=move || if tab.get() == Tab::Messages { "nav-tab active" } else { "nav-tab" }
                    aria-selected=move || if tab.get() == Tab::Messages { "true" } else { "false" }
                    on:click=move |_| tab.set(Tab::Messages)
                >
                    "Messages"
                </button>
                <button
                    type="button"
                    role="tab"
                    class=move || if tab.get() == Tab::Schedule { "nav-tab active" } else { "nav-tab" }
                    aria-selected=move || if tab.get() == Tab::Schedule { "true" } else { "false" }
                    on:click=move |_| tab.set(Tab::Schedule)
                >
                    "Schedule"
                </button>
                <button
                    type="button"
                    role="tab"
                    class=move || if tab.get() == Tab::Character { "nav-tab active" } else { "nav-tab" }
                    aria-selected=move || if tab.get() == Tab::Character { "true" } else { "false" }
                    on:click=move |_| tab.set(Tab::Character)
                >
                    "Character"
                </button>
                <button
                    type="button"
                    role="tab"
                    class=move || if tab.get() == Tab::General { "nav-tab active" } else { "nav-tab" }
                    aria-selected=move || if tab.get() == Tab::General { "true" } else { "false" }
                    on:click=move |_| tab.set(Tab::General)
                >
                    "General"
                </button>
            </nav>

            <main class="settings-main">
                {move || {
                    store.error.get().map(|err| {
                        view! {
                            <div class="error-banner" role="alert">
                                <span>{err}</span>
                                <button
                                    type="button"
                                    class="banner-close"
                                    aria-label="Dismiss error"
                                    on:click=move |_| store.error.set(None)
                                >
                                    "×"
                                </button>
                            </div>
                        }
                    })
                }}

                {move || match tab.get() {
                    Tab::Messages => view! { <MessagesTab /> }.into_any(),
                    Tab::Schedule => view! { <p class="empty-state">"Schedule coming next."</p> }.into_any(),
                    Tab::Character => view! { <p class="empty-state">"Character coming next."</p> }.into_any(),
                    Tab::General => view! { <p class="empty-state">"General coming next."</p> }.into_any(),
                }}
            </main>
        </div>
    }
}
