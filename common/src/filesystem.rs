/// Functions for dealing with the filesystem.
use std::path::PathBuf;
use tokio::{
    fs,
    io::{AsyncSeekExt, AsyncWriteExt},
};

#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

/// Ensure that a file with the given path exists and has the given length.
pub async fn allocate(path: &PathBuf, length: u64) -> Result<(), std::io::Error> {
    // First ensure that the directory exists
    let dir = path.parent().unwrap();
    fs::create_dir_all(dir).await?;
    // Then ensure that the file exists and has the right length
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)
        .await?;
    file.set_len(length).await?;
    Ok(())
}

/// Read a chunk of a file.
/// The chunk is specified by its number, as well as the chunk size.
/// The chunk number is zero-indexed.
///
/// Takes as an argument a HashMap of file paths to Mmaps.
/// These are used to avoid having to open the same file multiple times.
pub async fn read_chunk(
    path: &PathBuf,
    chunk_size: u64,
    chunk_number: u64,
    mmaps: &mut std::collections::HashMap<PathBuf, memmap::Mmap>,
) -> Result<Vec<u8>, std::io::Error> {
    debug!("Reading chunk {} of file {:?}", chunk_number, path);

    // If the file is already mmaped, use that.
    if let Some(mmap) = mmaps.get(path) {
        read_chunk_inner(mmap, chunk_size, chunk_number).await
    } else {
        // Otherwise, open the file and read the chunk.
        let file = std::fs::File::open(path)?;
        let mmap = unsafe { memmap::MmapOptions::new().map(&file).unwrap() };
        mmaps.insert(path.clone(), mmap);
        read_chunk_inner(mmaps.get(path).unwrap(), chunk_size, chunk_number).await
    }
}

async fn read_chunk_inner(
    mmap: &memmap::Mmap,
    chunk_size: u64,
    chunk_number: u64,
) -> Result<Vec<u8>, std::io::Error> {
    let offset = chunk_number * chunk_size;
    // If the chunk is out of bounds, return an empty buffer.
    let file_size = mmap.len();
    if offset >= file_size.try_into().unwrap() {
        return Ok(vec![]);
    }

    // If the chunk is close to the end of the file, it may be smaller than the chunk size.
    // In that case, we need to resize the buffer to the correct size.
    let mut buf = vec![0; chunk_size as usize];
    if offset + chunk_size > file_size.try_into().unwrap() {
        buf.resize((file_size - offset as usize) as usize, 0);
    }
    let buf_size = buf.len();
    buf.copy_from_slice(&mmap[offset as usize..offset as usize + buf_size]);
    return Ok(buf);
}

/// Write a chunk of a file.
/// The chunk is specified by its number, as well as the chunk size.
/// The chunk number is zero-indexed.
pub async fn write_chunk(
    path: &PathBuf,
    chunk_size: u64,
    chunk_number: u64,
    data: &[u8],
) -> Result<(), std::io::Error> {
    let mut file = fs::OpenOptions::new().write(true).open(path).await?;
    let offset = chunk_number * chunk_size;
    // If the chunk is out of bounds, return immediately.
    let file_size = file.metadata().await?.len();
    if offset >= file_size {
        return Ok(());
    }
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.write_all(data).await?;
    Ok(())
}
