# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2024-01-03

### Added
- **Configuration Management**: Comprehensive configuration system for MCP server with YAML/JSON support
- **Real-time Updates**: WebSocket-based real-time updates and progress tracking
- **Authentication & Rate Limiting**: JWT and API key authentication with configurable rate limiting
- **MCP Middleware System**: Extensible middleware framework for cross-cutting concerns
- **Enhanced Error Handling**: MCP-specific error types with detailed error context
- **MCP Prompts Support**: Reusable template system for MCP prompts
- **MCP Resources Pattern**: Structured data exposure through MCP resources
- **Structured Logging**: Comprehensive logging and metrics collection system
- **Performance Infrastructure**: Caching and performance optimization with monitoring
- **Comprehensive Test Coverage**: Significantly improved test coverage (60% → 80%+)

### Improved
- **Database Schema**: Aligned with real Things3 database structure
- **Code Quality**: Resolved all clippy warnings and linting issues
- **CI/CD Pipeline**: Enhanced reliability and coverage reporting
- **Test Reliability**: Fixed race conditions and environment-specific test issues
- **Documentation**: Added comprehensive guides and API documentation

### Fixed
- **Security Vulnerabilities**: Updated dependencies to resolve security issues
- **Environment Variable Parsing**: Improved configuration parsing reliability
- **Test Isolation**: Resolved test interference and race conditions
- **Database I/O**: Fixed database access issues in MCP tests
- **Memory Management**: Optimized memory usage in caching systems

### Technical Improvements
- **Code Coverage**: Extensive test coverage improvements across all modules
- **Performance Monitoring**: Added metrics collection and performance tracking
- **Error Recovery**: Enhanced error handling and recovery mechanisms
- **Concurrent Access**: Improved thread safety and concurrent operations
- **Resource Management**: Better resource cleanup and lifecycle management

## [0.1.0] - 2024-XX-XX

### Added
- Initial release with basic Things 3 database access
- Core data models and CLI interface
- Basic MCP server functionality
- SQLite database integration
- Export functionality (JSON, CSV, Markdown, OPML)