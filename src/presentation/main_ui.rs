//! Global overlays and page-transition animations.

use app::App;
use ratatui::Frame;
use tachyonfx::{EffectRenderer, Interpolation, Motion, fx};

/// Renders the active modal overlay.
pub fn render_popups(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    app.render_modal(frame, main_area);
}

/// Renders page-transition animations.
pub fn render_animations(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    if !app.services.animations_enabled {
        return;
    }

    let elapsed = app.services.animations.tick();
    let current_page = app.current_page_type_id();

    if !app.services.animations.startup_done {
        app.services.animations.startup_done = true;
        app.services.animations.last_page = current_page;
        let effect = fx::sweep_in(
            Motion::UpToDown,
            15,
            5,
            ratatui::style::Color::Black,
            (750u32, Interpolation::CubicOut),
        );
        app.services.animations.push(effect, main_area);
    } else if app.services.animations.last_page != current_page {
        let effect = fx::coalesce((300u32, Interpolation::QuadOut));
        app.services.animations.push(effect, main_area);
        app.services.animations.last_page = current_page;
    }

    for (effect, area) in &mut app.services.animations.effects {
        frame.render_effect(effect, *area, elapsed);
    }
    app.services.animations.effects.retain(|(e, _)| !e.done());
}
