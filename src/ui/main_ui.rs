//! Top-level frame composition.

use ratatui::{
    Frame,
    prelude::Alignment,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::{
    constants::Popups,
    ui::popup::{
        engine_path::render_enter_engine_path_popup,
        error::render_error_popup,
        help::render_help_popup,
        lichess::{
            game_code::render_enter_game_code_popup, puzzle_end::render_puzzle_end_popup,
            token::render_enter_lichess_token_popup,
        },
        loading::render_loading_popup,
        move_input::render_move_input_popup,
        multiplayer::{
            enter_ip::render_enter_multiplayer_ip, wait_for_player::render_wait_for_other_player,
        },
        new_game::render_new_game_confirmation_popup,
        pgn::path::render_load_pgn_popup,
        resignation::render_resign_confirmation_popup,
        success::render_success_popup,
    },
};
use chess_tui_ui::components::centered_rect::centered_rect;

use crate::app::App;
use chess_tui_net::get_host_ip;
use tachyonfx::{EffectRenderer, Interpolation, Motion, fx};

/// Renders global popups.
pub fn render_popups(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    match app.ui_state.current_popup {
        Some(Popups::EnterHostIP) => {
            render_enter_multiplayer_ip(frame, &app.game.ui.prompt);
        }
        Some(Popups::MoveInputSelection) => render_move_input_popup(frame, &app.game.ui.prompt),
        Some(Popups::WaitingForOpponentToJoin) => {
            render_wait_for_other_player(frame, get_host_ip());
        }
        Some(Popups::Help) => {
            render_help_popup(frame, app);
        }
        Some(Popups::Error) => {
            if let Some(ref msg) = app.ui_state.popup_message {
                render_error_popup(frame, msg);
            }
        }
        Some(Popups::Success) => {
            if let Some(ref msg) = app.ui_state.popup_message {
                render_success_popup(frame, msg);
            }
        }
        Some(Popups::SeekingLichessGame) => {
            let popup_area = centered_rect(60, 20, main_area);
            let block = Block::default()
                .title("Lichess")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::DarkGray));
            let paragraph = Paragraph::new("Seeking a game on Lichess...\n(Press 'Esc' to cancel)")
                .block(block)
                .alignment(Alignment::Center);
            frame.render_widget(paragraph, popup_area);
        }
        Some(Popups::EnterGameCode) => {
            render_enter_game_code_popup(frame, &app.game.ui.prompt);
        }
        Some(Popups::LoadPgnPath) => {
            render_load_pgn_popup(frame, &app.game.ui.prompt);
        }
        Some(Popups::EnterLichessToken) => {
            render_enter_lichess_token_popup(frame, &app.game.ui.prompt);
        }
        Some(Popups::ResignConfirmation) => {
            render_resign_confirmation_popup(frame, app);
        }
        Some(Popups::NewGameConfirmation) => {
            render_new_game_confirmation_popup(frame, app);
        }
        Some(Popups::PuzzleEndScreen) => {
            let message = if let Some(ref msg) = app.ui_state.popup_message {
                msg.clone()
            } else {
                "Puzzle solved! Well done!".to_string()
            };

            let (elo_change, is_calculating) = if let Some(puzzle_game) =
                &app.lichess_state.puzzle_game
            {
                (
                    puzzle_game.elo_change,
                    puzzle_game.elo_change.is_none() && puzzle_game.elo_change_receiver.is_some(),
                )
            } else {
                (None, false)
            };

            render_puzzle_end_popup(frame, &message, elo_change, is_calculating);
        }
        Some(Popups::Loading) => {
            let message = if let Some(ref msg) = app.ui_state.popup_message {
                msg.clone()
            } else {
                "Loading ...".to_string()
            };
            render_loading_popup(frame, message);
        }
        Some(Popups::EnterEnginePath) => {
            render_enter_engine_path_popup(frame, &app.game.ui.prompt);
        }
        _ => {}
    }
}

/// Renders page-transition animations.
pub fn render_animations(frame: &mut Frame<'_>, app: &mut App, main_area: ratatui::layout::Rect) {
    if !app.animations_enabled {
        return;
    }

    let elapsed = app.animations.tick();
    let current_page = app.ui_state.current_page;

    if !app.animations.startup_done {
        app.animations.startup_done = true;
        app.animations.last_page = Some(current_page);
        let effect = fx::sweep_in(
            Motion::UpToDown,
            15,
            5,
            ratatui::style::Color::Black,
            (750u32, Interpolation::CubicOut),
        );
        app.animations.push(effect, main_area);
    } else if app.animations.last_page != Some(current_page) {
        let effect = fx::coalesce((300u32, Interpolation::QuadOut));
        app.animations.push(effect, main_area);
        app.animations.last_page = Some(current_page);
    }

    for (effect, area) in &mut app.animations.effects {
        frame.render_effect(effect, *area, elapsed);
    }
    app.animations.effects.retain(|(e, _)| !e.done());
}
