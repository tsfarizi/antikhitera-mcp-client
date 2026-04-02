# 📚 Antikythera Documentation

Welcome to the Antikythera MCP Framework documentation collection.

## 📖 Available Documentation

### Getting Started

| Document | Description |
|:---------|:------------|
| [BUILD.md](BUILD.md) | Build instructions, feature flags, and compilation guide |
| [TESTING_GUIDE.md](TESTING_GUIDE.md) | Testing guide with conditional test execution |

### Architecture & Implementation

| Document | Description |
|:---------|:------------|
| [TRANSFORMATION_PLAN.md](TRANSFORMATION_PLAN.md) | Architectural transformation plan (Phase 1) |
| [PHASE2_IMPLEMENTATION.md](PHASE2_IMPLEMENTATION.md) | Stateless multi-agent implementation (Phase 2) |
| [CLEANUP_SUMMARY.md](CLEANUP_SUMMARY.md) | Code cleanup and workspace restructuring summary |

### Component Documentation

| Document | Description |
|:---------|:------------|
| [CLI_DOCUMENTATION.md](CLI_DOCUMENTATION.md) | CLI usage, TUI interface, and commands |
| [FFI_DOCUMENTATION.md](FFI_DOCUMENTATION.md) | FFI/C API reference for C/C++ integration |
| [POSTCARD_CACHE.md](POSTCARD_CACHE.md) | Postcard binary serialization for state persistence |

## 🚀 Quick Links

- **Main README**: [../README.md](../README.md)
- **Tests README**: [tests/README.md](../tests/README.md)
- **GitHub Repository**: [https://github.com/tsfarizi/antikythera-mcp-framework](https://github.com/tsfarizi/antikythera-mcp-framework)

## 📋 Documentation Index

### Build & Deploy
- [BUILD.md](BUILD.md) - Complete build guide
  - Native builds
  - WASM builds
  - Feature flags
  - Cross-compilation

### Testing
- [TESTING_GUIDE.md](TESTING_GUIDE.md) - Testing guide
  - Conditional test execution
  - Environment checking
  - Test utilities
  - CI/CD integration

### Architecture
- [TRANSFORMATION_PLAN.md](TRANSFORMATION_PLAN.md) - Phase 1: Workspace & FSM
  - Rust workspace structure
  - Feature flags implementation
  - FSM implementation
- [PHASE2_IMPLEMENTATION.md](PHASE2_IMPLEMENTATION.md) - Phase 2: Stateless Multi-Agent
  - Memory provider
  - FSM-driven runner
  - Pause/Resume functionality

### Components
- [CLI_DOCUMENTATION.md](CLI_DOCUMENTATION.md) - CLI & TUI
  - Mode selector
  - Chat interface
  - Commands
  - Keyboard shortcuts
- [FFI_DOCUMENTATION.md](FFI_DOCUMENTATION.md) - FFI Bindings
  - C API reference
  - Usage examples (C, C++, Python, Node.js)
  - Memory management
- [POSTCARD_CACHE.md](POSTCARD_CACHE.md) - State Persistence
  - Postcard serialization
  - Memory provider trait
  - Filesystem provider

### Project History
- [CLEANUP_SUMMARY.md](CLEANUP_SUMMARY.md) - Code restructuring
  - Workspace migration
  - Duplicate code removal
  - Import path updates

## 🔧 For Contributors

1. **Read First**: Start with [TRANSFORMATION_PLAN.md](TRANSFORMATION_PLAN.md) to understand the architecture
2. **Build**: Follow [BUILD.md](BUILD.md) for build instructions
3. **Test**: Use [TESTING_GUIDE.md](TESTING_GUIDE.md) for testing
4. **Contribute**: Check main [README.md](../README.md) for contribution guidelines

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/tsfarizi/antikythera-mcp-framework/issues)
- **Discussions**: [GitHub Discussions](https://github.com/tsfarizi/antikythera-mcp-framework/discussions)
- **Documentation**: This folder

---

*Last Updated: 2026-04-01*  
*Version: 0.8.0*
