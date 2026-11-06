//! Global overlays and page-transition animations.

use crate::runtime::App;
use ratatui::Frame;
use tachyonfx::{EffectRenderer, Interpolation, Motion, fx};

/// Renders the active modal overlay.
pub fn render_popups(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    app.render_modal(frame, main_area);
}

/// Renders page-transition animations.
pub fn render_animations(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    if !app.animations_enabled {
        return;
    }

    let elapsed = app.animations.tick();
    let current_page = app.current_page_type_id();

    if !app.animations.startup_done {
        app.animations.startup_done = true;
        app.animations.last_page = current_page;
        let effect = fx::sweep_in(
            Motion::UpToDown,
            15,
            5,
            ratatui::style::Color::Black,
            (750u32, Interpolation::CubicOut),
        );
        app.animations.push(effect, main_area);
    } else if app.animations.last_page != current_page {
        let effect = fx::coalesce((300u32, Interpolation::QuadOut));
        app.animations.push(effect, main_area);
        app.animations.last_page = current_page;
    }

    for (effect, area) in &mut app.animations.effects {
        frame.render_effect(effect, *area, elapsed);
    }
    app.animations.effects.retain(|(e, _)| !e.done());
}
