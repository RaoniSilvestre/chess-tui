//! Global overlays and page-transition animations.
//! Tachyonfx-based page-transition animation state.

use crate::App;
use ratatui::Frame;
use tachyonfx::{EffectRenderer, Interpolation, Motion, fx};

use pages::PageKind;
use ratatui::prelude::Rect;
use std::time::Instant;
use tachyonfx::Effect;

pub struct AnimationState {
    pub effects: Vec<(Effect, Rect)>,
    pub last_frame: Instant,
    pub startup_done: bool,
    pub last_page: Option<PageKind>,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            last_frame: Instant::now(),
            startup_done: false,
            last_page: None,
        }
    }
}

impl AnimationState {
    pub fn is_active(&self) -> bool {
        !self.effects.is_empty()
    }

    pub fn push(&mut self, effect: Effect, area: Rect) {
        self.effects.push((effect, area));
    }

    pub fn tick(&mut self) -> tachyonfx::Duration {
        let elapsed = self.last_frame.elapsed();
        self.last_frame = Instant::now();
        elapsed.into()
    }

    pub fn render(&mut self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        if !app.globals.animations_enabled {
            return;
        }

        let elapsed = self.tick();
        let current_page = Some(app.page_kind());

        if !self.startup_done {
            self.startup_done = true;
            self.last_page = current_page;
            let effect = fx::sweep_in(
                Motion::UpToDown,
                15,
                5,
                ratatui::style::Color::Black,
                (750u32, Interpolation::CubicOut),
            );
            self.push(effect, area);
        } else if self.last_page != current_page {
            let effect = fx::coalesce((300u32, Interpolation::QuadOut));
            self.push(effect, area);
            self.last_page = current_page;
        }

        for (effect, area) in &mut self.effects {
            frame.render_effect(effect, *area, elapsed);
        }
        self.effects.retain(|(e, _)| !e.done());
    }
}
