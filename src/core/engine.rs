// Logic based on board operations with arrays representaitons.
// Contains functions which calculate position in board, detect last move,
// transform data to stockfish format etc.
use std::io::Write;

use crate::utils::error::{CheatessError, CheatessResult};
pub use crate::utils::printer::{
    AnyBoard, BlackView, DefaultPrinter, PrettyPrinter, Printer, View, WhiteView,
};

pub struct Board<P: Printer, V: View> {
    pub raw: [[char; 8]; 8],
    printer: std::marker::PhantomData<(P, V)>,
}

impl<P: Printer + Sync + Send, V: View + Sync + Send> AnyBoard for Board<P, V> {
    fn print(&self, writer: &mut dyn Write) {
        Board::print(self, writer);
    }
    fn raw(&self) -> &[[char; 8]; 8] {
        &self.raw
    }
}

impl<P: Printer, V: View> Board<P, V> {
    pub fn new(data: [[char; 8]; 8]) -> Self {
        Board {
            raw: data,
            printer: std::marker::PhantomData,
        }
    }

    pub const fn default_white() -> Self {
        Board {
            raw: [
                ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
                ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
                ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
            ],
            printer: std::marker::PhantomData,
        }
    }

    pub const fn default_black() -> Self {
        Board {
            raw: [
                ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
                ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
                ['r', 'n', 'b', 'k', 'q', 'b', 'n', 'r'],
            ],
            printer: std::marker::PhantomData,
        }
    }

    pub fn print<W: Write + ?Sized>(&self, writer: &mut W) {
        let transposed_board: Vec<Vec<_>> = (0..8)
            .map(|row| (0..8).map(|col| self.raw[row][col]).collect())
            .collect();

        write!(writer, "    ").unwrap();
        for j in 0..8 {
            let col = V::col(j);
            let letter = (b'a' + col as u8) as char;
            write!(writer, " {letter}  ").unwrap();
        }
        writeln!(writer).unwrap();
        writeln!(writer, "  +---+---+---+---+---+---+---+---+").unwrap();

        for (i, row) in transposed_board.iter().enumerate() {
            let _row_idx = V::row(i);
            write!(writer, "{} |", 8 - _row_idx).unwrap();
            for col in row.iter() {
                let p = P::print_piece(*col);
                let space = if p.is_empty() { "  " } else { " " };
                write!(writer, " {p}{space}|").unwrap();
            }

            writeln!(writer, " {}", 8 - _row_idx).unwrap();
            writeln!(writer, "  +---+---+---+---+---+---+---+---+").unwrap();
        }
        write!(writer, "    ").unwrap();

        for j in 0..8 {
            let col = V::col(j);
            let letter = (b'a' + col as u8) as char;
            write!(writer, " {letter}  ").unwrap();
        }
        writeln!(writer).unwrap();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Color {
    White,
    Black,
}

pub fn create_board_default<P: Printer + 'static + Send + Sync>(
    player_color: &Color,
) -> Box<dyn AnyBoard + Send + Sync> {
    match player_color {
        Color::White => Box::new(Board::<P, WhiteView>::default_white()),
        Color::Black => Box::new(Board::<P, BlackView>::default_black()),
    }
}

pub fn create_board_from_data<P: Printer + 'static + Send + Sync>(
    data: [[char; 8]; 8],
    player_color: &Color,
) -> Box<dyn AnyBoard + Send + Sync> {
    match player_color {
        Color::White => Box::new(Board::<P, WhiteView>::new(data)),
        Color::Black => Box::new(Board::<P, BlackView>::new(data)),
    }
}

// Insert piece to array board, based on top left position.
pub fn register_piece(
    point: (i32, i32),
    board_size: (i32, i32),
    piece: char,
    board: &mut [[char; 8]; 8],
) -> CheatessResult<()> {
    let tile_width = board_size.0 / 8;
    let tile_height = board_size.1 / 8;

    let row = (point.1 / tile_height).clamp(0, 7) as usize;
    let col = (point.0 / tile_width).clamp(0, 7) as usize;

    board[col][row] = piece;
    Ok(())
}

