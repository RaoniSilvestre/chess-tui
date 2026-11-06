//! Board representation and move generation.

use super::coord::Coord;
use shakmaty::{Chess, Color, Move, Piece, Position, Rank, Role, Square, san::San};
use std::fmt;

/// ## Visual representation
///
/// ### How it's stored:
///
/// ```text
/// . a b c d e f g h .
/// 8 ♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖ 8
/// 7 ♙ ♙ ♙ ♙ ♙ ♙ ♙ ♙ 7
/// 6 . . . . . . . . 6
/// 5 . . . . . . . . 5
/// 4 . . . . . . . . 4
/// 3 . . . . . . . . 3
/// 2 ♟ ♟ ♟ ♟ ♟ ♟ ♟ ♟ 2
/// 1 ♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜ 1
/// . a b c d e f g h .
/// ```
///
/// Only the pure gameboard, no additional information.
#[derive(Debug, Clone)]
pub struct GameBoard {
    // Historic of the past Moves of the board
    moves: Vec<Move>,
    /// Historic of the past gameboards states.
    /// The last position is the current position.
    positions: Vec<Chess>,
    /// The number of consecutive non pawn or capture moves
    halfmove_clock: i32,
    /// Stores the captured pieces
    taken_pieces: Vec<Piece>,
    /// Track if the board is currently flipped (for coordinate conversion)
    is_flipped: bool,
    /// Current position index in history when navigating. None means viewing the latest position.
    history_position: Option<usize>,
    /// Original flip state before navigating history (used to restore when returning to latest)
    flip_before_history: Option<bool>,
}

/// Errors returned when installing a board history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryError {
    /// A board history must contain at least the starting position.
    EmptyPositions,
    /// `positions` must contain exactly one more entry than `moves`.
    LengthMismatch { positions: usize, moves: usize },
    /// The halfmove clock cannot be negative.
    NegativeHalfmoveClock,
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HistoryError::EmptyPositions => write!(f, "board history must contain a position"),
            HistoryError::LengthMismatch { positions, moves } => write!(
                f,
                "board history has {positions} positions but {moves} moves; expected positions == moves + 1"
            ),
            HistoryError::NegativeHalfmoveClock => {
                write!(f, "halfmove clock cannot be negative")
            }
        }
    }
}

impl std::error::Error for HistoryError {}

impl Default for GameBoard {
    fn default() -> Self {
        Self {
            moves: Vec::new(),
            positions: vec![Chess::default()],
            halfmove_clock: 0,
            taken_pieces: Vec::new(),
            is_flipped: false,
            history_position: None,
            flip_before_history: None,
        }
    }
}

impl GameBoard {
    /// Convert a move to Standard Algebraic Notation (e.g., "e4", "Nf3", "O-O", "Qxd5+")
    pub fn move_to_san(&self, move_index: usize) -> String {
        if move_index >= self.moves.len() || move_index >= self.positions.len() {
            return String::new();
        }

        // Get the position before this move was made
        let position = &self.positions[move_index];
        let chess_move = &self.moves[move_index];

        // Convert to SAN using shakmaty
        let san = San::from_move(position, chess_move);
        san.to_string()
    }

    /// Access the recorded moves.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// Access the recorded positions. The last entry is the latest position.
    pub fn positions(&self) -> &[Chess] {
        &self.positions
    }

    /// Access the captured pieces, in capture order.
    pub fn taken_pieces(&self) -> &[Piece] {
        &self.taken_pieces
    }

    /// Number of consecutive non-pawn, non-capture moves.
    pub fn halfmove_clock(&self) -> i32 {
        self.halfmove_clock
    }

    pub fn is_flipped(&self) -> bool {
        self.is_flipped
    }

    /// The history index currently being viewed, or `None` when viewing the latest position.
    pub fn history_position(&self) -> Option<usize> {
        self.history_position
    }

    /// Record a move for the fifty-move rule.
    pub fn record_move_for_draw_rule(&mut self, role_from: Role, role_to: Option<Role>) {
        match (role_from, role_to) {
            (Role::Pawn, _) | (_, Some(_)) => {
                self.halfmove_clock = 0;
            }
            _ => {
                self.halfmove_clock += 1;
            }
        }
    }

