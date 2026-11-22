//! Pure animation timing, grapheme typewriter, and popup timeline state machine.

use unicode_segmentation::UnicodeSegmentation;

use crate::content::{Message, QUIET_TEXT};
use crate::manifest::{AnimationName, AnimationSpec};

pub const TYPE_MS_PER_GRAPHEME: f64 = 22.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameClock {
    pub frames: u32,
    pub fps: f32,
    pub looped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameAt {
    pub frame: u32,
    pub cycles: u32,
    pub finished: bool,
}

impl From<&AnimationSpec> for FrameClock {
    fn from(spec: &AnimationSpec) -> Self {
        Self {
            frames: spec.frames,
            fps: spec.fps,
            looped: spec.r#loop,
        }
    }
}

impl FrameClock {
    pub fn at(&self, elapsed_ms: f64) -> FrameAt {
        let frames = self.frames.max(1) as u64;
        let fps = if self.fps.is_finite() && self.fps > 0.0 {
            self.fps as f64
        } else {
            1.0
        };
        let elapsed = elapsed_ms.max(0.0);
        let total_steps = ((elapsed * fps) / 1000.0).floor() as u64;

        if self.looped {
            FrameAt {
                frame: (total_steps % frames) as u32,
                cycles: (total_steps / frames) as u32,
                finished: false,
            }
        } else if total_steps >= frames {
            FrameAt {
                frame: (frames - 1) as u32,
                cycles: 1,
                finished: true,
            }
        } else {
            FrameAt {
                frame: total_steps as u32,
                cycles: 0,
                finished: false,
            }
        }
    }
}

/// Returns the prefix of `text` visible after `elapsed_ms`, splitting only on
/// extended grapheme cluster boundaries, plus whether the full text is shown.
pub fn typed(text: &str, elapsed_ms: f64) -> (&str, bool) {
    let count = (elapsed_ms.max(0.0) / TYPE_MS_PER_GRAPHEME).floor() as usize;
    if count == 0 {
        return ("", text.is_empty());
    }
    match text.grapheme_indices(true).nth(count) {
        Some((byte_idx, _)) => (&text[..byte_idx], false),
        None => (text, true),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Entering,
    Typing,
    Showing,
    Exiting,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    PlayEnter,
    Say {
        text: String,
        author: Option<String>,
    },
    ReportHitRect,
    PlayMood(AnimationName),
    PlayIdleLoop,
    HideBubble,
    PlayExit,
    Finished,
}

#[derive(Debug, Clone)]
pub struct PopupTimeline {
    message: Message,
    mood: AnimationName,
    phase: Phase,
    quiet_revealed: bool,
    display_ms: u32,
    remaining_ms: u32,
    mood_played: bool,
}

impl PopupTimeline {
    pub fn start(
        message: Message,
        quiet: bool,
        display_seconds: u32,
        mood: AnimationName,
    ) -> (Self, Vec<Effect>) {
        let display_ms = display_seconds.max(1) * 1000;
        let tl = Self {
            message,
            mood,
            phase: Phase::Entering,
            quiet_revealed: !quiet,
            display_ms,
            remaining_ms: display_ms,
            mood_played: false,
        };
        (tl, vec![Effect::PlayEnter])
    }

    pub fn on_animation_done(&mut self) -> Vec<Effect> {
        match self.phase {
            Phase::Entering => {
                self.phase = Phase::Typing;
                vec![self.say_effect()]
            }
            Phase::Showing if !self.mood_played => {
                self.mood_played = true;
                vec![Effect::PlayIdleLoop]
            }
            Phase::Exiting => {
                self.phase = Phase::Done;
                vec![Effect::Finished]
            }
            Phase::Typing | Phase::Showing | Phase::Done => Vec::new(),
        }
    }

    pub fn on_typing_done(&mut self) -> Vec<Effect> {
        if self.phase != Phase::Typing {
            return Vec::new();
        }
        self.phase = Phase::Showing;
        if !self.mood_played {
            vec![Effect::ReportHitRect, Effect::PlayMood(self.mood)]
        } else {
            vec![Effect::ReportHitRect]
        }
    }

    pub fn tick(&mut self, dt_ms: u32, hovered: bool) -> Vec<Effect> {
        if self.phase != Phase::Showing || hovered {
            return Vec::new();
        }
        self.remaining_ms = self.remaining_ms.saturating_sub(dt_ms);
        if self.remaining_ms == 0 {
            self.phase = Phase::Exiting;
            vec![Effect::HideBubble, Effect::PlayExit]
        } else {
            Vec::new()
        }
    }

    pub fn on_click(&mut self) -> Vec<Effect> {
        if self.phase != Phase::Showing {
            return Vec::new();
        }
        if !self.quiet_revealed {
            self.quiet_revealed = true;
            self.remaining_ms = self.display_ms;
            self.phase = Phase::Typing;
            vec![self.say_effect()]
        } else {
            self.phase = Phase::Exiting;
            vec![Effect::HideBubble, Effect::PlayExit]
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn is_active(&self) -> bool {
        self.phase != Phase::Done
    }

    fn say_effect(&self) -> Effect {
        if self.quiet_revealed {
            Effect::Say {
                text: self.message.text.clone(),
                author: self.message.author.clone(),
            }
        } else {
            Effect::Say {
                text: QUIET_TEXT.into(),
                author: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kind::MessageKind;

    fn sample_msg() -> Message {
        Message {
            kind: MessageKind::Goal,
            text: "Big goal: Ship".into(),
            author: None,
        }
    }

    #[test]
    fn frame_clock_one_shot() {
        let clock = FrameClock {
            frames: 8,
            fps: 12.0,
            looped: false,
        };
        assert_eq!(clock.at(0.0).frame, 0);
        assert_eq!(clock.at(100.0).frame, 1);
        let end = clock.at(700.0);
        assert_eq!(end.frame, 7);
        assert!(end.finished);
    }

    #[test]
    fn frame_clock_loop_counts_cycles() {
        let clock = FrameClock {
            frames: 6,
            fps: 8.0,
            looped: true,
        };
        let at = clock.at(750.0);
        assert_eq!(at.frame, 0);
        assert_eq!(at.cycles, 1);
        assert!(!at.finished);
    }

    #[test]
    fn typed_reveals_whole_graphemes() {
        assert_eq!(typed("Ca\u{0309}m", 44.0).0, "Ca\u{0309}");
        assert_eq!(typed("🙏ok", 22.0).0, "🙏");
    }

    #[test]
    fn typed_done_after_all_graphemes() {
        let (s, done) = typed("hi", 22.0);
        assert_eq!(s, "h");
        assert!(!done);
        let (s, done) = typed("hi", 44.0);
        assert_eq!(s, "hi");
        assert!(done);
    }

    #[test]
    fn happy_path_effects() {
        let (mut tl, eff) = PopupTimeline::start(sample_msg(), false, 8, AnimationName::Cheer);
        assert_eq!(eff, vec![Effect::PlayEnter]);
        assert_eq!(tl.phase(), Phase::Entering);

        assert_eq!(
            tl.on_animation_done(),
            vec![Effect::Say {
                text: "Big goal: Ship".into(),
                author: None
            }]
        );
        assert_eq!(
            tl.on_typing_done(),
            vec![
                Effect::ReportHitRect,
                Effect::PlayMood(AnimationName::Cheer)
            ]
        );
        assert_eq!(tl.on_animation_done(), vec![Effect::PlayIdleLoop]);
        assert_eq!(
            tl.tick(8000, false),
            vec![Effect::HideBubble, Effect::PlayExit]
        );
        assert_eq!(tl.on_animation_done(), vec![Effect::Finished]);
        assert!(!tl.is_active());
    }

    #[test]
    fn quiet_shows_keep_going_until_clicked() {
        let (mut tl, _) = PopupTimeline::start(sample_msg(), true, 8, AnimationName::Cheer);
        let say = tl.on_animation_done();
        assert_eq!(
            say,
            vec![Effect::Say {
                text: QUIET_TEXT.into(),
                author: None
            }]
        );
        tl.on_typing_done();
        tl.on_animation_done(); // mood done -> idle
        assert!(tl.tick(5000, false).is_empty());

        let reveal = tl.on_click();
        assert_eq!(
            reveal,
            vec![Effect::Say {
                text: "Big goal: Ship".into(),
                author: None
            }]
        );
        assert_eq!(tl.on_typing_done(), vec![Effect::ReportHitRect]);
        assert!(tl.tick(7900, false).is_empty());
        assert_eq!(
            tl.tick(100, false),
            vec![Effect::HideBubble, Effect::PlayExit]
        );
    }

    #[test]
    fn click_on_revealed_bubble_exits_early() {
        let (mut tl, _) = PopupTimeline::start(sample_msg(), false, 60, AnimationName::Idle);
        tl.on_animation_done();
        tl.on_typing_done();
        assert_eq!(tl.on_click(), vec![Effect::HideBubble, Effect::PlayExit]);
        assert_eq!(tl.phase(), Phase::Exiting);
    }

    #[test]
    fn hover_pauses_countdown() {
        let (mut tl, _) = PopupTimeline::start(sample_msg(), false, 8, AnimationName::Idle);
        tl.on_animation_done();
        tl.on_typing_done();
        assert!(tl.tick(60_000, true).is_empty());
        assert_eq!(tl.phase(), Phase::Showing);
    }

    #[test]
    fn clicks_ignored_while_entering_or_exiting() {
        let (mut tl, _) = PopupTimeline::start(sample_msg(), false, 8, AnimationName::Idle);
        assert!(tl.on_click().is_empty());
        tl.on_animation_done();
        tl.on_typing_done();
        tl.on_click(); // now Exiting
        assert_eq!(tl.phase(), Phase::Exiting);
        assert!(tl.on_click().is_empty());
    }

    #[test]
    fn idle_loop_cycle_completions_are_ignored() {
        let (mut tl, _) = PopupTimeline::start(sample_msg(), false, 8, AnimationName::Cheer);
        tl.on_animation_done(); // enter -> say
        tl.on_typing_done(); // say -> mood
        assert_eq!(tl.on_animation_done(), vec![Effect::PlayIdleLoop]); // mood -> idle
        assert!(tl.on_animation_done().is_empty());
        assert!(tl.on_animation_done().is_empty());
        assert_eq!(tl.phase(), Phase::Showing);
    }
}
