# Architecture Documentation

This directory contains comprehensive architecture documentation for the Rust Things library.

## Documents

### Core API Design
- **[Core API Design](core-api-design.md)** - Comprehensive API design covering data structures, database interfaces, error handling, caching, and serialization patterns.

### Caching Strategy
- **[Caching Strategy](caching-strategy.md)** - Multi-level caching architecture with L1 (memory), L2 (disk), and L3 (database) caches, including performance optimization and monitoring.

### Error Handling
- **[Error Handling](error-handling.md)** - Robust error handling strategy with comprehensive error types, recovery strategies, and monitoring capabilities.

### Serialization Patterns
- **[Serialization Patterns](serialization-patterns.md)** - Multi-format serialization support with version compatibility, custom serializers, and performance optimization.

## Design Principles

### 1. Performance First
- **Async/Await**: All I/O operations are asynchronous
- **Connection Pooling**: Reuse database connections efficiently
- **Multi-level Caching**: Memory, disk, and database caching
- **Batch Operations**: Support for bulk operations
- **Lazy Loading**: Load related data only when needed

### 2. Type Safety
- **Strong Typing**: Use Rust's type system to prevent runtime errors
- **Newtype Patterns**: Wrap primitive types for domain-specific meaning
- **Enum-based State**: Use enums for status and type fields
- **Option Types**: Explicit handling of optional data

### 3. Error Handling
- **Result Types**: All operations return `Result<T, ThingsError>`
- **Error Context**: Rich error information with context
- **Error Recovery**: Graceful handling of recoverable errors
- **Error Propagation**: Proper error bubbling with context

### 4. Caching Support
- **Multi-level Caching**: Memory and disk caching
- **Cache Invalidation**: Smart invalidation strategies
- **Cache Statistics**: Performance monitoring
- **Configurable TTL**: Time-to-live configuration

### 5. Serialization Support
- **Serde Integration**: Full serialization support
- **Multiple Formats**: JSON, MessagePack, Bincode, CBOR, YAML, TOML
- **Version Compatibility**: Backward/forward compatibility
- **Custom Serializers**: Domain-specific serialization

## Architecture Overview

```
┌─────────────────┐
│   Application   │ ← CLI, MCP Server
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   Core Library  │ ← things-core
│   - Models      │
│   - Database    │
│   - Cache       │
│   - Errors      │
│   - Serialization│
└─────────────────┘
         │
         ▼
┌─────────────────┐
│   Common Types  │ ← things-common
└─────────────────┘
```

## Key Components

### Data Models
- **Task**: Main entity with comprehensive metadata
- **Project**: Task organization and grouping
- **Area**: Project organization and grouping
- **Tag**: Categorization and labeling
- **ChecklistItem**: Task breakdown and subtasks

### Database Layer
- **ThingsDatabase**: Main database interface
- **Transaction**: Database transaction management
- **Query Operations**: Task, project, area, and tag operations
- **Connection Pooling**: Efficient connection management

### Caching Layer
- **L1 Cache**: Memory cache using Moka
- **L2 Cache**: Disk cache using SQLite
- **L3 Cache**: Database query result cache
- **Cache Manager**: Unified cache interface

### Error Handling
- **ThingsError**: Comprehensive error types
- **Error Context**: Rich error information
- **Recovery Strategies**: Retry logic and circuit breakers
- **Error Monitoring**: Statistics and alerting

### Serialization
- **Multi-format Support**: JSON, MessagePack, Bincode, CBOR, YAML, TOML
- **Version Compatibility**: Schema evolution support
- **Custom Serializers**: Domain-specific serialization
- **Performance Optimization**: Buffer pooling and compression

## Implementation Status

✅ **Core Data Structures** - Defined with comprehensive metadata  
✅ **Database Interface** - Designed with async operations and transactions  
✅ **Error Handling** - Comprehensive error types and recovery strategies  
✅ **Caching Interface** - Multi-level caching with performance monitoring  
✅ **Serialization Patterns** - Multi-format support with version compatibility  
✅ **API Documentation** - Comprehensive documentation structure  

## Next Steps

1. **Implementation** - Begin implementing the core library based on this architecture
2. **Testing** - Create comprehensive test suite
3. **Performance** - Benchmark and optimize performance
4. **Documentation** - Complete API documentation
5. **Examples** - Create usage examples and tutorials

## Contributing

When contributing to the architecture, please:

1. Update the relevant documentation files
2. Follow the established design principles
3. Consider performance implications
4. Maintain backward compatibility
5. Update this README if adding new components

## References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Serde Documentation](https://serde.rs/)
- [Tokio Documentation](https://tokio.rs/)
- [Moka Caching](https://github.com/moka-rs/moka)
- [Things 3 Database Schema](https://culturedcode.com/things/support/articles/2803573/)