    pub fn reset(&mut self) {
        self.moves.clear();
        self.positions.clear();
        self.positions.push(Chess::default());
        self.halfmove_clock = 0;
        self.is_flipped = false;
        self.history_position = None;
        self.flip_before_history = None;
    }

    /// Replace the full board history after validating its shape.
    pub fn load_history(
        &mut self,
        positions: Vec<Chess>,
        moves: Vec<Move>,
        taken_pieces: Vec<Piece>,
        halfmove_clock: i32,
    ) -> Result<(), HistoryError> {
        if positions.is_empty() {
            return Err(HistoryError::EmptyPositions);
        }
        if positions.len() != moves.len() + 1 {
            return Err(HistoryError::LengthMismatch {
                positions: positions.len(),
                moves: moves.len(),
            });
        }
        if halfmove_clock < 0 {
            return Err(HistoryError::NegativeHalfmoveClock);
        }

        self.positions = positions;
        self.moves = moves;
        self.taken_pieces = taken_pieces;
        self.halfmove_clock = halfmove_clock;
        self.history_position = None;
        self.flip_before_history = None;
        Ok(())
    }

    /// Ensure the board has at least the starting position.
    pub fn ensure_initial_position(&mut self) {
        if self.positions.is_empty() {
            self.positions.push(Chess::default());
        }
    }

    /// Record a move recovered from an already-applied remote position.
    ///
    /// This is a legacy recovery path for Lichess historical moves. Prefer
    /// [`Self::play_and_record`] for normal moves.
    #[doc(hidden)]
    pub fn record_recovery_move(&mut self, mv: Move) {
        self.moves.push(mv);
        self.history_position = None;
        self.flip_before_history = None;
    }

    /// The position currently being viewed.
    pub fn current_position(&self) -> &Chess {
        let index = self
            .history_position
            .unwrap_or_else(|| self.positions.len() - 1);
        &self.positions[index]
    }

    /// Check if the game is a draw by repetition
    pub fn is_draw_by_repetition(&self) -> bool {
        let mut position_counts = std::collections::HashMap::new();
        for board in self.positions.iter() {
            let count = position_counts.entry(board).or_insert(0);
            *count += 1;

            if *count >= 3 {
                return true;
            }
        }
        false
    }

    /// Check if the game is a draw
    pub fn is_draw(&self) -> bool {
        let chess = self.current_position();
        chess.is_stalemate()
            || self.halfmove_clock == 50
            || self.is_draw_by_repetition()
            || chess.is_insufficient_material()
    }

    /// Check if the last move was a promotion
    // We check manually if the last move was a pawn to one of the promotion squares
    // AND the promotion hasn't been handled yet (no promotion piece set)
    pub fn is_latest_move_promotion(&self) -> bool {
        self.moves
            .last()
            .map(|last_move| {
                // Check if it's a pawn move to promotion square
                let is_pawn_to_promotion_square = last_move.role() == Role::Pawn
                    && (last_move.to().rank() == Rank::First
                        || last_move.to().rank() == Rank::Eighth);

                // Only return true if it's a promotion move AND promotion hasn't been handled yet
                // If promotion() returns Some, the promotion was already handled
                is_pawn_to_promotion_square && last_move.promotion().is_none()
            })
            .unwrap_or(false)
    }

    /// Get piece type at a coordinate (handles flipped board)
    pub fn get_role_at_square(&self, square: &Square) -> Option<Role> {
        self.current_position()
            .board()
            .piece_at(*square)
            .map(|p| p.role)
    }

    /// Check if a square as a piece on it
    pub fn is_square_occupied(&self, square: &Square) -> bool {
        self.current_position().board().piece_at(*square).is_some()
    }

    /// Get the color of a piece on a given square
    pub fn get_piece_color_at_square(&self, square: &Square) -> Option<Color> {
        let piece = self.current_position().board().piece_at(*square);
        piece.map(|p| p.color)
    }

