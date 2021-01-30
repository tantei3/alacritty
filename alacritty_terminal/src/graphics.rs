use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Graphics {
    /// Unique identifier, from AtomicUsize
    pub id: usize,

    /// Pixels (in GL_RGB format)
    pub rgb: Vec<u8>,

    /// Graphics size
    pub height: u16,
    pub width: u16,

    /// Height of the cells when the graphics was inserted
    pub cell_height: u16,

}

