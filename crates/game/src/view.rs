//! Read-only view of game data used by board renderers.

use crate::{GameKind, GameRuntime};
use model::Board;
use shakmaty::{Chess, Color, Move, Role, Square};

/// Read-only view of the game data needed to render a board.
pub trait BoardView {
    fn board(&self) -> &Board;
    fn turn(&self) -> Color;
    fn flipped(&self) -> bool;
    fn kind(&self) -> GameKind;
    fn shows_last_move_from(&self, color: Color) -> bool;

    fn moves(&self) -> &[Move] {
        self.board().moves()
    }

    fn positions(&self) -> &[Chess] {
        self.board().positions()
    }

    fn current_position(&self) -> &Chess {
        self.board().current_position()
    }

    fn move_to_san(&self, index: usize) -> String {
        self.board().move_to_san(index)
    }

    fn get_piece_color_at_square(&self, square: &Square) -> Option<Color> {
        self.board().get_piece_color_at_square(square)
    }

    fn get_role_at_square(&self, square: &Square) -> Option<Role> {
        self.board().get_role_at_square(square)
    }

    fn get_authorized_positions(&self, color: Color, square: &Square) -> Vec<Square> {
        self.board().get_authorized_positions(color, square)
    }

    fn is_getting_checked(&self, color: Color) -> bool {
        self.board().is_getting_checked(color)
    }

    fn get_king_coordinates(&self, color: Color) -> Option<model::Coord> {
        self.board().get_king_coordinates(color)
    }
}

impl BoardView for GameRuntime {
    fn board(&self) -> &Board {
        self.session().board()
    }

    fn turn(&self) -> Color {
        self.session().turn()
    }

    fn flipped(&self) -> bool {
        self.session().is_flipped()
    }

    fn kind(&self) -> GameKind {
        GameRuntime::kind(self)
    }

    fn shows_last_move_from(&self, color: Color) -> bool {
        self.mode().shows_last_move_from(color)
    }
}
