pub mod bubble;
pub mod sprite;

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use serde::Serialize;
use tospu_model::{
    typed, AnimationName, Effect, FrameClock, PackView, PopupTimeline, Rect, ShowPayload,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::ipc;
use crate::popup::bubble::Bubble;
use crate::popup::sprite::Sprite;

#[derive(Serialize)]
struct SetHitRectArgs {
    rect: Option<Rect>,
}

#[derive(Serialize)]
struct EmptyArgs {}

struct AnimState {
    clock: FrameClock,
    start_ms: f64,
    last_cycles: u32,
    done_reported: bool,
}

struct TypingState {
    full_text: String,
    start_ms: f64,
}

struct Driver {
    timeline: Option<PopupTimeline>,
    pack: Option<PackView>,
    anim: Option<AnimState>,
    typing: Option<TypingState>,
    hovered: bool,
    last_frame_ms: Option<f64>,
    raf_running: bool,
    reduced_motion: bool,

    set_stage_visible: WriteSignal<bool>,
    set_pack: WriteSignal<Option<PackView>>,
    set_anim_name: WriteSignal<AnimationName>,
    set_sprite_frame: WriteSignal<u32>,
    set_bubble_visible: WriteSignal<bool>,
    set_bubble_text: WriteSignal<String>,
    set_bubble_author: WriteSignal<Option<String>>,
    bubble_ref: NodeRef<leptos::html::Div>,
    sprite_host_ref: NodeRef<leptos::html::Div>,
}

impl Driver {
    fn start_show(&mut self, payload: ShowPayload, now_ms: f64) {
        if self.timeline.as_ref().is_some_and(PopupTimeline::is_active) {
            return;
        }
        let mood = payload
            .pack
            .as_ref()
            .map(|p| p.manifest.animation_for_kind(payload.message.kind))
            .unwrap_or(AnimationName::Idle);

        self.pack = payload.pack.clone();
        self.set_pack.set(payload.pack);
        self.set_stage_visible.set(true);
        self.set_bubble_visible.set(false);
        self.set_bubble_text.set(String::new());
        self.set_bubble_author.set(None);
        self.anim = None;
        self.typing = None;
        self.last_frame_ms = Some(now_ms);

        let (tl, effects) = PopupTimeline::start(
            payload.message,
            payload.quiet,
            payload.display_seconds,
            mood,
        );
        self.timeline = Some(tl);
        self.run_effects(effects, now_ms);
    }

    fn start_anim(&mut self, name: AnimationName, now_ms: f64) {
        self.set_anim_name.set(name);
        self.set_sprite_frame.set(0);
        if let Some(pack) = &self.pack {
            let spec = pack.manifest.resolve_animation(name);
            let mut clock = FrameClock::from(spec);
            if name == AnimationName::Idle {
                clock.looped = true;
            }
            self.anim = Some(AnimState {
                clock,
                start_ms: now_ms,
                last_cycles: 0,
                done_reported: false,
            });
        } else {
            self.anim = None;
            if let Some(tl) = &mut self.timeline {
                let eff = tl.on_animation_done();
                self.run_effects(eff, now_ms);
            }
        }
    }

    fn run_effects(&mut self, effects: Vec<Effect>, now_ms: f64) {
        for eff in effects {
            match eff {
                Effect::PlayEnter => self.start_anim(AnimationName::Enter, now_ms),
                Effect::PlayMood(mood) => self.start_anim(mood, now_ms),
                Effect::PlayIdleLoop => self.start_anim(AnimationName::Idle, now_ms),
                Effect::PlayExit => self.start_anim(AnimationName::Exit, now_ms),
                Effect::Say { text, author } => {
                    self.set_bubble_visible.set(true);
                    self.set_bubble_author.set(author);
                    if self.reduced_motion {
                        self.set_bubble_text.set(text);
                        self.typing = None;
                        if let Some(tl) = &mut self.timeline {
                            let next = tl.on_typing_done();
                            self.run_effects(next, now_ms);
                        }
                    } else {
                        self.set_bubble_text.set(String::new());
                        self.typing = Some(TypingState {
                            full_text: text,
                            start_ms: now_ms,
                        });
                    }
                }
                Effect::ReportHitRect => self.report_hit_rect(),
                Effect::HideBubble => {
                    self.typing = None;
                    self.set_bubble_visible.set(false);
                }
                Effect::Finished => {
                    self.anim = None;
                    self.typing = None;
                    self.set_bubble_visible.set(false);
                    self.set_stage_visible.set(false);
                    wasm_bindgen_futures::spawn_local(async {
                        let _ =
                            ipc::invoke::<_, ()>("set_hit_rect", &SetHitRectArgs { rect: None })
                                .await;
                        let _ = ipc::invoke::<_, ()>("popup_finished", &EmptyArgs {}).await;
                    });
                }
            }
        }
    }

    fn report_hit_rect(&self) {
        let Some(bubble_el) = self.bubble_ref.get_untracked() else {
            return;
        };
        let Some(sprite_el) = self.sprite_host_ref.get_untracked() else {
            return;
        };
        let br = bubble_el.get_bounding_client_rect();
        let sr = sprite_el.get_bounding_client_rect();
        let left = br.left().min(sr.left());
        let top = br.top().min(sr.top());
        let right = br.right().max(sr.right());
        let bottom = br.bottom().max(sr.bottom());
        let rect = Rect {
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
        };
        wasm_bindgen_futures::spawn_local(async move {
            let _ =
                ipc::invoke::<_, ()>("set_hit_rect", &SetHitRectArgs { rect: Some(rect) }).await;
        });
    }

    fn step(&mut self, now_ms: f64) -> bool {
        let dt_ms = match self.last_frame_ms {
            Some(prev) => (now_ms - prev).clamp(0.0, 250.0) as u32,
            None => 0,
        };
        self.last_frame_ms = Some(now_ms);

        // 1. Sprite frame advance
        let mut anim_done = false;
        if let Some(anim) = &mut self.anim {
            if self.reduced_motion {
                self.set_anim_name.set(AnimationName::Idle);
                self.set_sprite_frame.set(0);
                if !anim.done_reported {
                    anim.done_reported = true;
                    anim_done = true;
                }
            } else {
                let at = anim.clock.at(now_ms - anim.start_ms);
                self.set_sprite_frame.set(at.frame);
                if at.finished && !anim.done_reported {
                    anim.done_reported = true;
                    anim_done = true;
                } else if anim.clock.looped && at.cycles > anim.last_cycles {
                    anim.last_cycles = at.cycles;
                    anim_done = true;
                }
            }
        }
        if anim_done {
            if let Some(tl) = &mut self.timeline {
                let eff = tl.on_animation_done();
                self.run_effects(eff, now_ms);
            }
        }

        // 2. Typewriter advance
        let mut typing_done = false;
        if let Some(typing) = &self.typing {
            let (shown, done) = typed(&typing.full_text, now_ms - typing.start_ms);
            self.set_bubble_text.set(shown.to_string());
            if done {
                typing_done = true;
            }
        }
        if typing_done {
            self.typing = None;
            if let Some(tl) = &mut self.timeline {
                let eff = tl.on_typing_done();
                self.run_effects(eff, now_ms);
            }
        }

        // 3. Countdown tick
        if let Some(tl) = &mut self.timeline {
            let eff = tl.tick(dt_ms, self.hovered);
            self.run_effects(eff, now_ms);
        }

        self.timeline.as_ref().is_some_and(PopupTimeline::is_active)
    }

    fn on_click(&mut self, now_ms: f64) {
        if let Some(tl) = &mut self.timeline {
            let eff = tl.on_click();
            self.run_effects(eff, now_ms);
        }
    }
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn prefers_reduced_motion() -> bool {
    web_sys::window()
        .and_then(|w| {
            w.match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .map(|m| m.matches())
        .unwrap_or(false)
}

fn ensure_raf(driver: Rc<RefCell<Driver>>) {
    {
        let mut d = driver.borrow_mut();
        if d.raf_running {
            return;
        }
        d.raf_running = true;
    }

    fn schedule(driver: Rc<RefCell<Driver>>) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let drv = driver.clone();
        let cb = Closure::once_into_js(move || {
            let keep_going = drv.borrow_mut().step(now_ms());
            if keep_going {
                schedule(drv);
            } else {
                drv.borrow_mut().raf_running = false;
            }
        });
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }

    schedule(driver);
}

#[component]
pub fn PopupApp() -> impl IntoView {
    let (stage_visible, set_stage_visible) = signal(false);
    let (pack, set_pack) = signal(None::<PackView>);
    let (anim_name, set_anim_name) = signal(AnimationName::Idle);
    let (sprite_frame, set_sprite_frame) = signal(0u32);
    let (bubble_visible, set_bubble_visible) = signal(false);
    let (bubble_text, set_bubble_text) = signal(String::new());
    let (bubble_author, set_bubble_author) = signal(None::<String>);
    let bubble_ref = NodeRef::<leptos::html::Div>::new();
    let sprite_host_ref = NodeRef::<leptos::html::Div>::new();

    let driver = Rc::new(RefCell::new(Driver {
        timeline: None,
        pack: None,
        anim: None,
        typing: None,
        hovered: false,
        last_frame_ms: None,
        raf_running: false,
        reduced_motion: prefers_reduced_motion(),
        set_stage_visible,
        set_pack,
        set_anim_name,
        set_sprite_frame,
        set_bubble_visible,
        set_bubble_text,
        set_bubble_author,
        bubble_ref,
        sprite_host_ref,
    }));

    {
        let drv = driver.clone();
        ipc::listen::<ShowPayload>("tospu://show", move |payload| {
            drv.borrow_mut().start_show(payload, now_ms());
            ensure_raf(drv.clone());
        });
    }

    let on_enter = {
        let drv = driver.clone();
        move |_| drv.borrow_mut().hovered = true
    };
    let on_leave = {
        let drv = driver.clone();
        move |_| drv.borrow_mut().hovered = false
    };
    let on_bubble_click = {
        let drv = driver.clone();
        move || drv.borrow_mut().on_click(now_ms())
    };
    let on_sprite_click = {
        let drv = driver.clone();
        move |_| drv.borrow_mut().on_click(now_ms())
    };

    view! {
        <main
            id="stage"
            class=move || if stage_visible.get() { "stage visible" } else { "stage" }
            on:mouseenter=on_enter
            on:mouseleave=on_leave
        >
            <Bubble
                visible=bubble_visible
                text=bubble_text
                author=bubble_author
                on_click=on_bubble_click
                node_ref=bubble_ref
            />
            <div node_ref=sprite_host_ref class="sprite-host" on:click=on_sprite_click>
                <Sprite pack=pack animation=anim_name frame=sprite_frame />
            </div>
        </main>
    }
}
