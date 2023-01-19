use core::time;
use std::{io::{stdout, Write, Stdout}, collections::VecDeque};

use bytesize::ByteSize;
use tokio::{sync::mpsc::{Receiver, Sender}, time::Instant};

use crate::server_state::ServerData;
use crossterm::{cursor::*, style::*, QueueableCommand, Result, terminal::{Clear, ClearType}};

#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Mark this chunk as downloaded
    /// (file_idx, chunk_idx, downloaded_bytes)
    ChunkDownloaded(u64, u64, usize),
    /// Mark this chunk as requested
    ChunkRequested(u64, u64),
    /// Mark this file as done
    FileDone(u64),
}

const MAX_FILE_NAME_LEN: usize = 20;
const AVERAGE_RATE_OVER_SECS: u64 = 5;
const SUPERBLOCK_STEPS: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];

pub struct ProgressIndicator {
    stream: Stdout,
    event_rx: Receiver<ProgressEvent>,
    event_tx: Sender<ProgressEvent>,
    file_lengths: Vec<u64>,
    file_states: Vec<Vec<usize>>,
    file_names: Vec<String>,
    done_files: Vec<bool>,
    downloaded_byte_counts: VecDeque<(Instant, usize)>,
}

impl ProgressIndicator {
    /// Create a new progress indicator
    pub fn new(server_data: &ServerData) -> Self {
        let file_lengths = server_data.files.iter().map(|f| f.1.num_chunks).collect();
        let file_names = server_data.files.iter().map(|f| f.0.path.clone()).collect();
        let file_states = server_data.files.iter().map(
            |f| vec![0; f.1.num_chunks as usize / SUPERBLOCK_STEPS.len() + (if f.1.num_chunks as usize % SUPERBLOCK_STEPS.len() == 0 {0} else {1})]
        ).collect();

        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        Self {
            stream: stdout(),
            event_rx,
            event_tx,
            file_lengths,
            file_names,
            done_files: vec![false; server_data.files.len()],
            downloaded_byte_counts: VecDeque::with_capacity(100),
            file_states,
        }
    }

    /// Get the event sender
    pub fn event_tx(&self) -> Sender<ProgressEvent> {
        self.event_tx.clone()
    }

    /// Run the progress indicator
    pub async fn run(&mut self) -> Result<()> {
        self.init_progress_bars().expect("Failed to initialise progress bars");
        loop {
            let event = self.event_rx.recv().await.unwrap();
            match event {
                ProgressEvent::ChunkDownloaded(file_idx, chunk_idx, chunk_size) => {
                    let superblock_idx = chunk_idx as usize / (SUPERBLOCK_STEPS.len());
                    self.file_states[file_idx as usize][superblock_idx] += 1;
                    self.on_file_line(file_idx as usize, |this| {
                        this.print_file_progress(file_idx as usize)?;
                        Ok(())
                    })?;

                    self.downloaded_byte_counts.push_back((Instant::now(), chunk_size));
                    self.print_first_row()?;

                    self.stream.flush().unwrap();
                },
                ProgressEvent::ChunkRequested(file_idx, chunk_idx) => {
                    // TODO
                    self.print_first_row()?;
                    self.stream.flush().unwrap();
                },
                ProgressEvent::FileDone(file_idx) => {
                    self.stream.queue(MoveToNextLine(file_idx as u16 + 1))?;
                    self.stream.queue(Print(self.file_names[file_idx as usize].clone()))?;
                    self.stream.queue(PrintStyledContent(" Done".green()))?;
                    self.stream.queue(Clear(ClearType::UntilNewLine))?;
                    self.stream.queue(MoveToPreviousLine(file_idx as u16 + 1))?;
                    self.print_first_row()?;
                    self.stream.flush().unwrap();

                    self.done_files[file_idx as usize] = true;
                    if self.done_files.iter().all(|f| *f) {
                        break Ok(());
                    }
                },
            }
        }
    }

