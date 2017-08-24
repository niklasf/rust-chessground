use rsvg::Handle;

struct PieceSetSide {
    pawn: Handle,
    knight: Handle,
    bishop: Handle,
    rook: Handle,
    queen: Handle,
    king: Handle,
}

pub struct PieceSet {
    black: PieceSetSide,
    white: PieceSetSide,
}

impl PieceSet {
    pub fn merida() -> PieceSet {
        PieceSet {
            black: PieceSetSide {
                pawn: Handle::new_from_str(include_str!("merida/bP.svg")).expect("merida/bP.svg"),
                knight: Handle::new_from_str(include_str!("merida/bN.svg")).expect("merida/bN.svg"),
                bishop: Handle::new_from_str(include_str!("merida/bB.svg")).expect("merida/bB.svg"),
                rook: Handle::new_from_str(include_str!("merida/bR.svg")).expect("merida/bR.svg"),
                queen: Handle::new_from_str(include_str!("merida/bQ.svg")).expect("merida/bQ.svg"),
                king: Handle::new_from_str(include_str!("merida/bK.svg")).expect("merida/bK.svg"),
            },
            white: PieceSetSide {
                pawn: Handle::new_from_str(include_str!("merida/wP.svg")).expect("merida/wP.svg"),
                knight: Handle::new_from_str(include_str!("merida/wN.svg")).expect("merida/wN.svg"),
                bishop: Handle::new_from_str(include_str!("merida/wB.svg")).expect("merida/wB.svg"),
                rook: Handle::new_from_str(include_str!("merida/wR.svg")).expect("merida/wR.svg"),
                queen: Handle::new_from_str(include_str!("merida/wQ.svg")).expect("merida/wQ.svg"),
                king: Handle::new_from_str(include_str!("merida/wK.svg")).expect("merida/wK.svg"),
            }
        }
    }
}
