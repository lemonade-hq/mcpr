# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2025-03-20

### Added
- Comprehensive MCP.md documentation with Mermaid diagrams
  - Detailed explanation of Model Context Protocol concepts
  - Visual representation of client-server architecture
  - Schema visualization for better understanding
  - Implementation examples and best practices
- Updated template generator to use the latest MCPR version
  - Generated projects now automatically use the current crate version
  - Templates include commented options for local development

### Improved
- Enhanced SSE transport template generation
  - Fixed template structure for better developer experience
  - Improved error handling in generated code
  - Added more comprehensive comments for easier customization
- Updated documentation for template usage
  - Clearer instructions for generating projects
  - Better examples of customizing templates
- Added type aliases for complex callback types
  - Improved code readability
  - Addressed Clippy warnings for complex types

### Fixed
- Template generation issues for SSE transport
  - Resolved path handling in generated projects
  - Fixed dependency management in template projects
- Minor code quality improvements
  - Addressed Clippy warnings
  - Enhanced code documentation

## [0.2.2] - 2025-03-15

### Added
- Enhanced GitHub Tools example with improved client-server architecture
  - Added repository search functionality
  - Implemented user-friendly interactive mode with colored output
  - Added progress indicators for operations
  - Improved error handling and user feedback

### Improved
- Simplified GitHub Tools example UI
  - Removed complex terminal UI in favor of a more intuitive interactive mode
  - Added default values for common operations
  - Enhanced formatting of search results and query responses
- Updated documentation
  - Added comprehensive instructions for running servers in background
  - Improved explanation of client-server architecture
  - Added guide for generating and extending MCP templates
  - Enhanced main README with direct links to examples

### Fixed
- Client-server disconnect handling in GitHub Tools example
  - Improved server process management
  - Enhanced client reconnection capabilities
  - Better error messages for connection issues

## [0.2.1] - 2025-03-15

### Added
- Server-Sent Events (SSE) transport implementation
  - Client registration and unique client ID assignment
  - Client-specific message queues
  - Polling mechanism for clients to receive messages
  - Proper error handling and logging
  - Support for all MCP message types
- Comprehensive documentation for generating and testing projects
  - Detailed instructions for both stdio and SSE transports
  - Troubleshooting guides for common issues
  - Advanced testing scenarios

### Fixed
- SSE transport message handling
  - Fixed client message queue management
  - Improved HTTP request routing for SSE endpoints
  - Enhanced error handling for network issues
  - Added proper timeout handling for message reception
- Template generation for SSE projects
  - Updated templates to use the correct transport configuration
  - Fixed dependency management in generated projects

### Improved
- Enhanced logging throughout the codebase
  - Added detailed debug and trace logging
  - Improved error messages for better troubleshooting
- Updated README with comprehensive testing guide
  - Step-by-step instructions for generating and testing projects
  - Examples of expected output for both transport types
  - Common issues and their solutions

### Note
- Version 0.2.0 was yanked and replaced with 0.2.1 with the same features and fixes

## [0.1.0] - 2024-03-12

### Added
- Initial release of the MCPR library
- Complete implementation of the MCP schema
- Transport layer for communication (stdio)
- CLI tools for generating server and client stubs
- Generator for creating MCP server and client stubs
- Examples demonstrating various MCP use cases:
  - Simple MCP examples
  - Browser search examples
  - Tool caller examples
  - Web scrape examples
  - Direct web scraper example
  - MCP web scraper examples 