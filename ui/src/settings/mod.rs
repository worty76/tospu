pub mod character;
pub mod general;
pub mod messages;
pub mod schedule;
pub mod store;
pub mod welcome;

use leptos::prelude::*;

use crate::settings::character::CharacterTab;
use crate::settings::general::GeneralTab;
use crate::settings::messages::MessagesTab;
use crate::settings::schedule::ScheduleTab;
use crate::settings::store::Store;
use crate::settings::welcome::WelcomeView;

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

    let show_welcome = move || {
        store
            .view
            .get()
            .map(|v| !v.settings.onboarded)
            .unwrap_or(false)
    };

    view! {
        {move || {
            if show_welcome() {
                view! { <WelcomeView /> }.into_any()
            } else {
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
                                Tab::Schedule => view! { <ScheduleTab /> }.into_any(),
                                Tab::Character => view! { <CharacterTab /> }.into_any(),
                                Tab::General => view! { <GeneralTab /> }.into_any(),
                            }}
                        </main>
                    </div>
                }
                .into_any()
            }
        }}
    }
}
