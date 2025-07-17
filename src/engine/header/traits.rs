/// Trait for headers that records used blocks.
pub trait UsedBlocksTrait {
    /// Calculates the number of used blocks.
    fn calc_used_blocks(&self) -> usize;

    /// Returns the number of used blocks and calculate them if not updated.
    fn get_used_blocks(&mut self) -> usize;

    /// Returns the number of saved blocks.
    fn get_saved_blocks(&self) -> usize;
}

/// Trait for headers that records used blocks.
pub trait IsTypeTrait {
    // Tells if the header is a regular file.
    fn is_regular_file(&self) -> bool;
    // Tells if the header is a hard link.
    fn is_hard_link(&self) -> bool;
    // Tells if the header is a symbolic link.
    fn is_symbolic_link(&self) -> bool;
    // Tells if the header is a character special file.
    fn is_character_special(&self) -> bool;
    // Tells if the header is a block special file.
    fn is_block_special(&self) -> bool;
    // Tells if the header is a directory.
    fn is_directory(&self) -> bool;
    // Tells if the header is a FIFO.
    fn is_fifo(&self) -> bool;
    // Tells if the header is a contiguous file.
    fn is_contiguous_file(&self) -> bool;
}