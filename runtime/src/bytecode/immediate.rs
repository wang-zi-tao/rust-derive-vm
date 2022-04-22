#[derive(Clone, Copy, Debug)]
pub enum ImmediateKind {
    Void,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}
impl ImmediateKind {
    pub fn size(&self) -> u16 {
        match self {
            ImmediateKind::Void => 0,
            ImmediateKind::U8 | ImmediateKind::I8 => 1,
            ImmediateKind::U16 | ImmediateKind::I16 => 2,
            ImmediateKind::U32 | ImmediateKind::I32 => 4,
            ImmediateKind::U64 | ImmediateKind::I64 => 8,
        }
    }

    pub fn is_signed(&self) -> bool {
        match self {
            ImmediateKind::Void
            | ImmediateKind::U8
            | ImmediateKind::U16
            | ImmediateKind::U32
            | ImmediateKind::U64 => false,
            ImmediateKind::I8 | ImmediateKind::I16 | ImmediateKind::I32 | ImmediateKind::I64 => {
                true
            }
        }
    }
}