    /// Get authorized positions for a piece at the given coordinate
    pub fn get_authorized_positions(&self, player_turn: Color, square: &Square) -> Vec<Square> {
        // Check if there's a piece at this position and it's the right color
        if let Some(piece) = self.current_position().board().piece_at(*square) {
            if piece.color != player_turn {
                return vec![];
            }
        } else {
            return vec![];
        }

        // Get all legal moves
        self.current_position()
            .clone()
            .legal_moves()
            .iter()
            .filter(|m| m.from() == Some(*square))
            .map(|m| m.to())
            .collect()
    }

    /// Check if the king is in check
    pub fn is_getting_checked(&self, player_turn: Color) -> bool {
        let chess = self.current_position();
        chess.is_check() && chess.turn() == player_turn
    }

    /// Get the king's coordinates, or `None` if the board is in an invalid state.
    pub fn get_king_coordinates(&self, color: Color) -> Option<Coord> {
        let king_square = self.current_position().board().king_of(color)?;
        let mut coord = Coord::from(king_square);
        if self.is_flipped {
            coord = coord.reverse();
        }
        Some(coord)
    }

    /// Check if game is checkmate
    pub fn is_checkmate(&self) -> bool {
        self.current_position().clone().is_checkmate()
    }

    /// Flip the board for alternating perspectives.
    pub fn flip(&mut self) {
        self.is_flipped = !self.is_flipped;
    }

    /// Set the board orientation explicitly.
    pub fn set_flipped(&mut self, flipped: bool) {
        self.is_flipped = flipped;
    }

    /// Get FEN position for UCI engine.
    pub fn fen(&self) -> String {
        // Use FEN display from shakmaty
        use shakmaty::fen::Fen;
        let fen = Fen::from_position(
            self.current_position().clone(),
            shakmaty::EnPassantMode::Legal,
        );
        fen.to_string()
    }

    /// Get black taken pieces
    pub fn black_taken_pieces(&self) -> Vec<Role> {
        self.taken_pieces
            .iter()
            .filter(|p| p.color == Color::Black)
            .map(|p| p.role)
            .collect()
    }

    /// Get white taken pieces
    pub fn white_taken_pieces(&self) -> Vec<Role> {
        self.taken_pieces
            .iter()
            .filter(|p| p.color == Color::White)
            .map(|p| p.role)
            .collect()
    }

