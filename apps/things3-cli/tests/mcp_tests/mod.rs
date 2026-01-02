//! MCP server tests organized into logical modules
//!
//! This module provides comprehensive testing for the MCP server implementation,
//! organized by functionality: tools, prompts, resources, and error handling.

mod common;
mod error_tests;
mod prompt_tests;
mod resource_tests;
mod tool_tests;

// Re-export common test utilities for use in submodules
pub(crate) use common::*;

