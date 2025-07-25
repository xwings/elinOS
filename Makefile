# elinOS Makefile
# build system for RISC-V experimental kernel

# =============================================================================
# Configuration Variables
# =============================================================================

# Project metadata
PROJECT_NAME := elinOS
BOOTLOADER_NAME := bootloader
BOOTLOADER_BIN := BOOT.bin
KERNEL_NAME := kernel
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

# Cross-compiler configuration
RISCV_PREFIX := riscv64-unknown-elf-
RISCV_CC := $(RISCV_PREFIX)gcc
RISCV_OBJDUMP := $(RISCV_PREFIX)objdump
RISCV_OBJCOPY := $(RISCV_PREFIX)objcopy

# C programs configuration
C_PROGRAMS_DIR := examples/c_programs
C_BUILD_DIR := target/c_programs
C_SOURCES := $(wildcard $(C_PROGRAMS_DIR)/*.c)
C_BINARIES := $(patsubst $(C_PROGRAMS_DIR)/%.c,$(C_BUILD_DIR)/%,$(C_SOURCES))

# RISC-V compiler flags (defined after C_PROGRAMS_DIR)
RISCV_CFLAGS := -march=rv64gc -mabi=lp64d -static -nostdlib -nostartfiles -ffreestanding -fno-stack-protector -T$(C_PROGRAMS_DIR)/program.ld -fPIC -fno-plt

# Logs
QEMU_LOG := qemu.log

# Disk image configuration
DISK_SIZE := 32M
DISK_IMAGE := disk.img
DISK_MOUNT := /tmp/elinOS-mount

# SD card image configuration
SDCARD_SIZE := 64M
SDCARD_IMAGE := sdcard.img
SDCARD_MOUNT := /tmp/elinOS-sdcard-mount

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
	@echo -e "$(COLOR_BOLD)$(COLOR_BLUE)$(PROJECT_NAME) v$(VERSION) - RISC-V Experimental Kernel$(COLOR_RESET)"
	@echo -e "$(COLOR_CYAN)==================================$(COLOR_RESET)"
	@echo ""
	@echo -e "$(COLOR_BOLD)Build Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(build|clean)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(COLOR_BOLD)Run Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(run|qemu)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(COLOR_BOLD)Development Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(test|format|clippy|doc)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(COLOR_BOLD)Disk Image Commands:$(COLOR_RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | grep -E "(disk|mount)" | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""
	@echo -e "$(COLOR_BOLD)Environment Information:$(COLOR_RESET)"
	@echo -e "  Target: $(COLOR_CYAN)$(TARGET)$(COLOR_RESET)"
	@echo -e "  OpenSBI: $(COLOR_CYAN)$(OPENSBI)$(COLOR_RESET)"
	@echo -e "  QEMU: $(COLOR_CYAN)$(shell $(QEMU) --version 2>/dev/null | head -1 || echo 'Not found')$(COLOR_RESET)"

# =============================================================================
# Build Commands
# =============================================================================

.PHONY: build
build: build-bootloader build-kernel ## Build the complete system (debug mode)

.PHONY: build-bootloader  
build-bootloader: ## Build the bootloader (debug mode)
	@echo -e "$(COLOR_BLUE)Building $(BOOTLOADER_NAME) (debug)...$(COLOR_RESET)"
	@cd bootloader && cargo build $(CARGO_FLAGS) $(DEBUG_FLAGS)
	@cp $(DEBUG_DIR)/$(BOOTLOADER_NAME) $(DEBUG_DIR)/$(BOOTLOADER_BIN)
	@echo -e "$(COLOR_GREEN)✓ Bootloader debug build completed: $(DEBUG_DIR)/$(BOOTLOADER_BIN)$(COLOR_RESET)"

.PHONY: build-kernel
build-kernel: ## Build the kernel (debug mode)
	@echo -e "$(COLOR_BLUE)Building $(KERNEL_NAME) (debug)...$(COLOR_RESET)"
	@rm -f $(QEMU_LOG)
	@cd kernel && cargo build $(CARGO_FLAGS) $(DEBUG_FLAGS)
	@echo -e "$(COLOR_GREEN)✓ Kernel debug build completed: $(DEBUG_DIR)/$(KERNEL_NAME)$(COLOR_RESET)"


.PHONY: build-release
build-release: build-bootloader-release build-kernel-release ## Build both bootloader and kernel (release mode)

.PHONY: build-bootloader-release
build-bootloader-release: ## Build the bootloader (release mode)
	@echo -e "$(COLOR_BLUE)Building $(BOOTLOADER_NAME) (release)...$(COLOR_RESET)"
	@cd bootloader && cargo build $(CARGO_FLAGS) $(RELEASE_FLAGS)
	@cp $(DEBUG_DIR)/$(BOOTLOADER_NAME) $(DEBUG_DIR)/$(BOOTLOADER_BIN)
	@echo -e "$(COLOR_GREEN)✓ Bootloader debug build completed: $(DEBUG_DIR)/$(BOOTLOADER_BIN)$(COLOR_RESET)"


.PHONY: build-kernel-release
build-kernel-release: ## Build the kernel (release mode)
	@echo -e "$(COLOR_BLUE)Building $(KERNEL_NAME) (release)...$(COLOR_RESET)"
	@rm -f $(QEMU_LOG)
	@cd kernel && cargo build $(CARGO_FLAGS) $(RELEASE_FLAGS)
	@echo -e "$(COLOR_GREEN)✓ Kernel release build completed: $(RELEASE_DIR)/$(KERNEL_NAME)$(COLOR_RESET)"


.PHONY: rebuild
rebuild: clean build ## Clean and rebuild the kernel

.PHONY: rebuild-release
rebuild-release: clean build-release ## Clean and rebuild the kernel (release)

.PHONY: all
all: build ext2-disk c-programs populate-disk

.PHONY: clean
clean: ## Clean build artifacts
	@echo -e "$(COLOR_YELLOW)Cleaning build artifacts...$(COLOR_RESET)"
	@cargo clean
	@rm -f $(DISK_IMAGE)
	@rm -f $(SDCARD_IMAGE)
	@rm -f $(QEMU_LOG)
	@rm -rf $(C_BUILD_DIR)
	@rm -rf $(DEBUG_DIR)/$(BOOTLOADER_BIN)
	@echo -e "$(COLOR_GREEN)✓ Clean completed$(COLOR_RESET)"

.PHONY: check
check: ## Check code without building
	@echo -e "$(COLOR_BLUE)Checking bootloader...$(COLOR_RESET)"
	@cd bootloader && cargo check $(CARGO_FLAGS)
	@echo -e "$(COLOR_BLUE)Checking kernel...$(COLOR_RESET)"
	@cd kernel && cargo check $(CARGO_FLAGS)

# =============================================================================
# C Programs Compilation
# =============================================================================

.PHONY: c-programs
c-programs: $(C_BINARIES) ## Compile all C example programs

$(C_BUILD_DIR)/%: $(C_PROGRAMS_DIR)/%.c | $(C_BUILD_DIR)
	@echo -e "$(COLOR_BLUE)Compiling C program: $<$(COLOR_RESET)"
	@$(RISCV_CC) $(RISCV_CFLAGS) -o $@ $<
	@echo -e "$(COLOR_GREEN)✓ Compiled: $@$(COLOR_RESET)"

$(C_BUILD_DIR):
	@mkdir -p $(C_BUILD_DIR)

.PHONY: c-programs-info
c-programs-info: c-programs ## Show information about compiled C programs
	@echo -e "$(COLOR_BLUE)C Programs Information:$(COLOR_RESET)"
	@for binary in $(C_BINARIES); do \
		if [ -f "$$binary" ]; then \
			echo -e "$(COLOR_CYAN)$$binary:$(COLOR_RESET)"; \
			file "$$binary"; \
			size "$$binary"; \
			echo ""; \
		fi; \
	done

.PHONY: c-programs-clean
c-programs-clean: ## Clean compiled C programs
	@echo -e "$(COLOR_YELLOW)Cleaning C programs...$(COLOR_RESET)"
	@rm -rf $(C_BUILD_DIR)
	@echo -e "$(COLOR_GREEN)✓ C programs cleaned$(COLOR_RESET)"

# =============================================================================
# Run Commands
# =============================================================================

.PHONY: run-console
run-console: build ## Run the kernel in QEMU (console mode)
	@echo -e "$(COLOR_BLUE)Starting $(PROJECT_NAME) in QEMU...$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-bios $(OPENSBI) \
		-kernel $(DEBUG_DIR)/$(KERNEL_NAME) \
		-drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0

.PHONY: run-console-debug
run-console-debug: build ## Run the elinOS with log output
	@echo -e "$(COLOR_BLUE)Starting $(PROJECT_NAME) with log output...$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-bios $(OPENSBI) \
		-kernel $(DEBUG_DIR)/$(KERNEL_NAME) \
		-drive file=${DISK_IMAGE},format=raw,if=none,id=disk0 \
        -device virtio-blk-device,drive=disk0 \
        -d guest_errors,int,unimp,exec,in_asm \
        -D qemu.log
		
.PHONY: run-fb
run-fb: build ## Run the kernel in QEMU with software framebuffer graphics
	@echo -e "$(COLOR_BLUE)Starting $(PROJECT_NAME) with software framebuffer graphics...$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)A graphics window should open showing framebuffer output$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-bios $(OPENSBI) \
		-kernel $(DEBUG_DIR)/$(BOOTLOADER_BIN) \
		-initrd $(DEBUG_DIR)/$(KERNEL_NAME) \
		-device virtio-blk-device,drive=hd0 \
		-drive file=$(DISK_IMAGE),format=raw,id=hd0 \
		-device virtio-gpu-device \
		-display gtk,show-cursor=on \
		-serial stdio

.PHONY: run-fb-debug
run-fb-debug: build ## Run the kernel in QEMU with software framebuffer testing
	@echo -e "$(COLOR_BLUE)Starting $(PROJECT_NAME) with software framebuffer testing...$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Testing framebuffer functionality without VirtIO GPU$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-bios $(OPENSBI) \
		-kernel $(DEBUG_DIR)/$(BOOTLOADER_BIN) \
		-initrd $(DEBUG_DIR)/$(KERNEL_NAME) \
		-device virtio-blk-device,drive=hd0 \
		-drive file=$(DISK_IMAGE),format=raw,id=hd0 \
		-device virtio-gpu-device \
		-display gtk,show-cursor=on \
		-serial stdio \
		-d guest_errors,unimp,exec,in_asm \
		-D qemu.log

.PHONY: run-sdimg
run-sdimg: build build-img c-programs populate-sdcard ## Run bootloader from SPI flash with SD card
	@echo -e "$(COLOR_BLUE)Starting $(PROJECT_NAME) with SPI flash boot and SD card...$(COLOR_RESET)"
	@$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-smp $(QEMU_SMP) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-bios $(OPENSBI) \
		-kernel $(DEBUG_DIR)/$(BOOTLOADER_BIN) \
		-device virtio-blk-device,drive=sdcard \
		-drive file=$(SDCARD_IMAGE),format=raw,if=none,id=sdcard

# =============================================================================
# Development Commands
# =============================================================================

.PHONY: test-console
test-console: clean all ## Run automated kernel tests using Python test runner
	@echo -e "$(COLOR_BLUE)Running automated kernel tests...$(COLOR_RESET)"
	@python3 test_runner.py --timeout 60 || (echo -e "$(COLOR_RED)✗ Tests failed$(COLOR_RESET)" && exit 1)
	@echo -e "$(COLOR_GREEN)✓ All tests passed$(COLOR_RESET)"

.PHONY: test-fb
test-fb:: clean all ## Run automated kernel tests with VGA graphics using Python test runner
	@echo -e "$(COLOR_BLUE)Running automated kernel with VGA graphics tests...$(COLOR_RESET)"
	@python3 test_runner.py --runtype=fb --timeout 60 || (echo -e "$(COLOR_RED)✗ Tests failed$(COLOR_RESET)" && exit 1)
	@echo -e "$(COLOR_GREEN)✓ All tests passed$(COLOR_RESET)"

.PHONY: test-sdimg
test-sdimg: clean build build-img c-programs populate-sdcard ## Run automated kernel tests with SD card using Python test runner
	@echo -e "$(COLOR_BLUE)Running automated kernel with SD card tests...$(COLOR_RESET)"
	@python3 test_runner.py --runtype=sdimg --timeout 60 || (echo -e "$(COLOR_RED)✗ Tests failed$(COLOR_RESET)" && exit 1)
	@echo -e "$(COLOR_GREEN)✓ All SD card tests passed$(COLOR_RESET)"

.PHONY: unittest
unittest: ## Run unit tests
	@echo -e "$(COLOR_BLUE)Running unit tests...$(COLOR_RESET)"
	@cd kernel && cargo test $(CARGO_FLAGS)

.PHONY: test-release
test-release: ## Run unit tests (release mode)
	@echo -e "$(COLOR_BLUE)Running unit tests (release)...$(COLOR_RESET)"
	@cd kernel && cargo test $(CARGO_FLAGS) $(RELEASE_FLAGS)

.PHONY: integration
integration: build ## Run integration tests
	@echo -e "$(COLOR_BLUE)Running integration tests...$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Integration tests not yet implemented$(COLOR_RESET)"

.PHONY: bench
bench: ## Run benchmarks
	@echo -e "$(COLOR_BLUE)Running benchmarks...$(COLOR_RESET)"
	@cd kernel && cargo bench $(CARGO_FLAGS)

.PHONY: format
format: ## Format code with rustfmt
	@echo -e "$(COLOR_BLUE)Formatting code...$(COLOR_RESET)"
	@cd kernel && cargo fmt
	@echo -e "$(COLOR_GREEN)✓ Code formatted$(COLOR_RESET)"

.PHONY: format-check
format-check: ## Check code formatting
	@echo -e "$(COLOR_BLUE)Checking code formatting...$(COLOR_RESET)"
	@cd kernel && cargo fmt -- --check

.PHONY: clippy
clippy: ## Run Clippy linter
	@echo -e "$(COLOR_BLUE)Running Clippy linter...$(COLOR_RESET)"
	@cd kernel && cargo clippy $(CARGO_FLAGS) -- -D warnings

.PHONY: clippy-fix
clippy-fix: ## Run Clippy with automatic fixes
	@echo -e "$(COLOR_BLUE)Running Clippy with fixes...$(COLOR_RESET)"
	@cd kernel && cargo clippy $(CARGO_FLAGS) --fix --allow-dirty

.PHONY: doc
doc: ## Generate documentation
	@echo -e "$(COLOR_BLUE)Generating documentation...$(COLOR_RESET)"
	@cd kernel && cargo doc $(CARGO_FLAGS) --no-deps --document-private-items
	@echo -e "$(COLOR_GREEN)✓ Documentation generated: $(DOCS_DIR)/$(KERNEL_NAME)/index.html$(COLOR_RESET)"

.PHONY: doc-open
doc-open: doc ## Generate and open documentation
	@echo -e "$(COLOR_BLUE)Opening documentation...$(COLOR_RESET)"
	@cd kernel && cargo doc $(CARGO_FLAGS) --no-deps --document-private-items --open

.PHONY: check-all
check-all: format-check clippy test ## Run all quality checks

.PHONY: fix-all
fix-all: format clippy-fix ## Apply all automatic fixes

# =============================================================================
# Disk Image Commands
# =============================================================================

.PHONY: ext2-disk
ext2-disk: ## Create an ext2 test disk image
	@echo -e "$(COLOR_BLUE)Creating ext2 disk image ($(DISK_SIZE))...$(COLOR_RESET)"
	@dd if=/dev/zero of=$(DISK_IMAGE) bs=1M count=$(shell echo $(DISK_SIZE) | sed 's/M//') 2>/dev/null
	@mkfs.ext2 $(DISK_IMAGE) >/dev/null 2>&1
	@echo -e "$(COLOR_GREEN)✓ ext2 disk image created: $(DISK_IMAGE)$(COLOR_RESET)"

.PHONY: populate-disk
populate-disk: $(DISK_IMAGE) ## Add test files to disk image
	@echo -e "$(COLOR_BLUE)Populating disk image with test files...$(COLOR_RESET)"
	@mkdir -p $(DISK_MOUNT)
	@sudo mount -o loop $(DISK_IMAGE) $(DISK_MOUNT) 2>/dev/null || true
	@echo "Hello from elinOS, LittleMa, LittleBai" | sudo tee $(DISK_MOUNT)/hello.txt >/dev/null
	@echo "This is a test file for the elinOS filesystem." | sudo tee $(DISK_MOUNT)/test.txt >/dev/null
	@echo "README for elinOS test disk" | sudo tee $(DISK_MOUNT)/README.md >/dev/null
	@echo "C Programs compiled for elinOS" | sudo tee $(DISK_MOUNT)/C_PROGRAMS.txt >/dev/null
	@if [ -f "$(DEBUG_DIR)/$(KERNEL_NAME)" ]; then \
		echo -e "$(COLOR_CYAN)  Copying kernel to disk...$(COLOR_RESET)"; \
		sudo cp "$(DEBUG_DIR)/$(KERNEL_NAME)" "$(DISK_MOUNT)/kernel"; \
	fi
	@for binary in $(C_BINARIES); do \
		if [ -f "$$binary" ]; then \
			echo -e "$(COLOR_CYAN)  Copying: $$(basename $$binary)$(COLOR_RESET)"; \
			sudo cp "$$binary" "$(DISK_MOUNT)"; \
		fi; \
	done	
	@sudo umount $(DISK_MOUNT) 2>/dev/null || true
	@rmdir $(DISK_MOUNT) 2>/dev/null || true
	@echo -e "$(COLOR_GREEN)✓ Disk populated with test files$(COLOR_RESET)"

.PHONY: mount-disk
mount-disk: $(DISK_IMAGE) ## Mount disk image for inspection
	@echo -e "$(COLOR_BLUE)Mounting disk image...$(COLOR_RESET)"
	@mkdir -p $(DISK_MOUNT)
	@sudo mount -o loop $(DISK_IMAGE) $(DISK_MOUNT)
	@echo -e "$(COLOR_GREEN)✓ Disk mounted at $(DISK_MOUNT)$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Run 'make unmount-disk' when done$(COLOR_RESET)"

.PHONY: unmount-disk
unmount-disk: ## Unmount disk image
	@echo -e "$(COLOR_BLUE)Unmounting disk image...$(COLOR_RESET)"
	@sudo umount $(DISK_MOUNT) 2>/dev/null || true
	@rmdir $(DISK_MOUNT) 2>/dev/null || true
	@echo -e "$(COLOR_GREEN)✓ Disk unmounted$(COLOR_RESET)"

.PHONY: clean-disk
clean-disk: ## Remove disk image
	@echo -e "$(COLOR_YELLOW)Removing disk image...$(COLOR_RESET)"
	@rm -f $(DISK_IMAGE)
	@echo -e "$(COLOR_GREEN)✓ Disk image removed$(COLOR_RESET)"

# =============================================================================
# SD Card Image Commands
# =============================================================================

.PHONY: build-img
build-img: ## Create SD card image with ext2 filesystem
	@echo -e "$(COLOR_BLUE)Creating SD card image ($(SDCARD_SIZE))...$(COLOR_RESET)"
	@dd if=/dev/zero of=$(SDCARD_IMAGE) bs=1M count=$(shell echo $(SDCARD_SIZE) | sed 's/M//') 2>/dev/null
	@mkfs.ext2 $(SDCARD_IMAGE) >/dev/null 2>&1
	@echo -e "$(COLOR_GREEN)✓ SD card image created: $(SDCARD_IMAGE)$(COLOR_RESET)"

.PHONY: populate-sdcard
populate-sdcard: $(SDCARD_IMAGE) ## Add test files and kernel to SD card image
	@echo -e "$(COLOR_BLUE)Populating SD card image with test files and kernel...$(COLOR_RESET)"
	@mkdir -p $(SDCARD_MOUNT)
	@sudo mount -o loop $(SDCARD_IMAGE) $(SDCARD_MOUNT) 2>/dev/null || true
	@echo "Hello from elinOS SD card, LittleMa, LittleBai" | sudo tee $(SDCARD_MOUNT)/hello.txt >/dev/null
	@echo "This is a test file for the elinOS SD card filesystem." | sudo tee $(SDCARD_MOUNT)/test.txt >/dev/null
	@echo "README for elinOS SD card test disk" | sudo tee $(SDCARD_MOUNT)/README.md >/dev/null
	@echo "C Programs compiled for elinOS on SD card" | sudo tee $(SDCARD_MOUNT)/C_PROGRAMS.txt >/dev/null
	@if [ -f "$(DEBUG_DIR)/$(KERNEL_NAME)" ]; then \
		echo -e "$(COLOR_CYAN)  Copying kernel to SD card...$(COLOR_RESET)"; \
		sudo cp "$(DEBUG_DIR)/$(KERNEL_NAME)" "$(SDCARD_MOUNT)/kernel"; \
	fi
	@for binary in $(C_BINARIES); do \
		if [ -f "$$binary" ]; then \
			echo -e "$(COLOR_CYAN)  Copying: $$(basename $$binary)$(COLOR_RESET)"; \
			sudo cp "$$binary" "$(SDCARD_MOUNT)"; \
		fi; \
	done	
	@sudo umount $(SDCARD_MOUNT) 2>/dev/null || true
	@rmdir $(SDCARD_MOUNT) 2>/dev/null || true
	@echo -e "$(COLOR_GREEN)✓ SD card populated with test files and kernel$(COLOR_RESET)"

.PHONY: mount-sdcard
mount-sdcard: $(SDCARD_IMAGE) ## Mount SD card image for inspection
	@echo -e "$(COLOR_BLUE)Mounting SD card image...$(COLOR_RESET)"
	@mkdir -p $(SDCARD_MOUNT)
	@sudo mount -o loop $(SDCARD_IMAGE) $(SDCARD_MOUNT)
	@echo -e "$(COLOR_GREEN)✓ SD card mounted at $(SDCARD_MOUNT)$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Run 'make unmount-sdcard' when done$(COLOR_RESET)"

.PHONY: unmount-sdcard
unmount-sdcard: ## Unmount SD card image
	@echo -e "$(COLOR_BLUE)Unmounting SD card image...$(COLOR_RESET)"
	@sudo umount $(SDCARD_MOUNT) 2>/dev/null || true
	@rmdir $(SDCARD_MOUNT) 2>/dev/null || true
	@echo -e "$(COLOR_GREEN)✓ SD card unmounted$(COLOR_RESET)"

.PHONY: clean-sdcard
clean-sdcard: ## Remove SD card image
	@echo -e "$(COLOR_YELLOW)Removing SD card image...$(COLOR_RESET)"
	@rm -f $(SDCARD_IMAGE)
	@echo -e "$(COLOR_GREEN)✓ SD card image removed$(COLOR_RESET)"

# =============================================================================
# Environment and Setup Commands
# =============================================================================

.PHONY: env-check
env-check: ## Check development environment
	@echo -e "$(COLOR_BLUE)Checking development environment...$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)Rust Toolchain:$(COLOR_RESET)"
	@rustc --version 2>/dev/null || echo -e "  $(COLOR_RED)✗ Rust not found$(COLOR_RESET)"
	@cargo --version 2>/dev/null || echo -e "  $(COLOR_RED)✗ Cargo not found$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)RISC-V Target:$(COLOR_RESET)"
	@rustup target list --installed | grep $(TARGET) >/dev/null && echo -e "  $(COLOR_GREEN)✓ $(TARGET) installed$(COLOR_RESET)" || echo -e "  $(COLOR_RED)✗ $(TARGET) not installed$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)QEMU:$(COLOR_RESET)"
	@$(QEMU) --version 2>/dev/null | head -1 || echo -e "  $(COLOR_RED)✗ QEMU not found$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)RISC-V Cross-Compiler:$(COLOR_RESET)"
	@$(RISCV_CC) --version 2>/dev/null | head -1 || echo -e "  $(COLOR_RED)✗ $(RISCV_CC) not found$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)OpenSBI Firmware:$(COLOR_RESET)"
	@test -f "$(OPENSBI)" && echo -e "  $(COLOR_GREEN)✓ $(OPENSBI)$(COLOR_RESET)" || echo -e "  $(COLOR_YELLOW)⚠ Using default firmware$(COLOR_RESET)"

.PHONY: setup
setup: ## Set up development environment
	@echo -e "$(COLOR_BLUE)Setting up development environment...$(COLOR_RESET)"
	@echo -e "$(COLOR_BOLD)Installing Rust nightly...$(COLOR_RESET)"
	@rustup toolchain install nightly
	@rustup default nightly
	@echo -e "$(COLOR_BOLD)Adding RISC-V target...$(COLOR_RESET)"
	@rustup target add $(TARGET)
	@echo -e "$(COLOR_BOLD)Installing development tools...$(COLOR_RESET)"
	@cargo install cargo-expand cargo-edit cargo-watch 2>/dev/null || true
	@echo -e "$(COLOR_GREEN)✓ Development environment setup complete$(COLOR_RESET)"
	@echo -e "$(COLOR_YELLOW)Note: You may need to install QEMU manually for your system$(COLOR_RESET)"

# =============================================================================
# Release Commands
# =============================================================================

.PHONY: package
package: build-release ## Package release artifacts
	@echo -e "$(COLOR_BLUE)Packaging release artifacts...$(COLOR_RESET)"
	@mkdir -p release
	@cp $(RELEASE_DIR)/$(KERNEL_NAME) release/
	@cp README.md CONTRIBUTING.md release/
	@tar czf release/$(PROJECT_NAME)-$(VERSION)-$(TARGET).tar.gz -C release .
	@echo -e "$(COLOR_GREEN)✓ Release package created: release/$(PROJECT_NAME)-$(VERSION)-$(TARGET).tar.gz$(COLOR_RESET)"

.PHONY: clean-release
clean-release: ## Clean release artifacts
	@echo -e "$(COLOR_YELLOW)Cleaning release artifacts...$(COLOR_RESET)"
	@rm -rf release/
	@echo -e "$(COLOR_GREEN)✓ Release artifacts cleaned$(COLOR_RESET)"

# =============================================================================
# Utility Commands
# =============================================================================

.PHONY: size
size: build ## Show binary size information
	@echo -e "$(COLOR_BLUE)Bootloader size information:$(COLOR_RESET)"
	@size $(DEBUG_DIR)/$(BOOTLOADER_NAME)
	@echo -e "$(COLOR_BLUE)Kernel size information:$(COLOR_RESET)"
	@size $(DEBUG_DIR)/$(KERNEL_NAME)

.PHONY: size-release
size-release: build-release ## Show release binary size information
	@echo -e "$(COLOR_BLUE)Bootloader release size information:$(COLOR_RESET)"
	@size $(RELEASE_DIR)/$(BOOTLOADER_NAME)
	@echo -e "$(COLOR_BLUE)Kernel release size information:$(COLOR_RESET)"
	@size $(RELEASE_DIR)/$(KERNEL_NAME)

.PHONY: objdump
objdump: build ## Disassemble the kernel binary
	@echo -e "$(COLOR_BLUE)Disassembling kernel...$(COLOR_RESET)"
	@riscv64-unknown-elf-objdump -d $(DEBUG_DIR)/$(KERNEL_NAME) | less

.PHONY: nm
nm: build ## Show kernel symbols
	@echo -e "$(COLOR_BLUE)Kernel symbols:$(COLOR_RESET)"
	@riscv64-unknown-elf-nm $(DEBUG_DIR)/$(KERNEL_NAME) | less

.PHONY: file-info
file-info: build ## Show file information
	@echo -e "$(COLOR_BLUE)File information:$(COLOR_RESET)"
	@file $(DEBUG_DIR)/$(KERNEL_NAME)
	@echo -e "$(COLOR_BLUE)ELF header:$(COLOR_RESET)"
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