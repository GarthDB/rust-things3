//! I/O abstraction layer for MCP server to enable testing
//!
//! This module provides a trait-based abstraction over I/O operations,
//! allowing the MCP server to work with both real stdin/stdout (production)
//! and mock I/O streams (testing).

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream};

/// Trait for MCP I/O operations
///
/// This trait abstracts over the I/O operations needed by the MCP server,
/// allowing for both production (stdin/stdout) and test (mock) implementations.
#[async_trait]
pub trait McpIo: Send + Sync {
    /// Read a line from the input stream
    ///
    /// Returns `Ok(Some(line))` if a line was read, `Ok(None)` on EOF,
    /// or `Err` if an error occurred.
    async fn read_line(&mut self) -> std::io::Result<Option<String>>;

    /// Write a line to the output stream
    ///
    /// The line should NOT include a trailing newline - it will be added automatically.
    async fn write_line(&mut self, line: &str) -> std::io::Result<()>;

    /// Flush the output stream
    async fn flush(&mut self) -> std::io::Result<()>;
}

/// Production I/O implementation using stdin/stdout
pub struct StdIo {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
    buffer: String,
}

impl StdIo {
    /// Create a new StdIo instance using stdin/stdout
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
            buffer: String::new(),
        }
    }
}

impl Default for StdIo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl McpIo for StdIo {
    async fn read_line(&mut self) -> std::io::Result<Option<String>> {
        self.buffer.clear();
        let bytes_read = self.reader.read_line(&mut self.buffer).await?;

        if bytes_read == 0 {
            Ok(None) // EOF
        } else {
            Ok(Some(self.buffer.trim().to_string()))
        }
    }

    async fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        Ok(())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush().await
    }
}

/// Mock I/O implementation for testing using DuplexStream
pub struct MockIo {
    reader: BufReader<tokio::io::ReadHalf<DuplexStream>>,
    writer: tokio::io::WriteHalf<DuplexStream>,
    buffer: String,
}

impl MockIo {
    /// Create a new MockIo instance from a DuplexStream
    ///
    /// The DuplexStream should be the "server" side of the duplex pair.
    /// The "client" side should be used to send requests and read responses.
    pub fn new(stream: DuplexStream) -> Self {
        let (read_half, write_half) = tokio::io::split(stream);
        Self {
            reader: BufReader::new(read_half),
            writer: write_half,
            buffer: String::new(),
        }
    }

    /// Create a pair of connected MockIo instances for testing
    ///
    /// Returns (server_io, client_io) where:
    /// - server_io: Used by the MCP server
    /// - client_io: Used by tests to send requests and read responses
    pub fn create_pair(buffer_size: usize) -> (Self, Self) {
        let (client_stream, server_stream) = tokio::io::duplex(buffer_size);
        let server_io = Self::new(server_stream);
        let client_io = Self::new(client_stream);
        (server_io, client_io)
    }
}

#[async_trait]
impl McpIo for MockIo {
    async fn read_line(&mut self) -> std::io::Result<Option<String>> {
        self.buffer.clear();
        let bytes_read = self.reader.read_line(&mut self.buffer).await?;

        if bytes_read == 0 {
            Ok(None) // EOF
        } else {
            Ok(Some(self.buffer.trim().to_string()))
        }
    }

    async fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        Ok(())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_io_bidirectional() {
        let (mut server_io, mut client_io) = MockIo::create_pair(1024);

        // Client writes, server reads
        client_io.write_line("Hello from client").await.unwrap();
        client_io.flush().await.unwrap();

        let line = server_io.read_line().await.unwrap();
        assert_eq!(line, Some("Hello from client".to_string()));

        // Server writes, client reads
        server_io.write_line("Hello from server").await.unwrap();
        server_io.flush().await.unwrap();

        let line = client_io.read_line().await.unwrap();
        assert_eq!(line, Some("Hello from server".to_string()));
    }

    #[tokio::test]
    async fn test_mock_io_multiple_lines() {
        let (mut server_io, mut client_io) = MockIo::create_pair(1024);

        // Write multiple lines
        client_io.write_line("line1").await.unwrap();
        client_io.write_line("line2").await.unwrap();
        client_io.write_line("line3").await.unwrap();
        client_io.flush().await.unwrap();

        // Read them back
        assert_eq!(
            server_io.read_line().await.unwrap(),
            Some("line1".to_string())
        );
        assert_eq!(
            server_io.read_line().await.unwrap(),
            Some("line2".to_string())
        );
        assert_eq!(
            server_io.read_line().await.unwrap(),
            Some("line3".to_string())
        );
    }

    #[tokio::test]
    async fn test_mock_io_empty_lines() {
        let (mut server_io, mut client_io) = MockIo::create_pair(1024);

        // Write empty line
        client_io.write_line("").await.unwrap();
        client_io.flush().await.unwrap();

        let line = server_io.read_line().await.unwrap();
        assert_eq!(line, Some("".to_string()));
    }

    #[tokio::test]
    async fn test_mock_io_eof() {
        let (mut server_io, client_io) = MockIo::create_pair(1024);

        // Drop the client to close the stream
        drop(client_io);

        // Reading should return None (EOF)
        let line = server_io.read_line().await.unwrap();
        assert_eq!(line, None);
    }
}