// Change (x,y) coordiantes to string position representation.
fn coords_to_position(row: usize, col: usize, player_color: &Color) -> String {
    if player_color == &Color::White {
        let file = (b'a' + col as u8) as char;
        let rank = (8 - row).to_string();
        format!("{file}{rank}")
    } else {
        let file = (b'h' - col as u8) as char;
        let rank = (row + 1).to_string();
        format!("{file}{rank}")
    }
}

#[derive(Debug)]
pub struct DiffSquare {
    row: usize,
    col: usize,
    piece_before: char,
    piece_after: char,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq)]
pub enum MoveType {
    Forward,
    Capture,
    Promotion,
    PromotionCapture,
    EnPassant,
    Castle,
    Unknown,
}

pub fn get_coords_moved_pieces(
    before: &[[char; 8]; 8],
    after: &[[char; 8]; 8],
) -> CheatessResult<Vec<DiffSquare>> {
    static MAX_MOVES: usize = 4;
    let mut changes: Vec<DiffSquare> = Vec::with_capacity(MAX_MOVES);

    for row in 0..8 {
        for col in 0..8 {
            if before[row][col] != after[row][col] {
                if changes.len() <= MAX_MOVES {
                    changes.push(DiffSquare {
                        row,
                        col,
                        piece_before: before[row][col],
                        piece_after: after[row][col],
                    })
                } else {
                    return Err(CheatessError::TooManyPositionChanges);
                }
            }
        }
    }
    Ok(changes)
}

