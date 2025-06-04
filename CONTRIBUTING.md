# Contributing to elinOS

Thank you for your interest in contributing to elinOS! This experimental RISC-V kernel welcomes contributions from developers at all levels, whether you're interested in systems programming, kernel development, or learning about operating systems.

## 🌟 Ways to Contribute

- **🐛 Bug Reports**: Help us identify and fix issues
- **💡 Feature Requests**: Suggest new functionality or improvements
- **📝 Documentation**: Improve existing docs or add new guides
- **🔧 Code Contributions**: Implement features, fix bugs, or optimize performance
- **🧪 Testing**: Help test the kernel on different systems and configurations
- **📚 Educational Content**: Create tutorials, examples, or learning materials

## 🚀 Getting Started

### Prerequisites

Make sure you have the development environment set up:

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup default nightly

# Add RISC-V target
rustup target add riscv64gc-unknown-none-elf

# Install QEMU
sudo apt install qemu-system-riscv64  # Ubuntu/Debian
# or
brew install qemu                      # macOS
```

### First-Time Setup

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/your-username/elinOS.git
   cd elinOS
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/original-owner/elinOS.git
   ```
4. **Test the build**:
   ```bash
   make build
   make run
   ```

## 🛠️ Development Workflow

### 1. Choose an Issue

