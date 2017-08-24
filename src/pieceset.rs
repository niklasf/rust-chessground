use rsvg::Handle;

struct PieceSetSide {
    king: Handle,
}

pub struct PieceSet {
    black: PieceSetSide,
}

impl PieceSet {
    pub fn cburnett() -> PieceSet {
        PieceSet {
            black: PieceSetSide {
                king: Handle::new_from_str(include_str!("cburnett/bK.svg")).expect("bK.svg"),
            }
        }
    }
}
