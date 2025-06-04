# elinOS Makefile
# Professional build system for RISC-V experimental kernel

# =============================================================================
# Configuration Variables
# =============================================================================

# Project metadata
PROJECT_NAME := elinOS
KERNEL_NAME := kernel.bin
VERSION := 0.1.0

# Build configuration
TARGET := riscv64gc-unknown-none-elf
CARGO_FLAGS := --target $(TARGET)
RELEASE_FLAGS := --release
DEBUG_FLAGS := 

# Paths
BUILD_DIR := target/$(TARGET)
DEBUG_DIR := $(BUILD_DIR)/debug
RELEASE_DIR := $(BUILD_DIR)/release
DOCS_DIR := target/doc

# QEMU configuration
QEMU := qemu-system-riscv64
QEMU_MACHINE := virt
QEMU_CPU := rv64
QEMU_MEMORY := 128M
QEMU_SMP := 1

# QEMU firmware paths (common locations)
OPENSBI_PATHS := \
	/usr/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
	/usr/share/opensbi/opensbi-riscv64-generic-fw_dynamic.bin \
	/opt/homebrew/share/qemu/opensbi-riscv64-generic-fw_dynamic.bin \
	opensbi-riscv64-generic-fw_dynamic.bin

# Find OpenSBI firmware
OPENSBI := $(firstword $(wildcard $(OPENSBI_PATHS)))
ifeq ($(OPENSBI),)
    OPENSBI := default
endif

# Disk image configuration
DISK_SIZE := 32M
DISK_IMAGE := disk.img
DISK_MOUNT := /tmp/elinOS-mount