- Browse [open issues](https://github.com/original-owner/elinOS/issues)
- Look for issues labeled `good first issue` for beginners
- Check `help wanted` for areas where we need assistance
- Comment on the issue to let others know you're working on it

### 2. Create a Feature Branch

```bash
# Update your main branch
git checkout main
git pull upstream main

# Create a new branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/issue-description
```

### 3. Make Your Changes

#### Code Style Guidelines

- Follow Rust idioms and best practices
- Use descriptive variable and function names
- Document public APIs with doc comments
- Add `#![no_std]` compatible code only
- Prefer safe abstractions over unsafe code

#### Example of Good Code Style:

```rust
/// Allocates memory using the most appropriate allocator for the given size
///
/// # Arguments
/// * `size` - Number of bytes to allocate (must be > 0)
/// * `alignment` - Required alignment (must be power of 2)
///
/// # Returns
/// * `Ok(ptr)` - Non-null pointer to allocated memory
/// * `Err(AllocError)` - Allocation failed
///
/// # Examples
/// ```
/// let ptr = allocate_aligned_memory(1024, 16)?;
/// // Use the memory...
/// deallocate_memory(ptr, 1024);
/// ```
pub fn allocate_aligned_memory(size: usize, alignment: usize) -> AllocResult<NonNull<u8>> {
    if size == 0 || !alignment.is_power_of_two() {
        return Err(AllocError::InvalidParameters);
    }
    
    // Implementation...
}
```

#### Safety Guidelines

Always document `unsafe` blocks:

```rust
unsafe {
    // SAFETY: ptr is guaranteed to be valid because:
    // 1. It was just allocated by our allocator
    // 2. We verified it's non-null above
    // 3. The size matches what we allocated
    core::ptr::write(ptr, value);
}
```

### 4. Testing

Run the full test suite before submitting:

```bash
# Run all tests
make check-all

# Specific test categories
make test           # Unit tests
make integration    # Integration tests
make clippy         # Linting
make format         # Code formatting
```

Add tests for new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_allocation_basic() {
        let ptr = allocate_memory(1024);
        assert!(ptr.is_some());
        
        // Test writing to the memory
        if let Some(addr) = ptr {
            unsafe {
                *(addr as *mut u8) = 42;
                assert_eq!(*(addr as *const u8), 42);
            }
            deallocate_memory(addr, 1024);
        }
    }
    
    #[test]
    fn test_zero_size_allocation() {
        let ptr = allocate_memory(0);
        assert!(ptr.is_none());
    }
}
```

### 5. Documentation

Update documentation for any user-facing changes:

- Update inline documentation for public APIs
- Add examples to doc comments
- Update relevant files in `docs/` directory
- Consider adding entries to the README if significant

### 6. Commit Your Changes

Write clear, descriptive commit messages:

```bash
git add .
git commit -m "memory: Add aligned allocation support

- Implement allocate_aligned_memory() function
- Add support for power-of-2 alignments up to page size
- Include comprehensive test coverage
- Update documentation with usage examples

Fixes #123"
```

### 7. Submit a Pull Request

```bash
# Push to your fork
git push origin feature/your-feature-name

# Then create a PR on GitHub
```

## 📋 Pull Request Guidelines

### PR Title Format

Use one of these prefixes:

- `feat:` - New features
- `fix:` - Bug fixes  
- `docs:` - Documentation updates
- `test:` - Test additions or modifications
- `refactor:` - Code refactoring
- `perf:` - Performance improvements
- `style:` - Code style/formatting changes

Examples:
- `feat: Add VirtIO network device support`
- `fix: Resolve memory corruption in buddy allocator`
- `docs: Update architecture guide with memory zones`

### PR Description Template

```markdown
## Summary
Brief description of what this PR does.

## Changes
- List of specific changes made
- Use bullet points for clarity
- Include any breaking changes

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Performance impact assessed (if applicable)

## Documentation
- [ ] Code comments added/updated
- [ ] API documentation updated
- [ ] User documentation updated (if needed)

## Related Issues
Fixes #123
Relates to #456
```

### Code Review Process

1. **Automated Checks**: CI will run tests, linting, and formatting checks
2. **Manual Review**: Maintainers will review code for:
   - Correctness and safety
   - Code style and conventions
   - Performance implications
   - Documentation quality
3. **Feedback**: Address any requested changes
4. **Approval**: Once approved, your PR will be merged

## 🎯 Contribution Areas

### 🧠 Memory Management
- Implement new allocation algorithms
- Optimize existing allocators
- Add memory debugging tools
- Improve memory safety

### 💾 Filesystem Support
- Add new filesystem types
- Improve existing filesystem implementations
- Add write support to read-only filesystems
- Optimize I/O performance

### 🔌 Device Drivers
- Implement new VirtIO device types
- Add support for other device interfaces
- Improve device detection and initialization
- Add hardware-specific optimizations

### 🔧 System Calls
- Implement new Linux-compatible system calls
- Improve existing system call implementations
- Add system call debugging and tracing
- Optimize system call performance

### 📊 Performance & Debugging
- Add performance monitoring tools
- Implement kernel debugging features
- Create benchmarking infrastructure
- Profile and optimize critical paths

### 📚 Documentation & Education
- Write tutorials and guides
- Create educational examples
- Improve API documentation
- Add code comments and explanations

## 🐛 Bug Reports

When reporting bugs, please include:

1. **Environment**: 
   - Host OS and version
   - QEMU version
   - Rust toolchain version

2. **Steps to Reproduce**:
   - Exact commands run
   - Configuration used
   - Input provided

3. **Expected vs Actual Behavior**:
   - What should happen
   - What actually happens

4. **Additional Context**:
   - Error messages or logs
   - Screenshots (if relevant)
   - Any debugging you've already done

### Bug Report Template

```markdown
**Environment**
- Host OS: Ubuntu 22.04
- QEMU: 7.0.0
- Rust: nightly-2024-01-15

**Steps to Reproduce**
1. Run `make build`
2. Execute `make run`
3. Type command `ls`

**Expected Behavior**
Should list files in the filesystem

**Actual Behavior**
Kernel panics with "invalid memory access"

**Additional Context**
```
[paste error output or logs here]
```

**Debugging Done**
- Tried with different QEMU versions
- Tested on fresh clone of repository
```

## 💡 Feature Requests

For feature requests, please describe:

1. **Problem**: What problem does this solve?
2. **Proposed Solution**: How should it work?
3. **Alternatives**: Other ways to solve this problem
4. **Implementation**: Any ideas on how to implement it
5. **Impact**: Who benefits and how?

## 🤝 Community Guidelines

### Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please:

- **Be respectful** to all community members
- **Be collaborative** in discussions and code reviews
- **Be constructive** when providing feedback
- **Be patient** with newcomers and learners
- **Be professional** in all interactions

### Communication Channels

- **GitHub Issues**: Bug reports, feature requests, technical discussions
- **Pull Requests**: Code review and implementation discussions
- **Discussions**: General questions, ideas, and community chat

## 📚 Learning Resources

### For Beginners

- [Rust Book](https://doc.rust-lang.org/book/) - Learn Rust fundamentals
- [Rust Embedded Book](https://docs.rust-embedded.org/book/) - `no_std` programming
- [RISC-V Specification](https://riscv.org/specifications/) - Understand the architecture
- [OSDev Wiki](https://wiki.osdev.org/) - Operating system development concepts

### For Advanced Contributors

- [docs/architecture.md](docs/architecture.md) - elinOS system design
- [docs/memory_improvements.md](docs/memory_improvements.md) - Memory management deep dive
- [docs/development.md](docs/development.md) - Advanced development topics

## 🏆 Recognition

Contributors will be recognized in:

- **CONTRIBUTORS.md** file
- **Release notes** for significant contributions
- **GitHub contributor graph**
- **Special mentions** in documentation

## ❓ Questions?

- Check [existing issues](https://github.com/original-owner/elinOS/issues) first
- Open a new issue with the `question` label
- Review our [documentation](docs/) for detailed information

---

**Thank you for contributing to elinOS!** 🚀

Your contributions help make this experimental kernel a valuable resource for learning and research in operating systems development. 