// IPC utilities for cross-process Vulkan texture sharing
//
// This module provides simple file-based IPC for passing texture handles
// between producer and consumer processes. In production, you might use:
// - Named pipes (Windows)
// - Unix domain sockets (Linux)
// - Shared memory with synchronization primitives
// - Message queues

use serde::{Deserialize, Serialize};
use std::{
    fs,
    io,
    path::Path,
    thread,
    time::Duration,
};

/// Message format for IPC communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    /// Texture handle with metadata
    TextureHandle {
        raw_handle: u64,
        memory_type_index: u32,
        size: u64,
        width: u32,
        height: u32,
        format: String,
    },
    /// Semaphore handle for synchronization
    SemaphoreHandle {
        raw_handle: u64,
    },
    /// Signal that producer is ready
    ProducerReady,
    /// Signal that consumer is ready
    ConsumerReady,
    /// Signal that producer has rendered a frame
    FrameReady {
        frame_number: u32,
    },
    /// Signal to shutdown
    Shutdown,
}

/// Simple file-based IPC channel (producer writes, consumer reads)
pub struct IpcChannel {
    path: String,
}

impl IpcChannel {
    pub fn new(name: &str) -> Self {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("geyser_ipc_{}.bin", name));
        Self {
            path: path.to_string_lossy().to_string(),
        }
    }

    /// Send a message (blocking write)
    pub fn send(&self, message: &IpcMessage) -> io::Result<()> {
        let encoded = bincode::serialize(message)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        // Write atomically by using a temp file + rename
        let temp_path = format!("{}.tmp", self.path);
        fs::write(&temp_path, &encoded)?;
        fs::rename(&temp_path, &self.path)?;
        
        Ok(())
    }

    /// Receive a message (blocking read with retry)
    pub fn receive(&self, timeout_secs: u64) -> io::Result<IpcMessage> {
        let max_attempts = timeout_secs * 10; // Check every 100ms
        
        for _ in 0..max_attempts {
            if Path::new(&self.path).exists() {
                match fs::read(&self.path) {
                    Ok(data) => {
                        let message: IpcMessage = bincode::deserialize(&data)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                        return Ok(message);
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        // File was deleted between exists check and read, retry
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Timeout waiting for message at {}", self.path),
        ))
    }

    /// Try to receive without blocking
    pub fn try_receive(&self) -> io::Result<Option<IpcMessage>> {
        if !Path::new(&self.path).exists() {
            return Ok(None);
        }

        let data = fs::read(&self.path)?;
        let message: IpcMessage = bincode::deserialize(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(message))
    }

    /// Clear the channel (delete the file)
    pub fn clear(&self) -> io::Result<()> {
        if Path::new(&self.path).exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// Get the path for debugging
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for IpcChannel {
    fn drop(&mut self) {
        let _ = self.clear();
    }
}

/// Bi-directional IPC channel pair
pub struct IpcChannelPair {
    pub send: IpcChannel,
    pub receive: IpcChannel,
}

impl IpcChannelPair {
    /// Create a producer channel pair (sends to consumer, receives from consumer)
    pub fn producer() -> Self {
        Self {
            send: IpcChannel::new("producer_to_consumer"),
            receive: IpcChannel::new("consumer_to_producer"),
        }
    }

    /// Create a consumer channel pair (sends to producer, receives from producer)
    pub fn consumer() -> Self {
        Self {
            send: IpcChannel::new("consumer_to_producer"),
            receive: IpcChannel::new("producer_to_consumer"),
        }
    }

    /// Clear both channels
    pub fn clear_all(&self) -> io::Result<()> {
        self.send.clear()?;
        self.receive.clear()?;
        Ok(())
    }
}

/// Helper to convert Geyser format to string
pub fn format_to_string(format: geyser::common::TextureFormat) -> String {
    format!("{:?}", format)
}

/// Helper to convert string to Geyser format
pub fn string_to_format(s: &str) -> Result<geyser::common::TextureFormat, String> {
    use geyser::common::TextureFormat;
    
    match s {
        "Rgba8Unorm" => Ok(TextureFormat::Rgba8Unorm),
        "Bgra8Unorm" => Ok(TextureFormat::Bgra8Unorm),
        "Rgba8Srgb" => Ok(TextureFormat::Rgba8Srgb),
        "Bgra8Srgb" => Ok(TextureFormat::Bgra8Srgb),
        "R8Unorm" => Ok(TextureFormat::R8Unorm),
        "Rg8Unorm" => Ok(TextureFormat::Rg8Unorm),
        "R16Float" => Ok(TextureFormat::R16Float),
        "Rg16Float" => Ok(TextureFormat::Rg16Float),
        "Rgba16Float" => Ok(TextureFormat::Rgba16Float),
        "R16Uint" => Ok(TextureFormat::R16Uint),
        "R16Sint" => Ok(TextureFormat::R16Sint),
        "R32Float" => Ok(TextureFormat::R32Float),
        "Rg32Float" => Ok(TextureFormat::Rg32Float),
        "Rgba32Float" => Ok(TextureFormat::Rgba32Float),
        "R32Uint" => Ok(TextureFormat::R32Uint),
        "R32Sint" => Ok(TextureFormat::R32Sint),
        "Depth32Float" => Ok(TextureFormat::Depth32Float),
        "Depth24Plus" => Ok(TextureFormat::Depth24Plus),
        "Depth24PlusStencil8" => Ok(TextureFormat::Depth24PlusStencil8),
        "Rgb10a2Unorm" => Ok(TextureFormat::Rgb10a2Unorm),
        "Rg11b10Float" => Ok(TextureFormat::Rg11b10Float),
        _ => Err(format!("Unknown texture format: {}", s)),
    }
}
