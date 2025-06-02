# elinKernel Build System

# Configuration
TARGET := riscv64gc-unknown-none-elf
KERNEL_NAME := kernel
KERNEL_BIN := kernel.bin

# Directories
TARGET_DIR := target
BUILD_DIR := $(TARGET_DIR)/$(TARGET)/release

# Rust/Cargo settings
RUSTFLAGS := -C target-cpu=generic-rv64 -C target-feature=+m,+a,+c -C link-arg=-Tsrc/linker.ld

# Tools (with fallbacks)
OBJCOPY := $(shell command -v rust-objcopy 2>/dev/null || echo "~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-objcopy")
OBJDUMP := $(shell command -v rust-objdump 2>/dev/null || echo "~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-objdump")

# Default target
.PHONY: all
all: build

# Clean and build (equivalent to build.sh)
.PHONY: build
build: clean $(KERNEL_BIN) info

# Clean build artifacts
.PHONY: clean
clean:
	@echo "🧹 Cleaning target directory..."
	rm -rf $(TARGET_DIR)
	cargo clean
	rm -f $(KERNEL_BIN)

# Build ELF kernel
$(BUILD_DIR)/$(KERNEL_NAME): src/*.rs Cargo.toml src/linker.ld
	@echo "🔨 Building elinKernel kernel with release profile..."
	RUSTFLAGS="$(RUSTFLAGS)" cargo build --release --target $(TARGET)
	@if [ ! -f "$@" ]; then \
		echo "❌ Error: ELF file not found. Build failed."; \
		exit 1; \
	fi

# Convert ELF to binary
$(KERNEL_BIN): $(BUILD_DIR)/$(KERNEL_NAME)
	@echo "📊 ELF file information:"
	@file $<
	@echo "📊 ELF sections:"
	@$(OBJDUMP) -h $<
	@echo "📊 First few instructions:"
	@$(OBJDUMP) -d $< | head -n 20
	@echo "🔧 Creating bootable binary image..."
	$(OBJCOPY) \
		--strip-all \
		--set-section-flags .bss=alloc,load,contents \
		--set-section-flags .text=alloc,load,contents \
		--set-section-flags .rodata=alloc,load,contents \
		--set-section-flags .data=alloc,load,contents \
		-O binary \
		$< $@
	@if [ ! -f "$@" ]; then \
		echo "❌ Error: kernel.bin not found. Build failed."; \
		exit 1; \
	fi

# Show build information
.PHONY: info
info: $(KERNEL_BIN)
	@echo "✅ Kernel built successfully"
	@echo "📊 Binary file information:"
	@file $(KERNEL_BIN)
	@echo "📊 Binary size:"
	@ls -l $(KERNEL_BIN)
	@echo "📊 First 32 bytes of binary (hex):"
	@hexdump -C -n 32 $(KERNEL_BIN)

# Install required tools
.PHONY: install-tools
install-tools:
	@echo "🔧 Installing required tools..."
	rustup target add $(TARGET)
	@echo "✅ Tools installed!"

# Show help
.PHONY: help
help:
	@echo "elinKernel Build System"
	@echo "=================="
	@echo "Targets:"
	@echo "  build         - Clean and build kernel (replaces build.sh)"
	@echo "  clean         - Clean build artifacts"
	@echo "  install-tools - Install required Rust tools"
	@echo "  help          - Show this help"
	@echo ""
	@echo "After building, use ./run.sh to run in QEMU" 