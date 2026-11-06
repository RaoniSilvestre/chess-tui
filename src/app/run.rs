use crate::{
    app::{App, AppResult},
    event::Event,
    handlers::handler::handle_key_events,
    sound::play_move_sound,
    ui::tui::Tui,
};
use chess_tui_game::GameEvent;

pub fn run(app: &mut App) -> AppResult<()> {
    let mut tui = Tui::new()?;

    while app.running {
        tui.draw(app)?;
        handle_events(app, &mut tui)?;

        handle_game_events(app);
        app.check_game_end_status();
        app.check_lichess_updates();
        app.tick_resume_state();
    }

    Ok(())
}

fn handle_events(app: &mut App, tui: &mut Tui) -> AppResult<()> {
    if app.animations.is_active() {
        match tui.events.try_next() {
            Some(Event::Tick) => app.tick(),
            Some(Event::Key(key_event)) => handle_key_events(key_event, app)?,
            Some(Event::Mouse(mouse_event)) => app.dispatch_mouse(mouse_event),
            Some(Event::Resize(_, _)) | None => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    } else {
        match tui.events.next()? {
            Event::Tick => app.tick(),
            Event::Key(key_event) => handle_key_events(key_event, app)?,
            Event::Mouse(mouse_event) => app.dispatch_mouse(mouse_event),
            Event::Resize(_, _) => {}
        }
    }
    Ok(())
}

fn handle_game_events(app: &mut App) {
    for event in app.game.runtime.take_events() {
        match event {
            GameEvent::MovePlayed => play_move_sound(),
            GameEvent::Error(message) => {
                log::error!("Game mode error: {message}");
                app.show_error(format!("Game error:\n{message}"));
            }
            GameEvent::OpponentJoined => {
                app.close_modal();
            }
        }
    }
}
