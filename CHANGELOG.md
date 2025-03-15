# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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