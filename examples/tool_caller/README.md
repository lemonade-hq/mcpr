# Tool Caller Examples

This directory contains examples demonstrating how to implement tool calling functionality using the Model Context Protocol (MCP).

## Contents

- `llm_tool_caller.rs`: An example that uses an LLM to call MCP tools
- `standalone_tool_caller.rs`: A standalone tool caller that doesn't require an LLM

## Running the Examples

### LLM Tool Caller

To run the LLM tool caller:

```bash
cargo run --example llm_tool_caller
```

Or use the provided script:

```bash
./run_llm_tool_caller.sh
```

### Standalone Tool Caller

To run the standalone tool caller:

```bash
cargo run --example standalone_tool_caller
```

Or use the provided script:

```bash
./run_standalone_tool_caller.sh
```

## What These Examples Demonstrate

These examples demonstrate:

1. How to implement a tool caller that can interact with MCP servers
2. How to use an LLM to determine which tools to call based on user input
3. How to implement a standalone tool caller that doesn't require an LLM
4. How to handle complex tool calling scenarios with multiple tools

The tool caller examples show how MCP can be used to create flexible, extensible systems that can leverage various tools to accomplish tasks. 