    /// Reconstruct game history from a string of space-separated UCI moves
    /// Optionally verifies against an expected final FEN
    #[allow(clippy::expect_used)]
    pub fn reconstruct_history(&mut self, moves_str: &str, expected_fen: Option<&str>) {
        if moves_str.trim().is_empty() {
            return;
        }

        log::info!(
            "Reconstructing history from {} moves",
            moves_str.split_whitespace().count()
        );

        // Reset board to initial state
        self.reset();

        // Start from the initial position
        let mut current_position = self.positions[0].clone();
        let mut moves_applied = 0;
        let mut moves_failed = 0;

        // Parse moves string (space-separated UCI moves)
        let move_strings: Vec<&str> = moves_str.split_whitespace().collect();

        for (i, move_uci) in move_strings.iter().enumerate() {
            // Parse UCI move using shakmaty
            match move_uci.parse::<shakmaty::uci::UciMove>() {
                Ok(uci) => {
                    match uci.to_move(&current_position) {
                        Ok(chess_move) => {
                            // Track captures
                            match chess_move {
                                Move::Normal { .. } => {
                                    if let Some(captured_piece) =
                                        current_position.board().piece_at(chess_move.to())
                                    {
                                        self.taken_pieces.push(captured_piece);
                                    }
                                }
                                Move::EnPassant { .. } => {
                                    let from_square = chess_move
                                        .from()
                                        .expect("En passant move must have a 'from' square");
                                    let to_square = chess_move.to();
                                    let captured_pawn_square =
                                        Square::from_coords(to_square.file(), from_square.rank());
                                    if let Some(captured_piece) =
                                        current_position.board().piece_at(captured_pawn_square)
                                    {
                                        self.taken_pieces.push(captured_piece);
                                    }
                                }
                                Move::Castle { .. } => {
                                    // No piece is taken when castling.
                                }
                                Move::Put { .. } => {
                                    // Putting a piece on board, not a capture
                                }
                            }

                            // Apply the move
                            match current_position.play(&chess_move) {
                                Ok(new_pos) => {
                                    self.moves.push(chess_move);
                                    self.positions.push(new_pos.clone());
                                    current_position = new_pos;
                                    moves_applied += 1;
                                }
                                Err(e) => {
                                    log::warn!(
                                        "Failed to play move {}: {} - {}. Stopping history reconstruction.",
                                        i + 1,
                                        move_uci,
                                        e
                                    );
                                    moves_failed += 1;
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "Illegal move {}: {} - {}. Stopping history reconstruction.",
                                i + 1,
                                move_uci,
                                e
                            );
                            moves_failed += 1;
                            break;
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to parse UCI move {}: {} - {}. Skipping.",
                        i + 1,
                        move_uci,
                        e
                    );
                    moves_failed += 1;
                }
            }
        }

        if moves_failed > 0 {
            log::warn!(
                "Failed to apply {} out of {} moves. History may be incomplete.",
                moves_failed,
                move_strings.len()
            );
        }

        log::info!(
            "Successfully applied {} moves out of {} total",
            moves_applied,
            move_strings.len()
        );

        // Verify that the final position matches the expected FEN
        if let Some(fen_str) = expected_fen
            && let Some(final_position) = self.positions.last()
        {
            let final_fen = shakmaty::fen::Fen::from_position(
                final_position.clone(),
                shakmaty::EnPassantMode::Legal,
            )
            .to_string();

            // Compare FEN strings (ignoring move counters which might differ)
            let final_fen_parts: Vec<&str> = final_fen.split(' ').collect();
            let expected_fen_parts: Vec<&str> = fen_str.split(' ').collect();

            let positions_match = final_fen_parts.len() >= 4
                && expected_fen_parts.len() >= 4
                && final_fen_parts[0..4] == expected_fen_parts[0..4];

            if !positions_match {
                log::warn!(
                    "Final position from moves doesn't match FEN. Final: {}, Expected: {}",
                    final_fen,
                    fen_str
                );
                log::warn!("Using FEN position and rebuilding history from it");

                // If positions don't match, use the FEN position as the final position
                // Try to parse the expected FEN
                if let Ok(fen) = shakmaty::fen::Fen::from_ascii(fen_str.as_bytes())
                    && let Ok(position) =
                        fen.into_position::<shakmaty::Chess>(shakmaty::CastlingMode::Standard)
                {
                    // Replace the last position in history with the FEN position
                    if let Some(last_pos) = self.positions.last_mut() {
                        *last_pos = position;
                    }
                }
            } else {
                log::info!("Final position matches FEN - history is correct");
            }
        }
    }

    /// Execute a move without recording it in `moves`.
    /// Returns the executed Move if successful, None if illegal.
    #[allow(clippy::expect_used)]
    fn execute_internal(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
    ) -> Option<Move> {
        let chess = self.current_position().clone();

        // Find matching legal move
        let legal_moves = chess.legal_moves();

        // Check if this looks like a castling move (king moving 2 squares horizontally)
        let is_potential_castling = {
            let piece_at_from = chess.board().piece_at(from);
            if let Some(piece) = piece_at_from {
                if piece.role == Role::King && piece.color == chess.turn() {
                    // Check if moving 2 squares horizontally (castling)
                    let from_file = from.file();
                    let to_file = to.file();
                    let from_rank = from.rank();
                    let to_rank = to.rank();
                    from_rank == to_rank && (from_file as i8 - to_file as i8).abs() == 2
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_potential_castling {
            log::debug!("Detected potential castling move: {} -> {}", from, to);
            // Log all castling moves available
            let castling_moves: Vec<_> = legal_moves
                .iter()
                .filter(|m| matches!(m, shakmaty::Move::Castle { .. }))
                .collect();
            if !castling_moves.is_empty() {
                for cm in &castling_moves {
                    log::debug!(
                        "  Available castling: {:?} (from: {:?}, to: {})",
                        cm,
                        cm.from(),
                        cm.to()
                    );
                }
            } else {
                log::warn!(
                    "Castling move {} -> {} requested but no castling moves available",
                    from,
                    to
                );
            }
        }

        let matching_move = legal_moves.iter().find(|m| {
            // Regular move matching (this will also match castling moves correctly)
            m.from() == Some(from) && m.to() == to && {
                match (promotion, m.promotion()) {
                    (Some(promo), Some(move_promo)) => promo == move_promo,
                    (None, None) => true,
                    (None, Some(_)) => true, // Allow promotion moves without specifying
                    (Some(_), None) => false, // Reject if we expect promotion but move has none
                }
            }
        });

        if let Some(shakmaty_move) = matching_move {
            // It's a legal move. Check for capture.
            match *shakmaty_move {
                Move::Normal { .. } => {
                    if let Some(captured_piece) = chess.board().piece_at(shakmaty_move.to()) {
                        self.taken_pieces.push(captured_piece);
                    }
                }
                Move::EnPassant { .. } => {
                    let from_square = shakmaty_move
                        .from()
                        .expect("En passant move must have a 'from' square");
                    let to_square = shakmaty_move.to();
                    let captured_pawn_square =
                        Square::from_coords(to_square.file(), from_square.rank());
                    if let Some(captured_piece) = chess.board().piece_at(captured_pawn_square) {
                        self.taken_pieces.push(captured_piece);
                    }
                }
                Move::Castle { .. } => {
                    // No piece is taken when castling.
                }
                Move::Put { .. } => {
                    // Putting a piece on board, not a capture
                }
            }

            // Execute move
            match chess.play(shakmaty_move) {
                Ok(new_chess) => {
                    // Update history
                    self.positions.push(new_chess);
                    // Reset history navigation when a new move is made
                    self.history_position = None;
                    self.flip_before_history = None;
                    Some(shakmaty_move.clone())
                }
                Err(e) => {
                    log::error!("Failed to play move {} -> {}: {}", from, to, e);
                    None
                }
            }
        } else {
            // Move not found - check if it's a castling move that needs special handling
            if is_potential_castling {
                // Clone chess before using it in the play call
                let chess_clone = chess.clone();
                // Try to find castling move from the same square
                let castling_move = legal_moves
                    .iter()
                    .find(|m| matches!(m, shakmaty::Move::Castle { .. }) && m.from() == Some(from));

                if let Some(castle_move) = castling_move {
                    log::info!(
                        "Found castling move for {} -> {}: {:?} (actual destination: {})",
                        from,
                        to,
                        castle_move,
                        castle_move.to()
                    );
                    // Execute the castling move even though destination doesn't match exactly
                    match chess_clone.play(castle_move) {
                        Ok(new_chess) => {
                            self.positions.push(new_chess);
                            self.history_position = None;
                            self.flip_before_history = None;
                            log::info!("Castling move executed successfully: {:?}", castle_move);
                            return Some(castle_move.clone());
                        }
                        Err(e) => {
                            log::error!("Failed to play castling move: {}", e);
                        }
                    }
                } else {
                    log::warn!(
                        "Castling move {} -> {} requested but no castling available from {}",
                        from,
                        to,
                        from
                    );
                }
            }

            // Log why move wasn't found
            let piece_at_from = chess.board().piece_at(from);
            let piece_at_to = chess.board().piece_at(to);
            log::debug!(
                "Move {} -> {} not found in legal moves. Piece at from: {:?}, piece at to: {:?}, promotion: {:?}, current turn: {:?}",
                from,
                to,
                piece_at_from,
                piece_at_to,
                promotion,
                chess.turn()
            );
            None
        }
    }

    /// Play a move and record the executed move in history.
    pub fn play(&mut self, from: Square, to: Square, promotion: Option<Role>) -> Option<Move> {
        self.play_and_record(from, to, promotion, |executed| executed.clone())
    }

    /// Play a move and record a caller-provided history entry.
    ///
    /// This is used for promotion placeholders and normalized history moves.
    pub fn play_and_record<F>(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
        record: F,
    ) -> Option<Move>
    where
        F: FnOnce(&Move) -> Move,
    {
        let executed = self.execute_internal(from, to, promotion)?;
        self.moves.push(record(&executed));
        self.history_position = None;
        self.flip_before_history = None;
        Some(executed)
    }

    /// Undo the latest move and position.
    pub fn undo_last_move(&mut self) -> Option<Move> {
        let mv = self.moves.pop()?;
        self.positions.pop();
        self.history_position = None;
        self.flip_before_history = None;
        Some(mv)
    }

    /// Get the correct flip state for a given position index in solo mode
    /// In solo mode, the board flips after each move, so:
    /// - index 0 (initial): not flipped
    /// - index 1 (after move 1): flipped
    /// - index 2 (after move 2): not flipped
    /// - etc.
    fn get_flip_state_for_index(&self, index: usize, is_solo_mode: bool) -> bool {
        if !is_solo_mode {
            // In non-solo modes, use the current flip state
            return self.is_flipped;
        }
        // In solo mode, flip state alternates: odd indices are flipped
        (index % 2) == 1
    }

    /// Navigate to the next position in history (forward in time)
    /// Returns true if navigation was successful, false if already at the latest position
    /// is_solo_mode: whether we're in solo mode (affects board flipping)
    pub fn navigate_history_next(&mut self, is_solo_mode: bool) -> bool {
        if self.positions.is_empty() {
            return false;
        }

        let max_index = self.positions.len() - 1;

        match self.history_position {
            None => {
                // Already at the latest position
                false
            }
            Some(index) => {
                if index < max_index {
                    let new_index = index + 1;
                    self.history_position = Some(new_index);
                    // Update flip state based on position index
                    self.is_flipped = self.get_flip_state_for_index(new_index, is_solo_mode);
                    true
                } else {
                    // Reached the latest position, reset to None
                    // Restore the original flip state
                    self.history_position = None;
                    if let Some(original_flip) = self.flip_before_history.take() {
                        self.is_flipped = original_flip;
                    } else {
                        // Fallback: calculate based on latest index
                        self.is_flipped = self.get_flip_state_for_index(max_index, is_solo_mode);
                    }
                    true
                }
            }
        }
    }

    /// Navigate to the previous position in history (backward in time)
    /// Returns true if navigation was successful, false if already at the first position
    /// is_solo_mode: whether we're in solo mode (affects board flipping)
    pub fn navigate_history_previous(&mut self, is_solo_mode: bool) -> bool {
        if self.positions.is_empty() {
            return false;
        }

        match self.history_position {
            None => {
                // Currently at latest position, go to previous
                // Store the current flip state before navigating
                if self.positions.len() > 1 {
                    self.flip_before_history = Some(self.is_flipped);
                    let prev_index = self.positions.len() - 2;
                    self.history_position = Some(prev_index);
                    // Update flip state based on position index
                    self.is_flipped = self.get_flip_state_for_index(prev_index, is_solo_mode);
                    true
                } else {
                    false
                }
            }
            Some(index) => {
                if index > 0 {
                    let new_index = index - 1;
                    self.history_position = Some(new_index);
                    // Update flip state based on position index
                    self.is_flipped = self.get_flip_state_for_index(new_index, is_solo_mode);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Reset history navigation to view the latest position
    /// is_solo_mode: whether we're in solo mode (affects board flipping)
    pub fn reset_history_navigation(&mut self, is_solo_mode: bool) {
        if self.history_position.is_some() {
            self.history_position = None;
            // Restore the original flip state
            if let Some(original_flip) = self.flip_before_history.take() {
                self.is_flipped = original_flip;
            } else {
                // Fallback: calculate based on latest index
                let max_index = self.positions.len().saturating_sub(1);
                self.is_flipped = self.get_flip_state_for_index(max_index, is_solo_mode);
            }
        }
    }

    /// Truncate history at the given index, removing all moves and positions after it
    /// This is used when making a move from a historical position to create a new branch
    pub fn truncate_history_at(&mut self, index: usize) {
        if index >= self.positions.len() {
            return;
        }

        // Truncate position history to index + 1 (keep the position at index)
        // We keep index + 1 because position_history[i] is the position AFTER move i
        let new_len = index + 1;
        if new_len < self.positions.len() {
            self.positions.truncate(new_len);
        }

        // Truncate move history to index (remove moves after this point)
        if index < self.moves.len() {
            self.moves.truncate(index);
        }

        // Reset history navigation since we're now at the latest position
        self.history_position = None;
        self.flip_before_history = None;
    }
}
