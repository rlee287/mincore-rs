use rustix::fs::fstat;
use rustix::mm::{mmap, ProtFlags, MapFlags, munmap};
use rustix::io::{Result as RustixResult, Errno};
use rustix::param::page_size;

use libc::mincore;

use std::os::fd::AsFd;
use std::io::Error;

pub fn mincore_wrapper<Fd: AsFd>(fd: &Fd) -> RustixResult<Vec<bool>> {
    let file_size = usize::try_from(fstat(fd)?.st_size).unwrap();
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