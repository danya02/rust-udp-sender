/// Functions for dealing with the filesystem.

use std::path::PathBuf;
use tokio::{fs, io::{AsyncSeekExt, AsyncReadExt, AsyncWriteExt}};

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
pub async fn read_chunk(path: &PathBuf, chunk_size: u64, chunk_number: u64) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .await?;
    let mut buf = vec![0; chunk_size as usize];
    let offset = chunk_number * chunk_size;
    // If the chunk is out of bounds, return an empty buffer.
    let file_size = file.metadata().await?.len();
    if offset >= file_size {
        return Ok(vec![]);
    }

    // If the chunk is close to the end of the file, it may be smaller than the chunk size.
    // In that case, we need to resize the buffer to the correct size.
    if offset + chunk_size > file_size {
        buf.resize((file_size - offset) as usize, 0);
    }
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Write a chunk of a file.
/// The chunk is specified by its number, as well as the chunk size.
/// The chunk number is zero-indexed.
pub async fn write_chunk(path: &PathBuf, chunk_size: u64, chunk_number: u64, data: &[u8]) -> Result<(), std::io::Error> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .await?;
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