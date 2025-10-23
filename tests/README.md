# Geyser Tests

## Prerequisites

### Vulkan Tests
To run Vulkan integration tests, you need:
- **Vulkan SDK** installed from https://vulkan.lunarg.com/
- Vulkan-capable GPU with appropriate drivers
- On Windows: `vulkan-1.lib` must be in your library path
- On Linux: `libvulkan.so` must be installed

### Metal Tests
To run Metal tests (macOS only):
- macOS 10.13 or later
- Metal-capable GPU

## Running Tests

### All Tests (requires Vulkan SDK)
```bash
cargo test --features vulkan
```

### Unit Tests Only (no GPU required)
```bash
cargo test --lib --features vulkan
```

### Integration Tests (requires Vulkan SDK + GPU)
```bash
cargo test --test integration_tests --features vulkan
```

### Specific Backend
```bash
# Vulkan only
cargo test --features vulkan

# Metal only (macOS)
cargo test --features metal

# Both
cargo test --features vulkan,metal
```

## Test Structure

- `tests/integration_tests.rs` - Full integration tests requiring GPU
- `src/*/mod.rs` - Unit tests embedded in each module (future)
- `examples/` - Runnable examples demonstrating usage

## Known Limitations

1. Integration tests require actual GPU hardware and drivers
2. Cross-process sharing tests require elevated permissions on some platforms
3. Windows tests require the Vulkan SDK to be installed
4. Some tests may be skipped on platforms without required features
