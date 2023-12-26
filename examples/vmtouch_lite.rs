use mincore::mincore_wrapper;

use std::env::args;
use std::fs::File;

pub fn main() -> Result<(), String> {
    let args_vec: Vec<_> = args().collect();
    if args_vec.len() != 2 {
        return Err(format!("Usage: {} [filename]", args_vec[0]));
    }
    let open_file = File::open(&args_vec[1]).map_err(|e| format!("Error opening file: {}", e))?;

    let mincore_map = mincore_wrapper(&open_file).map_err(|e| format!("Error finding cached blocks: {}", e))?;
    let page_count = mincore_map.len();
    let mapped_page_count = mincore_map.iter().filter(|x| **x).count();

    println!("Mapped page count {}/{}", mapped_page_count, page_count);
    Ok(())
}