pub fn detect_move(
    before: &[[char; 8]; 8],
    after: &[[char; 8]; 8],
    player_color: &Color,
) -> CheatessResult<(String, MoveType)> {
    let from: Option<(usize, usize)>;
    let mut to: Option<(usize, usize)> = None;
    let mut move_type: MoveType = MoveType::Unknown;

    let changed_moves = get_coords_moved_pieces(before, after)?;
    log::trace!("Coordinates of moved pieces: {changed_moves:?}");

    match changed_moves.len() {
        // move forward / capture piece / promotion w/t capture
        2 => {
            if changed_moves //naive check if moved without any capture -> Forward or Promotion
                .iter()
                .all(|d| d.piece_before == ' ' || d.piece_after == ' ')
            {
                log::trace!("Detected move: Forward or Promotion");
                let _from = changed_moves
                    .iter()
                    .find(|&d| d.piece_before != ' ' && d.piece_after == ' ')
                    .expect("There should be at least one record with ' '!!!");
                from = Some((_from.row, _from.col));

                for diff in &changed_moves {
                    if diff.piece_before == ' ' && diff.piece_after != ' ' {
                        to = Some((diff.row, diff.col));
                        if diff.piece_after == _from.piece_before {
                            move_type = MoveType::Forward;
                        } else {
                            move_type = MoveType::Promotion;
                        }
                    }
                }
            } else {
                // e.x. [(1,2, 'p', ' '), (1,1, 'N', 'p') ] or  [(6,6,'P', ' ' ), (7,6, 'b', 'Q')]  -> Capture or PromotionCapture
                log::trace!("Detected move: Capture or PromotionCapture");
                let _from = changed_moves
                    .iter()
                    .find(|&d| d.piece_after == ' ')
                    .expect("There should be at least one record with ' '!!!");
                from = Some((_from.row, _from.col));

                for diff in &changed_moves {
                    if diff.piece_before != ' ' && diff.piece_after != ' ' {
                        to = Some((diff.row, diff.col));
                        if diff.piece_after == _from.piece_before {
                            move_type = MoveType::Capture;
                        } else {
                            move_type = MoveType::PromotionCapture;
                            let x = coords_to_position(_from.row, _from.col, player_color);
                            let y = coords_to_position(diff.row, diff.col, player_color);
                            let new_piece = diff.piece_after.to_lowercase();
                            return Ok((format!("{x}{y}{new_piece}"), move_type));
                        }
                    }
                }
            }
        }
        //en passant
        3 => {
            log::trace!("Detected move: En Passant");
            let _to = changed_moves
                .iter()
                .find(|&d| d.piece_before == ' ' && d.piece_after != ' ')
                .expect("There should be exactly one record with a pawn move: ' ' -> 'P' | 'p'");
            to = Some((_to.row, _to.col));

            let _from = changed_moves
                .iter()
                .find(|&d| d.piece_before == _to.piece_after)
                .expect("There should be exactly one record in which the target pawn moves from the starting position to the target position");
            from = Some((_from.row, _from.col));
            move_type = MoveType::EnPassant;
        }
        //castle
        4 => {
            log::trace!("Detected move: Castle");
            let _to = changed_moves
                .iter()
                .find(|&d| d.piece_before == ' ' && (d.piece_after == 'k' || d.piece_after == 'K'))
                .expect("King's end position not found");
            to = Some((_to.row, _to.col));

            let _from = changed_moves
                .iter()
                .find(|&d| d.piece_before == _to.piece_after)
                .expect("King's start position not found");
            from = Some((_from.row, _from.col));
            move_type = MoveType::Castle;
        }
        x => {
            return Err(CheatessError::InvalidAmountOfMoves(x));
        }
    }

    if let (Some((from_row, from_col)), Some((to_row, to_col))) = (from, to) {
        let x = coords_to_position(from_row, from_col, player_color);
        let y = coords_to_position(to_row, to_col, player_color);
        Ok((format!("{x}{y}"), move_type))
    } else {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::printer::DefaultPrinter;
    use rstest::{fixture, rstest};

    #[fixture]
    fn empty_board() -> [[char; 8]; 8] {
        [[' '; 8]; 8]
    }

    #[rstest]
    #[case(Color::White,[
                ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
                ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
                ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R']
            ])]
    #[case(Color::Black,            [
                ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
                ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
                ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
                ['r', 'n', 'b', 'k', 'q', 'b', 'n', 'r']
            ])]
    fn check_create_board_default(#[case] player_color: Color, #[case] result: [[char; 8]; 8]) {
        let board = create_board_default::<DefaultPrinter>(&player_color);
        assert_eq!(*board.raw(), result);
    }

    #[rstest]
    #[case((0,0),(800,800),(0,0))] //top left piece
    #[case((0,315),(360,360),(0,7))] //top right piece
    #[case((315,0),(360,360),(7,0))] //bottom left piece
    #[case((700,700),(800,800),(7,7))] //bottom right piece
    #[case((180,135),(360,360),(4,3))] //d4 piece

    fn register_piece_correct_insert(
        #[case] point: (i32, i32),
        #[case] board_width_height: (i32, i32),
        #[case] result_row_col: (usize, usize),
        mut empty_board: [[char; 8]; 8],
    ) {
        assert!(register_piece(point, board_width_height, 'X', &mut empty_board).is_ok());
        assert_eq!(empty_board[result_row_col.0][result_row_col.1], 'X');
    }

    #[rstest]
    #[case(0, 0,"a8".to_string(), Color::White)]
    #[case(0, 7,"h8".to_string(), Color::White)]
    #[case(7, 0,"a1".to_string(), Color::White)]
    #[case(7, 7,"h1".to_string(), Color::White)]
    #[case(4, 4,"e4".to_string(), Color::White)]
    #[case(3, 6,"g5".to_string(), Color::White)]
    #[case(0, 0,"h1".to_string(), Color::Black)]
    #[case(0, 7,"a1".to_string(), Color::Black)]
    #[case(7, 0,"h8".to_string(), Color::Black)]
    #[case(7, 7,"a8".to_string(), Color::Black)]
    #[case(4, 4,"d5".to_string(), Color::Black)]
    #[case(3, 6,"b4".to_string(), Color::Black)]
    fn correct_change_coords_to_position(
        #[case] row: usize,
        #[case] col: usize,
        #[case] pos: String,
        #[case] player_color: Color,
    ) {
        let result = coords_to_position(row, col, &player_color);

        assert_eq!(result, pos);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', ' ', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
    ],"e2e4".to_string(),Color::White, Board::<DefaultPrinter, WhiteView>::default_white())]
    #[case([
        ['r', ' ', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', 'n', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
    ],"b8c6".to_string(),Color::White, Board::<DefaultPrinter, WhiteView>::default_white())]
    #[case([
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', 'n', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', ' ', 'b', 'k', 'q', 'b', 'n', 'r'],
    ],"g8f6".to_string(),Color::Black,Board::<DefaultPrinter,BlackView>::default_black())]
    fn detect_move_forward(
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
        #[case] init_board: Board<DefaultPrinter, impl View>,
    ) {
        let result = detect_move(&init_board.raw, &after_move, &player_color);

        assert!(result.is_ok());

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::Forward);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', ' ', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', ' ', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', 'q', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', 'Q', 'K', 'B', 'N', 'R'],
    ],[
        ['r', 'n', 'b', ' ', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', ' ', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', 'Q', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', ' ', 'K', 'B', 'N', 'R'],
    ],"d1d4".to_string(), Color::White)]
    #[case([
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'P', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'q', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],[
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'P', 'q', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],"g4e2".to_string(),Color::Black)]
    fn detect_move_capture_piece(
        #[case] before_move: [[char; 8]; 8],
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
    ) {
        let result = detect_move(&before_move, &after_move, &player_color);

        assert!(result.is_ok());

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::Capture);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', ' ', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', 'P', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', 'Q', 'K', 'B', 'N', 'R'],
    ],[
        ['r', 'n', 'b', 'Q', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', ' ', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', 'Q', 'K', 'B', 'N', 'R'],
    ],"d7d8".to_string(), Color::White)]
    #[case([
        ['R', 'N', ' ', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],[
        ['R', 'N', 'r', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', ' ', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],"f2f1".to_string(),Color::Black)]
    fn detect_move_simple_promotion(
        #[case] before_move: [[char; 8]; 8],
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
    ) {
        let result = detect_move(&before_move, &after_move, &player_color);

        assert!(result.is_ok());

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::Promotion);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', ' ', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', 'P', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', 'Q', 'K', 'B', 'N', 'R'],
    ],[
        ['r', 'n', 'Q', ' ', 'k', 'b', 'n', 'r'],
        ['p', 'p', ' ', ' ', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', 'P', ' ', ' ', ' ', ' ', 'N', ' '],
        [' ', ' ', 'p', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', ' ', 'P', ' ', ' ', 'P', ' ', 'P'],
        ['R', 'N', ' ', 'Q', 'K', 'B', 'N', 'R'],
    ],"d7c8q".to_string(), Color::White)]
    #[case([
        ['R', 'N', ' ', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],[
        ['R', 'Q', ' ', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', ' ', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        ['p', ' ', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'q', 'k', ' ', ' ']
    ],"f2g1q".to_string(),Color::Black)]
    fn detect_move_promotion_with_capture(
        #[case] before_move: [[char; 8]; 8],
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
    ) {
        let result = detect_move(&before_move, &after_move, &player_color);

        assert!(result.is_ok());

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::PromotionCapture);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', ' ', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', 'P', 'p', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', ' ', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
    ],[
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', ' ', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', 'P', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', ' ', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', 'B', 'N', 'R'],
    ],"d5e6".to_string(), Color::White)]
    #[case([
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', 'P', 'p', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', ' ', 'p'],
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r']
    ],[
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', ' ', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', 'p', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', ' ', 'p'],
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r']
    ],"b4c3".to_string(),Color::Black)]
    fn detect_move_en_passant(
        #[case] before_move: [[char; 8]; 8],
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
    ) {
        let result = detect_move(&before_move, &after_move, &player_color);

        assert!(result.is_ok());

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::EnPassant);
    }

    #[rstest]
    #[case([
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', 'K', ' ', ' ', 'R'],
    ],[
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        ['R', 'N', 'B', 'Q', ' ', 'R', 'K', ' '],
    ],"e1g1".to_string(), Color::White)]
    #[case([
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        ['R', ' ', ' ', ' ', 'K', 'B', 'N', 'R'],
    ],[
        ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['P', 'P', 'P', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', 'K', 'R', ' ', 'B', 'N', 'R'],
    ],"e1c1".to_string(),Color::White)]
    #[case([
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', ' ', ' ', 'k', 'q', 'b', 'n', 'r']
    ],[
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        [' ', 'k', 'r', ' ', 'q', 'b', 'n', 'r']
    ],"e8g8".to_string(), Color::Black)]
    #[case([
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', 'k', ' ', ' ', ' ', 'r']
    ],[
        ['R', 'N', 'B', 'K', 'Q', 'B', 'N', 'R'],
        ['P', 'P', 'p', 'P', 'P', 'P', 'P', 'P'],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' ', ' '],
        ['p', 'p', 'p', 'p', 'p', 'p', 'p', 'p'],
        ['r', 'n', 'b', ' ', 'r', 'k', ' ', ' ']
    ],"e8c8".to_string(),Color::Black)]
    fn detect_move_castle(
        #[case] before_move: [[char; 8]; 8],
        #[case] after_move: [[char; 8]; 8],
        #[case] _move: String,
        #[case] player_color: Color,
    ) {
        let result = detect_move(&before_move, &after_move, &player_color);

        let (result_move, move_type) = result.unwrap();
        assert_eq!(result_move, _move);
        assert_eq!(move_type, MoveType::Castle);
    }

    #[rstest]
    fn show_board_with_pieces() {
        let mut buf = Vec::new();
        let board = Board::<PrettyPrinter, BlackView>::default_black();
        board.print(&mut buf);

        let output = String::from_utf8(buf).unwrap();

        let predicted = "     h   g   f   e   d   c   b   a  
  +---+---+---+---+---+---+---+---+
1 | ♖ | ♘ | ♗ | ♔ | ♕ | ♗ | ♘ | ♖ | 1
  +---+---+---+---+---+---+---+---+
2 | ♙ | ♙ | ♙ | ♙ | ♙ | ♙ | ♙ | ♙ | 2
  +---+---+---+---+---+---+---+---+
3 |   |   |   |   |   |   |   |   | 3
  +---+---+---+---+---+---+---+---+
4 |   |   |   |   |   |   |   |   | 4
  +---+---+---+---+---+---+---+---+
5 |   |   |   |   |   |   |   |   | 5
  +---+---+---+---+---+---+---+---+
6 |   |   |   |   |   |   |   |   | 6
  +---+---+---+---+---+---+---+---+
7 | ♟ | ♟ | ♟ | ♟ | ♟ | ♟ | ♟ | ♟ | 7
  +---+---+---+---+---+---+---+---+
8 | ♜ | ♞ | ♝ | ♚ | ♛ | ♝ | ♞ | ♜ | 8
  +---+---+---+---+---+---+---+---+
     h   g   f   e   d   c   b   a  \n"
            .to_string();
        assert_eq!(output, predicted);
    }

    #[rstest]
    fn show_board_with_letters() {
        let mut buf = Vec::new();
        let board = Board::<DefaultPrinter, WhiteView>::default_white();
        board.print(&mut buf);

        let output = String::from_utf8(buf).unwrap();
        let predicted = "     a   b   c   d   e   f   g   h  
  +---+---+---+---+---+---+---+---+
8 | r | n | b | q | k | b | n | r | 8
  +---+---+---+---+---+---+---+---+
7 | p | p | p | p | p | p | p | p | 7
  +---+---+---+---+---+---+---+---+
6 |   |   |   |   |   |   |   |   | 6
  +---+---+---+---+---+---+---+---+
5 |   |   |   |   |   |   |   |   | 5
  +---+---+---+---+---+---+---+---+
4 |   |   |   |   |   |   |   |   | 4
  +---+---+---+---+---+---+---+---+
3 |   |   |   |   |   |   |   |   | 3
  +---+---+---+---+---+---+---+---+
2 | P | P | P | P | P | P | P | P | 2
  +---+---+---+---+---+---+---+---+
1 | R | N | B | Q | K | B | N | R | 1
  +---+---+---+---+---+---+---+---+
     a   b   c   d   e   f   g   h  \n";

        assert_eq!(output, predicted);
    }
}