# Colors for pretty output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_RED := \033[31m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m
COLOR_MAGENTA := \033[35m
COLOR_CYAN := \033[36m

# =============================================================================
# Default Target
# =============================================================================

.DEFAULT_GOAL := help

# =============================================================================
# Help System
# =============================================================================

.PHONY: help
help: ## Show this help message
	@echo "$(COLOR_BOLD)$(COLOR_BLUE)$(PROJECT_NAME) v$(VERSION) - RISC-V Experimental Kernel$(COLOR_RESET)"
	@echo "$(COLOR_CYAN)=================================="
	@echo ""
	@echo "$(COLOR_BOLD)Build Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(build|clean)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(COLOR_BOLD)Run Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(run|qemu)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(COLOR_BOLD)Development Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(test|format|clippy|doc)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(COLOR_BOLD)Disk Image Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(disk|mount)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo "$(COLOR_BOLD)Environment Information:$(COLOR_RESET)"
	@echo "  Target: $(COLOR_CYAN)$(TARGET)$(COLOR_RESET)"
	@echo "  OpenSBI: $(COLOR_CYAN)$(OPENSBI)$(COLOR_RESET)"
	@echo "  QEMU: $(COLOR_CYAN)$(shell $(QEMU) --version 2>/dev/null | head -1 || echo 'Not found')$(COLOR_RESET)"

# =============================================================================
# Build Commands
# =============================================================================

.PHONY: build
build: ## Build the kernel (debug mode)
	@echo "$(COLOR_BLUE)Building $(PROJECT_NAME) (debug)...$(COLOR_RESET)"
	@cargo build $(CARGO_FLAGS) $(DEBUG_FLAGS)
	@echo "$(COLOR_GREEN)✓ Build completed: $(DEBUG_DIR)/$(KERNEL_NAME)$(COLOR_RESET)"

.PHONY: build-release
build-release: ## Build the kernel (release mode)
	@echo "$(COLOR_BLUE)Building $(PROJECT_NAME) (release)...$(COLOR_RESET)"
	@cargo build $(CARGO_FLAGS) $(RELEASE_FLAGS)
	@echo "$(COLOR_GREEN)✓ Release build completed: $(RELEASE_DIR)/$(KERNEL_NAME)$(COLOR_RESET)"

.PHONY: rebuild
rebuild: clean build ## Clean and rebuild the kernel

.PHONY: rebuild-release
rebuild-release: clean build-release ## Clean and rebuild the kernel (release)

.PHONY: clean
clean: ## Clean build artifacts
	@echo "$(COLOR_YELLOW)Cleaning build artifacts...$(COLOR_RESET)"
	@cargo clean
	@rm -f $(DISK_IMAGE)
	@echo "$(COLOR_GREEN)✓ Clean completed$(COLOR_RESET)"

.PHONY: check
check: ## Check code without building
	@echo "$(COLOR_BLUE)Checking code...$(COLOR_RESET)"
	@cargo check $(CARGO_FLAGS)

# =============================================================================
# Run Commands
# =============================================================================

.PHONY: run
run: build ## Run the kernel in QEMU (console mode)
	@echo "$(COLOR_BLUE)Starting $(PROJECT_NAME) in QEMU...$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-bios $(OPENSBI) \
		-kernel $(KERNEL_NAME) \
		-drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0


.PHONY: run-graphics
run-graphics: build ## Run the kernel in QEMU with graphics
	@echo "$(COLOR_BLUE)Starting $(PROJECT_NAME) with graphics...$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
        -display gtk \
        -serial mon:vc \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-bios $(OPENSBI) \
		-kernel $(KERNEL_NAME) \
		-drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0 

.PHONY: run-debug
run-debug: build ## Run the kernel with GDB debugging enabled
	@echo "$(COLOR_BLUE)Starting $(PROJECT_NAME) with GDB debugging...$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Connect with: gdb $(DEBUG_DIR)/$(KERNEL_NAME) -ex 'target remote :1234'$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-bios $(OPENSBI) \
		-kernel $(KERNEL_NAME) \
		-drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0 \
        -d guest_errors,int,unimp \
        -D qemu.log \
		-s -S

# =============================================================================
# Development Commands
# =============================================================================

.PHONY: test
test: ## Run unit tests
	@echo "$(COLOR_BLUE)Running unit tests...$(COLOR_RESET)"
	@cargo test $(CARGO_FLAGS)

.PHONY: test-release
test-release: ## Run unit tests (release mode)
	@echo "$(COLOR_BLUE)Running unit tests (release)...$(COLOR_RESET)"
	@cargo test $(CARGO_FLAGS) $(RELEASE_FLAGS)

.PHONY: integration
integration: build ## Run integration tests
	@echo "$(COLOR_BLUE)Running integration tests...$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Integration tests not yet implemented$(COLOR_RESET)"

.PHONY: bench
bench: ## Run benchmarks
	@echo "$(COLOR_BLUE)Running benchmarks...$(COLOR_RESET)"
	@cargo bench $(CARGO_FLAGS)

.PHONY: format
format: ## Format code with rustfmt
	@echo "$(COLOR_BLUE)Formatting code...$(COLOR_RESET)"
	@cargo fmt
	@echo "$(COLOR_GREEN)✓ Code formatted$(COLOR_RESET)"

.PHONY: format-check
format-check: ## Check code formatting
	@echo "$(COLOR_BLUE)Checking code formatting...$(COLOR_RESET)"
	@cargo fmt -- --check

.PHONY: clippy
clippy: ## Run Clippy linter
	@echo "$(COLOR_BLUE)Running Clippy linter...$(COLOR_RESET)"
	@cargo clippy $(CARGO_FLAGS) -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Run Clippy with automatic fixes
	@echo "$(COLOR_BLUE)Running Clippy with fixes...$(COLOR_RESET)"
	@cargo clippy $(CARGO_FLAGS) --fix --allow-dirty

.PHONY: doc
doc: ## Generate documentation
	@echo "$(COLOR_BLUE)Generating documentation...$(COLOR_RESET)"
	@cargo doc $(CARGO_FLAGS) --no-deps --document-private-items
	@echo "$(COLOR_GREEN)✓ Documentation generated: $(DOCS_DIR)/$(KERNEL_NAME)/index.html$(COLOR_RESET)"

.PHONY: doc-open
doc-open: doc ## Generate and open documentation
	@echo "$(COLOR_BLUE)Opening documentation...$(COLOR_RESET)"
	@cargo doc $(CARGO_FLAGS) --no-deps --document-private-items --open

.PHONY: check-all
check-all: format-check clippy test ## Run all quality checks

.PHONY: fix-all
fix-all: format clippy-fix ## Apply all automatic fixes

# =============================================================================
# Disk Image Commands
# =============================================================================

.PHONY: create-fat32
create-disk: ## Create a FAT32 test disk image
	@echo "$(COLOR_BLUE)Creating FAT32 disk image ($(DISK_SIZE))...$(COLOR_RESET)"
	@dd if=/dev/zero of=$(DISK_IMAGE) bs=1M count=$(shell echo $(DISK_SIZE) | sed 's/M//') 2>/dev/null
	@mkfs.fat -F32 $(DISK_IMAGE) >/dev/null 2>&1
	@echo "$(COLOR_GREEN)✓ FAT32 disk image created: $(DISK_IMAGE)$(COLOR_RESET)"

.PHONY: create-ext4
create-ext4: ## Create an ext4 test disk image
	@echo "$(COLOR_BLUE)Creating ext4 disk image ($(DISK_SIZE))...$(COLOR_RESET)"
	@dd if=/dev/zero of=$(DISK_IMAGE) bs=1M count=$(shell echo $(DISK_SIZE) | sed 's/M//') 2>/dev/null
	@mkfs.ext4 $(DISK_IMAGE) >/dev/null 2>&1
	@echo "$(COLOR_GREEN)✓ ext4 disk image created: $(DISK_IMAGE)$(COLOR_RESET)"

.PHONY: populate-disk
populate-disk: $(DISK_IMAGE) ## Add test files to disk image
	@echo "$(COLOR_BLUE)Populating disk image with test files...$(COLOR_RESET)"
	@mkdir -p $(DISK_MOUNT)
	@sudo mount -o loop $(DISK_IMAGE) $(DISK_MOUNT) 2>/dev/null || true
	@echo "Hello from elinOS, LittleMa, LittleBai" | sudo tee $(DISK_MOUNT)/hello.txt >/dev/null
	@echo "This is a test file for the elinOS filesystem." | sudo tee $(DISK_MOUNT)/test.txt >/dev/null
	@echo "README for elinOS test disk" | sudo tee $(DISK_MOUNT)/README.md >/dev/null
	@sudo umount $(DISK_MOUNT) 2>/dev/null || true
	@rmdir $(DISK_MOUNT) 2>/dev/null || true
	@echo "$(COLOR_GREEN)✓ Disk populated with test files$(COLOR_RESET)"

.PHONY: mount-disk
mount-disk: $(DISK_IMAGE) ## Mount disk image for inspection
	@echo "$(COLOR_BLUE)Mounting disk image...$(COLOR_RESET)"
	@mkdir -p $(DISK_MOUNT)
	@sudo mount -o loop $(DISK_IMAGE) $(DISK_MOUNT)
	@echo "$(COLOR_GREEN)✓ Disk mounted at $(DISK_MOUNT)$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Run 'make unmount-disk' when done$(COLOR_RESET)"

.PHONY: unmount-disk
unmount-disk: ## Unmount disk image
	@echo "$(COLOR_BLUE)Unmounting disk image...$(COLOR_RESET)"
	@sudo umount $(DISK_MOUNT) 2>/dev/null || true
	@rmdir $(DISK_MOUNT) 2>/dev/null || true
	@echo "$(COLOR_GREEN)✓ Disk unmounted$(COLOR_RESET)"

.PHONY: clean-disk
clean-disk: ## Remove disk image
	@echo "$(COLOR_YELLOW)Removing disk image...$(COLOR_RESET)"
	@rm -f $(DISK_IMAGE)
	@echo "$(COLOR_GREEN)✓ Disk image removed$(COLOR_RESET)"

# =============================================================================
# Environment and Setup Commands
# =============================================================================

.PHONY: env-check
env-check: ## Check development environment
	@echo "$(COLOR_BLUE)Checking development environment...$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)Rust Toolchain:$(COLOR_RESET)"
	@rustc --version 2>/dev/null || echo "  $(COLOR_RED)✗ Rust not found$(COLOR_RESET)"
	@cargo --version 2>/dev/null || echo "  $(COLOR_RED)✗ Cargo not found$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)RISC-V Target:$(COLOR_RESET)"
	@rustup target list --installed | grep $(TARGET) >/dev/null && echo "  $(COLOR_GREEN)✓ $(TARGET) installed$(COLOR_RESET)" || echo "  $(COLOR_RED)✗ $(TARGET) not installed$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)QEMU:$(COLOR_RESET)"
	@$(QEMU) --version 2>/dev/null | head -1 || echo "  $(COLOR_RED)✗ QEMU not found$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)OpenSBI Firmware:$(COLOR_RESET)"
	@test -f "$(OPENSBI)" && echo "  $(COLOR_GREEN)✓ $(OPENSBI)$(COLOR_RESET)" || echo "  $(COLOR_YELLOW)⚠ Using default firmware$(COLOR_RESET)"

.PHONY: setup
setup: ## Set up development environment
	@echo "$(COLOR_BLUE)Setting up development environment...$(COLOR_RESET)"
	@echo "$(COLOR_BOLD)Installing Rust nightly...$(COLOR_RESET)"
	@rustup toolchain install nightly
	@rustup default nightly
	@echo "$(COLOR_BOLD)Adding RISC-V target...$(COLOR_RESET)"
	@rustup target add $(TARGET)
	@echo "$(COLOR_BOLD)Installing development tools...$(COLOR_RESET)"
	@cargo install cargo-expand cargo-edit cargo-watch 2>/dev/null || true
	@echo "$(COLOR_GREEN)✓ Development environment setup complete$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Note: You may need to install QEMU manually for your system$(COLOR_RESET)"

# =============================================================================
# Release Commands
# =============================================================================

.PHONY: package
package: build-release ## Package release artifacts
	@echo "$(COLOR_BLUE)Packaging release artifacts...$(COLOR_RESET)"
	@mkdir -p release
	@cp $(RELEASE_DIR)/$(KERNEL_NAME) release/
	@cp README.md CONTRIBUTING.md release/
	@tar czf release/$(PROJECT_NAME)-$(VERSION)-$(TARGET).tar.gz -C release .
	@echo "$(COLOR_GREEN)✓ Release package created: release/$(PROJECT_NAME)-$(VERSION)-$(TARGET).tar.gz$(COLOR_RESET)"

.PHONY: clean-release
clean-release: ## Clean release artifacts
	@echo "$(COLOR_YELLOW)Cleaning release artifacts...$(COLOR_RESET)"
	@rm -rf release/
	@echo "$(COLOR_GREEN)✓ Release artifacts cleaned$(COLOR_RESET)"

# =============================================================================
# Utility Commands
# =============================================================================

.PHONY: size
size: build ## Show binary size information
	@echo "$(COLOR_BLUE)Binary size information:$(COLOR_RESET)"
	@size $(DEBUG_DIR)/$(KERNEL_NAME)

.PHONY: size-release
size-release: build-release ## Show release binary size information
	@echo "$(COLOR_BLUE)Release binary size information:$(COLOR_RESET)"
	@size $(RELEASE_DIR)/$(KERNEL_NAME)

.PHONY: objdump
objdump: build ## Disassemble the kernel binary
	@echo "$(COLOR_BLUE)Disassembling kernel...$(COLOR_RESET)"
	@riscv64-unknown-elf-objdump -d $(DEBUG_DIR)/$(KERNEL_NAME) | less

.PHONY: nm
nm: build ## Show kernel symbols
	@echo "$(COLOR_BLUE)Kernel symbols:$(COLOR_RESET)"
	@riscv64-unknown-elf-nm $(DEBUG_DIR)/$(KERNEL_NAME) | less

.PHONY: file-info
file-info: build ## Show file information
	@echo "$(COLOR_BLUE)File information:$(COLOR_RESET)"
	@file $(DEBUG_DIR)/$(KERNEL_NAME)
	@echo "$(COLOR_BLUE)ELF header:$(COLOR_RESET)"
	@readelf -h $(DEBUG_DIR)/$(KERNEL_NAME) 2>/dev/null || echo "readelf not available"

# =============================================================================
# Special Targets
# =============================================================================

# Ensure disk image exists for mounting operations
$(DISK_IMAGE):
	@$(MAKE) create-disk

# Prevent make from deleting intermediate files
.PRECIOUS: $(DISK_IMAGE)

# Ensure these targets run even if files with same names exist
.PHONY: all build clean help run test doc format clippy

# =============================================================================
# Make Configuration
# =============================================================================

# Use bash for shell commands
SHELL := /bin/bash

# Disable built-in rules and suffixes
MAKEFLAGS += --no-builtin-rules
.SUFFIXES: 