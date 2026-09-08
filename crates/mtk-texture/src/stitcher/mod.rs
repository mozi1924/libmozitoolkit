pub mod region;
pub mod stitcher;

pub use region::StitcherRegion;
pub use stitcher::{
    smallest_encompassing_power_of_two, StitchedAtlas, StitchedChunk, StitchedSlot, Stitcher,
    StitcherHolder,
};