    /// Initialise the progress bars
    fn init_progress_bars(&mut self) -> Result<()> {
        // Leave enough blank lines for the progress bars
        for _ in 0..self.file_names.len() {
            self.stream.queue(Print('\n'))?;
        }
        self.stream.queue(MoveToPreviousLine(self.file_names.len() as u16))?;
        for file_idx in 0..self.file_names.len() {
            self.stream.queue(Clear(ClearType::UntilNewLine))?;
            self.print_file_progress(file_idx)?;
            self.stream.queue(MoveToNextLine(1))?;
        }
        self.stream.queue(MoveToPreviousLine(self.file_names.len() as u16 + 1))?;
        self.stream.flush()?;
        Ok(())
    }

    /// Print the first row, containing the download rate
    fn print_first_row(&mut self) -> Result<()> {
        self.stream.queue(MoveToColumn(0))?;
        if self.downloaded_byte_counts.len() == 0 {
            self.stream.queue(Print("...stalled..."))?;
            self.stream.queue(Clear(ClearType::UntilNewLine))?;
            return Ok(());
        }
        while self.downloaded_byte_counts.front().is_some_and(|(when, _)| when.elapsed() > time::Duration::from_secs(AVERAGE_RATE_OVER_SECS)) {
            self.downloaded_byte_counts.pop_front();
        }
        let latest_downloaded_bytes = self.downloaded_byte_counts.iter().map(|(_, bytes)| bytes).sum::<usize>();
        let rate = latest_downloaded_bytes / AVERAGE_RATE_OVER_SECS as usize;
        let rate_str = format!("{} / s", ByteSize::b(rate as u64));
        self.stream.queue(Print(rate_str))?;
        self.stream.queue(Clear(ClearType::UntilNewLine))?;

        Ok(())
    }

    /// Perform the given output on the given file's line
    fn on_file_line(&mut self, file_idx: usize, what: impl Fn(&mut Self) -> Result<()>) -> Result<()> {
        self.stream.queue(MoveToNextLine(file_idx as u16 + 1))?;
        what(self)?;
        self.stream.queue(MoveToPreviousLine(file_idx as u16 + 1))?;
        self.print_first_row()?;
        self.stream.flush()?;
        Ok(())
    }

    /// Print the progress of the given file
    fn print_file_progress(&mut self, file_idx: usize) -> Result<()> {
        // Print the file name
        let file_name = &self.file_names[file_idx];
        let first_name_part = file_name[..file_name.len().min(MAX_FILE_NAME_LEN)].to_string();
        self.stream.queue(Print(format!("{}: ", first_name_part)))?;
        self.stream.queue(PrintStyledContent("[".blue()))?;
        for (superblock_idx, superblock_val) in self.file_states[file_idx].iter().enumerate() {
            let superblock_length: usize;
            // If this is the last superblock, then it may be shorter than the others
            if superblock_idx == self.file_states[file_idx].len() - 1 {
                let maybe_superblock_len =  self.file_lengths[file_idx] as usize % SUPERBLOCK_STEPS.len();
                if maybe_superblock_len == 0 {
                    superblock_length = SUPERBLOCK_STEPS.len();
                } else {
                    superblock_length = maybe_superblock_len as usize;
                }
            } else {
                superblock_length = SUPERBLOCK_STEPS.len()
            }

            if superblock_val >= &superblock_length {  // Every block in this superblock is done
                self.stream.queue(PrintStyledContent("X".green()))?;
                continue;
            }
            let superblock_symbol = SUPERBLOCK_STEPS[*superblock_val];
            if *superblock_val == 0 {  // This superblock is not started yet
                self.stream.queue(PrintStyledContent(superblock_symbol.yellow()))?;
            } else { // This superblock is in progress
                self.stream.queue(PrintStyledContent(superblock_symbol.cyan()))?;
            }
        }
        self.stream.queue(PrintStyledContent("]".blue()))?;
        self.stream.queue(Clear(ClearType::UntilNewLine))?;
        Ok(())
    }
}