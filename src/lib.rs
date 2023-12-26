//! A library providing a safe wrapper around the [`mincore`](https://www.man7.org/linux/man-pages/man2/mincore.2.html) system call. This library also re-exports `rustix::param::page_size` for convenience in interpreting the returned result.

#![doc(html_root_url = "https://docs.rs/mincore-rs/0.1.0")]

use rustix::fs::{fstat, FileType};
use rustix::mm::{mmap, ProtFlags, MapFlags, munmap};
use rustix::io::{Result as RustixResult, Errno};

pub use rustix::param::page_size;

use libc::mincore;

use std::os::fd::AsFd;
use std::io::Error;

/// A function that takes a file descriptor and returns a vector indicating
/// which pages are in memory.
///
/// Note that this function does not follow symlinks, and that it is the
/// caller's responsibility to ensure that `fd` refers to a regular file.
/// (Failing to check this will result in a return value of EACCES).
pub fn mincore_wrapper<Fd: AsFd>(fd: &Fd) -> RustixResult<Vec<bool>> {
    let file_stat = fstat(fd)?;
    // Micro-optimization: check if regular file first before calling mmap
    // If it is not a regular file, return the same errno that mmap would
    if FileType::from_raw_mode(file_stat.st_mode) != FileType::RegularFile {
        return Err(Errno::ACCESS);
    }
    let file_size = usize::try_from(file_stat.st_size).unwrap();
    let page_size = page_size();
    // Size is from mincore man page
    let vec_len = (file_size+page_size-1)/page_size;
    let mut vec_out: Vec<u8> = Vec::with_capacity(vec_len);

    unsafe {
        // SAFETY: see argument comments
        let file_mmap = mmap(
            std::ptr::null_mut(), // pointer is location hint which can be NULL (no location hint)
            file_size, // memory map should match the length of the file and returning an error if this is 0 is fine
            ProtFlags::empty(), // we mmap to determine residency info, not to access the contents (and possibly perturb the state)
            MapFlags::SHARED, // we should see updates to this mmap
            fd, // is valid file descriptor that we received as argument
             0 // start from the beginning of the file
        )?;
        // SAFETY: mincore takes a pointer to a virtual memory region and writes
        // RAM residency information to the memory region at vec_out, with the
        // length computed above using the expression from the mincore man page
        // We have allocated the underlying buffer by using with_capacity
        if mincore(file_mmap, file_size, vec_out.as_mut_ptr()) != 0 {
            // Returncode of either 0 (success) or -1 (failure, see errno)
            // We don't do any other calls in between mincore and last_os_error so errno is untouched
            // errno is thread-unique so there are no race conditions
            return Err(Errno::from_io_error(&Error::last_os_error()).unwrap());
        }
        // SAFETY: this is the unmodified pointer we got from mmap earlier
        munmap(file_mmap, file_size)?;
        // SAFETY: we just filled up the vector with valid values
        vec_out.set_len(vec_len);
    }
    Ok(vec_out.into_iter().map(|x| x!=0).collect())
}
