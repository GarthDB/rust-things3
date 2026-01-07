//! MCP server tests organized into logical modules
//!
//! This module provides comprehensive testing for the MCP server implementation,
//! organized by functionality: tools, prompts, resources, and error handling.

#![cfg(feature = "mcp-server")]

pub(crate) mod common;
mod error_tests;
mod prompt_tests;
mod resource_tests;
mod tool_tests;
